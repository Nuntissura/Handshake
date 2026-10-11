//! Pre-WP-KERNEL-006 account-scope seam for HTTP read boundaries (HBR-PRIV-002).
//!
//! # Why this exists, and what it is not
//!
//! Handshake has no authentication layer yet — `WP-KERNEL-006` owns
//! `LocalAccount` / `Principal` / `AuthenticatedSession` and the PostgreSQL RLS
//! that will enforce them. This module does **not** invent one and performs no
//! credential check or session issuance. Its authority is instead the exact
//! five-field product-local resource scope persisted by Tauri and installed in
//! backend server state at boot; callers cannot select that identity.
//!
//! What it does do is close the concrete defect that exists today. Before this,
//! every ModelLane navigation, diagnostics, and model-registry route returned
//! whatever row matched the id in the path — to anybody who could reach the
//! port. The scope was not merely *unauthenticated*, it was *absent*: there was
//! no place in the request where an owning account could even be expressed, so
//! the store had nothing to filter on and the query enumerated the table.
//!
//! The server-owned exact scope is required and its absence or corruption fails
//! closed. The two HTTP headers are non-authoritative equality assertions:
//!
//! * absent headers use the exact server scope;
//! * if supplied, `X-Handshake-Owner-Account` and `X-Handshake-Workspace` must
//!   match that authority exactly;
//! * blank, malformed, or mismatched assertions are denied with stable reason
//!   codes and can never widen or replace the server authority.
//!
//! A derived `ResourceScopeQuery` exists only as a SQL prefilter. Account-facing
//! projections and detail endpoints must still compare all five exact fields.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::swarm_orchestration::resource_scope::{
    ExactResourceScopeAttribution, OwnerAccountId, ResourceScope, ResourceScopeQuery,
    WorkspaceScopeRef,
};

/// Strict Tauri -> backend handoff for the persisted product-local identity.
/// The value is server-owned authority; HTTP headers may only assert equality.
pub const PRODUCT_LOCAL_RESOURCE_SCOPE_ENV: &str = "HANDSHAKE_PRODUCT_LOCAL_RESOURCE_SCOPE_JSON";

/// Optional equality assertion for the server-owned account.
pub const OWNER_ACCOUNT_HEADER: &str = "x-handshake-owner-account";
/// Optional equality assertion for the server-owned workspace.
pub const WORKSPACE_HEADER: &str = "x-handshake-workspace";

pub const MISSING_SCOPE_CODE: &str = "RESOURCE_SCOPE_REQUIRED";
pub const MALFORMED_SCOPE_CODE: &str = "RESOURCE_SCOPE_MALFORMED";
pub const MISMATCHED_SCOPE_CODE: &str = "RESOURCE_SCOPE_MISMATCH";
pub const SERVER_SCOPE_UNAVAILABLE_CODE: &str = "RESOURCE_SCOPE_AUTHORITY_UNAVAILABLE";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductLocalResourceScope(ExactResourceScopeAttribution);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductLocalResourceScopeEnvelope {
    schema_version: u32,
    scope: StrictExactResourceScopeAttribution,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictExactResourceScopeAttribution {
    owner_account_id: OwnerAccountId,
    actor_principal_id: crate::swarm_orchestration::resource_scope::ActorPrincipalId,
    authenticated_session_id: crate::swarm_orchestration::resource_scope::AuthenticatedSessionRef,
    access_space_id: crate::swarm_orchestration::resource_scope::AccessSpaceRef,
    workspace_id: WorkspaceScopeRef,
}

#[derive(Debug, thiserror::Error)]
pub enum ProductLocalResourceScopeError {
    #[error("{PRODUCT_LOCAL_RESOURCE_SCOPE_ENV} is missing or not Unicode")]
    Missing,
    #[error("{PRODUCT_LOCAL_RESOURCE_SCOPE_ENV} is not strict schema-version-1 JSON: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("{PRODUCT_LOCAL_RESOURCE_SCOPE_ENV} has unsupported schema_version {0}")]
    UnsupportedVersion(u32),
    #[error("{PRODUCT_LOCAL_RESOURCE_SCOPE_ENV} has a nil {0}")]
    NilIdentifier(&'static str),
    #[error("{PRODUCT_LOCAL_RESOURCE_SCOPE_ENV} has a blank workspace_id")]
    BlankWorkspace,
}

impl ProductLocalResourceScope {
    pub fn from_env() -> Result<Self, ProductLocalResourceScopeError> {
        let value = std::env::var(PRODUCT_LOCAL_RESOURCE_SCOPE_ENV)
            .map_err(|_| ProductLocalResourceScopeError::Missing)?;
        Self::from_json(&value)
    }

    pub fn from_json(value: &str) -> Result<Self, ProductLocalResourceScopeError> {
        let envelope: ProductLocalResourceScopeEnvelope = serde_json::from_str(value)?;
        if envelope.schema_version != 1 {
            return Err(ProductLocalResourceScopeError::UnsupportedVersion(
                envelope.schema_version,
            ));
        }
        let scope = envelope.scope;
        for (name, id) in [
            ("owner_account_id", scope.owner_account_id.as_uuid()),
            ("actor_principal_id", scope.actor_principal_id.as_uuid()),
            (
                "authenticated_session_id",
                scope.authenticated_session_id.as_uuid(),
            ),
            ("access_space_id", scope.access_space_id.as_uuid()),
        ] {
            if id.is_nil() {
                return Err(ProductLocalResourceScopeError::NilIdentifier(name));
            }
        }
        if scope.workspace_id.as_str().trim().is_empty() {
            return Err(ProductLocalResourceScopeError::BlankWorkspace);
        }
        Ok(Self(ExactResourceScopeAttribution {
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        }))
    }

    pub fn from_exact(
        scope: ExactResourceScopeAttribution,
    ) -> Result<Self, ProductLocalResourceScopeError> {
        let encoded = serde_json::json!({ "schema_version": 1, "scope": scope });
        Self::from_json(&encoded.to_string())
    }

    pub fn exact(&self) -> &ExactResourceScopeAttribution {
        &self.0
    }

    pub fn resource_scope(&self) -> ResourceScope {
        RequestAccountScope::from_exact(self.0.clone()).resource_scope()
    }
}

/// The account scope a request is authorized for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestAccountScope {
    exact: ExactResourceScopeAttribution,
    query: ResourceScopeQuery,
}

impl RequestAccountScope {
    pub fn from_exact(exact: ExactResourceScopeAttribution) -> Self {
        let query = ResourceScopeQuery::for_owner(exact.owner_account_id)
            .within_workspace(exact.workspace_id.clone());
        Self { exact, query }
    }

    pub fn query(&self) -> &ResourceScopeQuery {
        &self.query
    }

    pub fn exact(&self) -> &ExactResourceScopeAttribution {
        &self.exact
    }

    pub fn into_query(self) -> ResourceScopeQuery {
        self.query
    }

    pub fn into_exact(self) -> ExactResourceScopeAttribution {
        self.exact
    }

    pub fn resource_scope(&self) -> ResourceScope {
        ResourceScope::new(self.exact.owner_account_id, self.exact.actor_principal_id)
            .with_session(self.exact.authenticated_session_id)
            .with_access_space(self.exact.access_space_id)
            .with_workspace(self.exact.workspace_id.clone())
    }
}

/// Why a request was refused before it reached storage.
///
/// The body carries a stable machine-readable code and a fixed operator-facing
/// hint. It deliberately never echoes the requested resource id, the stored
/// owner, or anything about whether the resource exists — a denial must not
/// become an existence oracle (HBR-PRIV-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountScopeRejection {
    status: StatusCode,
    code: &'static str,
    detail: &'static str,
}

impl AccountScopeRejection {
    pub const fn code(&self) -> &'static str {
        self.code
    }

    const fn missing() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: MISSING_SCOPE_CODE,
            detail: "a supplied scope assertion must be nonblank and well formed",
        }
    }

    const fn malformed() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: MALFORMED_SCOPE_CODE,
            detail: "the supplied owning-account scope is not a well-formed identifier",
        }
    }

    const fn mismatched() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: MISMATCHED_SCOPE_CODE,
            detail: "the supplied scope assertion does not match the server-owned product scope",
        }
    }

    const fn server_scope_unavailable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: SERVER_SCOPE_UNAVAILABLE_CODE,
            detail: "the server-owned product scope authority is unavailable",
        }
    }
}

#[derive(Debug, Serialize)]
struct AccountScopeRejectionBody {
    error: &'static str,
    detail: &'static str,
}

impl IntoResponse for AccountScopeRejection {
    fn into_response(self) -> Response {
        (
            // FORBIDDEN, not UNAUTHORIZED: there is no authentication challenge
            // to issue yet, and 401 would imply one exists.
            self.status,
            Json(AccountScopeRejectionBody {
                error: self.code,
                detail: self.detail,
            }),
        )
            .into_response()
    }
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for RequestAccountScope
where
    S: Send + Sync,
{
    type Rejection = AccountScopeRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let server_scope = parts
            .extensions
            .get::<ProductLocalResourceScope>()
            .cloned()
            .ok_or_else(AccountScopeRejection::server_scope_unavailable)?;
        if let Some(raw_owner) = parts.headers.get(OWNER_ACCOUNT_HEADER) {
            let raw_owner = raw_owner
                .to_str()
                .map_err(|_| AccountScopeRejection::malformed())?
                .trim();
            if raw_owner.is_empty() {
                return Err(AccountScopeRejection::missing());
            }
            let owner =
                Uuid::parse_str(raw_owner).map_err(|_| AccountScopeRejection::malformed())?;
            if OwnerAccountId::from_uuid(owner) != server_scope.exact().owner_account_id {
                return Err(AccountScopeRejection::mismatched());
            }
        }

        if let Some(raw_workspace) = parts.headers.get(WORKSPACE_HEADER) {
            let raw_workspace = raw_workspace
                .to_str()
                .map_err(|_| AccountScopeRejection::malformed())?
                .trim();
            let workspace = WorkspaceScopeRef::new(raw_workspace)
                .map_err(|_| AccountScopeRejection::malformed())?;
            if workspace != server_scope.exact().workspace_id {
                return Err(AccountScopeRejection::mismatched());
            }
        }

        Ok(Self::from_exact(server_scope.exact().clone()))
    }
}

/// Shape of the denial body, for tests and for the UserManual entry.
pub fn rejection_body_shape() -> serde_json::Value {
    json!({ "error": MISSING_SCOPE_CODE, "detail": "<fixed hint, never resource metadata>" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef,
    };
    use axum::http::Request;

    fn exact_scope() -> ExactResourceScopeAttribution {
        ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::mint(),
            actor_principal_id: ActorPrincipalId::mint(),
            authenticated_session_id: AuthenticatedSessionRef::mint(),
            access_space_id: AccessSpaceRef::mint(),
            workspace_id: WorkspaceScopeRef::new("ws-alpha").expect("valid workspace"),
        }
    }

    async fn extract(
        server_scope: Option<ProductLocalResourceScope>,
        headers: &[(&str, &str)],
    ) -> Result<RequestAccountScope, AccountScopeRejection> {
        let mut builder = Request::builder().uri("/swarm/model-lanes/navigation/runs/run-1");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let request = builder.body(()).expect("build request");
        let (mut parts, ()) = request.into_parts();
        if let Some(server_scope) = server_scope {
            parts.extensions.insert(server_scope);
        }
        RequestAccountScope::from_request_parts(&mut parts, &()).await
    }

    #[test]
    fn product_local_scope_envelope_is_strict_and_complete() {
        let exact = exact_scope();
        let valid = serde_json::json!({ "schema_version": 1, "scope": exact });
        let parsed = ProductLocalResourceScope::from_json(&valid.to_string())
            .expect("strict exact scope envelope");
        assert_eq!(parsed.exact(), &exact);

        for invalid in [
            serde_json::json!({ "schema_version": 2, "scope": exact }),
            serde_json::json!({ "schema_version": 1, "scope": exact, "unknown": true }),
            serde_json::json!({
                "schema_version": 1,
                "scope": {
                    "owner_account_id": exact.owner_account_id,
                    "actor_principal_id": exact.actor_principal_id,
                    "authenticated_session_id": exact.authenticated_session_id,
                    "access_space_id": exact.access_space_id,
                    "workspace_id": exact.workspace_id,
                    "unknown": true
                }
            }),
            serde_json::json!({
                "schema_version": 1,
                "scope": {
                    "owner_account_id": Uuid::nil(),
                    "actor_principal_id": exact.actor_principal_id,
                    "authenticated_session_id": exact.authenticated_session_id,
                    "access_space_id": exact.access_space_id,
                    "workspace_id": exact.workspace_id
                }
            }),
            serde_json::json!({
                "schema_version": 1,
                "scope": {
                    "owner_account_id": exact.owner_account_id,
                    "actor_principal_id": exact.actor_principal_id,
                    "authenticated_session_id": exact.authenticated_session_id,
                    "access_space_id": exact.access_space_id,
                    "workspace_id": "   "
                }
            }),
        ] {
            ProductLocalResourceScope::from_json(&invalid.to_string())
                .expect_err("invalid server scope must fail closed");
        }
    }

    #[tokio::test]
    async fn a_missing_server_scope_authority_is_service_unavailable() {
        let rejection = extract(None, &[])
            .await
            .expect_err("missing server-owned scope must fail closed");
        assert_eq!(rejection.code(), SERVER_SCOPE_UNAVAILABLE_CODE);
        assert_eq!(
            rejection.into_response().status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn absent_headers_use_server_authority_without_selecting_scope() {
        let exact = exact_scope();
        let server_scope = ProductLocalResourceScope::from_exact(exact.clone()).unwrap();
        let extracted = extract(Some(server_scope), &[])
            .await
            .expect("server-owned exact scope is sufficient authority");
        assert_eq!(extracted.exact(), &exact);
    }

    #[tokio::test]
    async fn a_blank_scope_header_is_treated_as_absent_not_as_a_wildcard() {
        let server_scope = ProductLocalResourceScope::from_exact(exact_scope()).unwrap();
        let rejection = extract(Some(server_scope), &[(OWNER_ACCOUNT_HEADER, "   ")])
            .await
            .expect_err("a blank owning-account header must be refused");
        assert_eq!(rejection.code(), MISSING_SCOPE_CODE);
    }

    #[tokio::test]
    async fn a_malformed_scope_header_is_denied() {
        let server_scope = ProductLocalResourceScope::from_exact(exact_scope()).unwrap();
        let rejection = extract(Some(server_scope), &[(OWNER_ACCOUNT_HEADER, "not-a-uuid")])
            .await
            .expect_err("a malformed owning-account header must be refused");
        assert_eq!(rejection.code(), MALFORMED_SCOPE_CODE);
        assert_eq!(rejection.into_response().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_blank_workspace_header_is_malformed_rather_than_a_silent_widening() {
        let exact = exact_scope();
        let owner = exact.owner_account_id.to_string();
        let server_scope = ProductLocalResourceScope::from_exact(exact).unwrap();
        let rejection = extract(
            Some(server_scope),
            &[
                (OWNER_ACCOUNT_HEADER, owner.as_str()),
                (WORKSPACE_HEADER, "  "),
            ],
        )
        .await
        .expect_err("a present-but-blank workspace narrowing must not be dropped");
        assert_eq!(rejection.code(), MALFORMED_SCOPE_CODE);
    }

    #[tokio::test]
    async fn matching_headers_yield_the_full_server_owned_exact_scope() {
        let exact = exact_scope();
        let owner = exact.owner_account_id.to_string();
        let server_scope = ProductLocalResourceScope::from_exact(exact.clone()).unwrap();
        let narrowed = extract(
            Some(server_scope),
            &[
                (OWNER_ACCOUNT_HEADER, owner.as_str()),
                (WORKSPACE_HEADER, "ws-alpha"),
            ],
        )
        .await
        .expect("matching assertions must expose server-owned exact scope");
        assert_eq!(narrowed.exact(), &exact);
        assert_eq!(
            narrowed.query().workspace().map(|ws| ws.as_str()),
            Some("ws-alpha")
        );
    }

    #[tokio::test]
    async fn mismatched_owner_and_workspace_assertions_are_denied() {
        let exact = exact_scope();
        let owner = exact.owner_account_id.to_string();
        for headers in [
            vec![
                (OWNER_ACCOUNT_HEADER, OwnerAccountId::mint().to_string()),
                (WORKSPACE_HEADER, "ws-alpha".to_owned()),
            ],
            vec![
                (OWNER_ACCOUNT_HEADER, owner.clone()),
                (WORKSPACE_HEADER, "ws-other".to_owned()),
            ],
        ] {
            let borrowed = headers
                .iter()
                .map(|(name, value)| (*name, value.as_str()))
                .collect::<Vec<_>>();
            let rejection = extract(
                Some(ProductLocalResourceScope::from_exact(exact.clone()).unwrap()),
                &borrowed,
            )
            .await
            .expect_err("mismatched assertion must be denied");
            assert_eq!(rejection.code(), MISMATCHED_SCOPE_CODE);
        }

        let extracted = extract(
            Some(ProductLocalResourceScope::from_exact(exact).unwrap()),
            &[(OWNER_ACCOUNT_HEADER, owner.as_str())],
        )
        .await
        .expect("an absent workspace header cannot widen the server-owned exact scope");
        assert_eq!(extracted.query().workspace().unwrap().as_str(), "ws-alpha");
    }

    #[test]
    fn a_denial_body_never_carries_resource_metadata() {
        // HBR-PRIV-004: the rejection is built from fixed literals only, so
        // there is no path by which a resource id or owner can reach it.
        for rejection in [
            AccountScopeRejection::missing(),
            AccountScopeRejection::malformed(),
            AccountScopeRejection::mismatched(),
            AccountScopeRejection::server_scope_unavailable(),
        ] {
            assert!(!rejection.detail.contains("run"));
            assert!(!rejection.detail.contains("owner_account_id="));
        }
    }
}

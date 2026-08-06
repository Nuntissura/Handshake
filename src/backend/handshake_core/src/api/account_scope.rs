//! Pre-WP-KERNEL-006 account-scope seam for HTTP read boundaries (HBR-PRIV-002).
//!
//! # Why this exists, and what it is not
//!
//! Handshake has no authentication layer yet — `WP-KERNEL-006` owns
//! `LocalAccount` / `Principal` / `AuthenticatedSession` and the PostgreSQL RLS
//! that will enforce them. This module does **not** invent one, and it must not
//! be mistaken for one: it performs no credential check, issues no session, and
//! trusts the caller's claimed account.
//!
//! What it does do is close the concrete defect that exists today. Before this,
//! every ModelLane navigation, diagnostics, and model-registry route returned
//! whatever row matched the id in the path — to anybody who could reach the
//! port. The scope was not merely *unauthenticated*, it was *absent*: there was
//! no place in the request where an owning account could even be expressed, so
//! the store had nothing to filter on and the query enumerated the table.
//!
//! So this introduces the **required input** and makes its absence a denial:
//!
//! * every scoped route must carry `X-Handshake-Owner-Account: <uuid>`;
//! * a missing, blank, or malformed value is `403` with a stable reason code —
//!   never a fallback to "return everything";
//! * `X-Handshake-Workspace: <id>` optionally narrows further, which is the
//!   same-project privacy case (HBR-PRIV-003).
//!
//! When KERNEL-006 lands, the change here is to derive the same
//! `ResourceScopeQuery` from the authenticated session instead of from the
//! header, and to reject the header outright. The stores below it do not change,
//! because they already require a scope rather than defaulting to one.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::swarm_orchestration::resource_scope::{
    OwnerAccountId, ResourceScopeQuery, WorkspaceScopeRef,
};

/// Required. Carries the owning account the caller is reading as.
pub const OWNER_ACCOUNT_HEADER: &str = "x-handshake-owner-account";
/// Optional. Narrows the read to one workspace within that account.
pub const WORKSPACE_HEADER: &str = "x-handshake-workspace";

pub const MISSING_SCOPE_CODE: &str = "RESOURCE_SCOPE_REQUIRED";
pub const MALFORMED_SCOPE_CODE: &str = "RESOURCE_SCOPE_MALFORMED";

/// The account scope a request is authorized for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestAccountScope(ResourceScopeQuery);

impl RequestAccountScope {
    pub fn query(&self) -> &ResourceScopeQuery {
        &self.0
    }

    pub fn into_query(self) -> ResourceScopeQuery {
        self.0
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
    code: &'static str,
    detail: &'static str,
}

impl AccountScopeRejection {
    pub const fn code(&self) -> &'static str {
        self.code
    }

    const fn missing() -> Self {
        Self {
            code: MISSING_SCOPE_CODE,
            detail: "this route requires an owning-account scope; send the X-Handshake-Owner-Account header",
        }
    }

    const fn malformed() -> Self {
        Self {
            code: MALFORMED_SCOPE_CODE,
            detail: "the supplied owning-account scope is not a well-formed identifier",
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
            StatusCode::FORBIDDEN,
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
        let raw_owner = parts
            .headers
            .get(OWNER_ACCOUNT_HEADER)
            .ok_or_else(AccountScopeRejection::missing)?
            .to_str()
            .map_err(|_| AccountScopeRejection::malformed())?
            .trim()
            .to_owned();
        if raw_owner.is_empty() {
            // A blank header is an absent scope, not a wildcard.
            return Err(AccountScopeRejection::missing());
        }
        let owner = Uuid::parse_str(&raw_owner).map_err(|_| AccountScopeRejection::malformed())?;
        let mut query = ResourceScopeQuery::for_owner(OwnerAccountId::from_uuid(owner));

        if let Some(raw_workspace) = parts.headers.get(WORKSPACE_HEADER) {
            let raw_workspace = raw_workspace
                .to_str()
                .map_err(|_| AccountScopeRejection::malformed())?
                .trim();
            // Present-but-blank is malformed rather than "no workspace filter":
            // silently widening a narrowing header is exactly the failure mode
            // HBR-PRIV-003 is about.
            let workspace = WorkspaceScopeRef::new(raw_workspace)
                .map_err(|_| AccountScopeRejection::malformed())?;
            query = query.within_workspace(workspace);
        }

        Ok(Self(query))
    }
}

/// Shape of the denial body, for tests and for the UserManual entry.
pub fn rejection_body_shape() -> serde_json::Value {
    json!({ "error": MISSING_SCOPE_CODE, "detail": "<fixed hint, never resource metadata>" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    async fn extract(headers: &[(&str, &str)]) -> Result<RequestAccountScope, AccountScopeRejection> {
        let mut builder = Request::builder().uri("/swarm/model-lanes/navigation/runs/run-1");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let request = builder.body(()).expect("build request");
        let (mut parts, ()) = request.into_parts();
        RequestAccountScope::from_request_parts(&mut parts, &()).await
    }

    #[tokio::test]
    async fn a_request_with_no_scope_header_is_denied_not_widened() {
        // The whole point: absence must be a denial. If this ever returns Ok,
        // the route behind it has silently gone back to "return everything".
        let rejection = extract(&[])
            .await
            .expect_err("a route with no owning-account scope must be refused");
        assert_eq!(rejection.code(), MISSING_SCOPE_CODE);
        assert_eq!(
            rejection.into_response().status(),
            StatusCode::FORBIDDEN,
            "a missing scope must not be answered with data"
        );
    }

    #[tokio::test]
    async fn a_blank_scope_header_is_treated_as_absent_not_as_a_wildcard() {
        let rejection = extract(&[(OWNER_ACCOUNT_HEADER, "   ")])
            .await
            .expect_err("a blank owning-account header must be refused");
        assert_eq!(rejection.code(), MISSING_SCOPE_CODE);
    }

    #[tokio::test]
    async fn a_malformed_scope_header_is_denied() {
        let rejection = extract(&[(OWNER_ACCOUNT_HEADER, "not-a-uuid")])
            .await
            .expect_err("a malformed owning-account header must be refused");
        assert_eq!(rejection.code(), MALFORMED_SCOPE_CODE);
        assert_eq!(rejection.into_response().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_blank_workspace_header_is_malformed_rather_than_a_silent_widening() {
        let owner = Uuid::now_v7().to_string();
        let rejection = extract(&[
            (OWNER_ACCOUNT_HEADER, owner.as_str()),
            (WORKSPACE_HEADER, "  "),
        ])
        .await
        .expect_err("a present-but-blank workspace narrowing must not be dropped");
        assert_eq!(rejection.code(), MALFORMED_SCOPE_CODE);
    }

    #[tokio::test]
    async fn a_well_formed_scope_header_yields_the_owner_and_optional_workspace() {
        let owner = Uuid::now_v7();
        let scope = extract(&[(OWNER_ACCOUNT_HEADER, owner.to_string().as_str())])
            .await
            .expect("a well-formed owning-account header must be accepted");
        assert_eq!(scope.query().owner_account_id().as_uuid(), owner);
        assert!(scope.query().workspace().is_none());

        let narrowed = extract(&[
            (OWNER_ACCOUNT_HEADER, owner.to_string().as_str()),
            (WORKSPACE_HEADER, "ws-alpha"),
        ])
        .await
        .expect("a workspace-narrowed scope must be accepted");
        assert_eq!(
            narrowed.query().workspace().map(|ws| ws.as_str()),
            Some("ws-alpha")
        );
    }

    #[test]
    fn a_denial_body_never_carries_resource_metadata() {
        // HBR-PRIV-004: the rejection is built from fixed literals only, so
        // there is no path by which a resource id or owner can reach it.
        for rejection in [
            AccountScopeRejection::missing(),
            AccountScopeRejection::malformed(),
        ] {
            assert!(!rejection.detail.contains("run"));
            assert!(!rejection.detail.contains("owner_account_id="));
        }
    }
}

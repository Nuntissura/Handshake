use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Account that OWNS a resource. Distinct from the actor that touched it:
/// HBR-PRIV-005 requires owner and actor to remain separately recoverable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OwnerAccountId(Uuid);

/// Principal that PERFORMED the write. May differ from the owner (a model lane
/// acting on an operator's resource), which is exactly the delegation case
/// HBR-PRIV-005 wants attributable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorPrincipalId(Uuid);

/// Authenticated session the write was made under. Pinning this is what lets
/// HBR-PRIV-006 prove a running lane is not silently retargeted when the
/// operator switches context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuthenticatedSessionRef(Uuid);

/// Opaque AccessSpace seam. WP-1 stores and compares it; it does not interpret
/// it. WP-KERNEL-007 defines what selection over an AccessSpace means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccessSpaceRef(Uuid);

macro_rules! uuid_newtype_impls {
    ($($ty:ident),+ $(,)?) => {$(
        impl $ty {
            /// Wrap an existing identifier. Minting is deliberately separate so
            /// call sites cannot accidentally invent identity where they meant
            /// to carry it.
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            /// Mint a fresh identifier. UUID v7 per HBR-INT-008 (time-ordered,
            /// so ownership rows keep replay locality with the event ledger).
            pub fn mint() -> Self {
                Self(Uuid::now_v7())
            }

            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }
    )+};
}

uuid_newtype_impls!(
    OwnerAccountId,
    ActorPrincipalId,
    AuthenticatedSessionRef,
    AccessSpaceRef
);

/// Workspace scope. `TEXT` rather than a UUID because the existing
/// `workspaces` table and the ~58 migrations that already carry `workspace_id`
/// use text ids; inventing a second representation here would fork the only
/// scope key that is actually resolvable today.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceScopeRef(String);

impl WorkspaceScopeRef {
    /// Reject empty/whitespace ids: an all-blank scope would silently widen a
    /// query rather than narrowing it.
    pub fn new(value: impl Into<String>) -> Result<Self, ResourceScopeError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ResourceScopeError::EmptyWorkspaceScope);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceScopeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// The scope stamped onto a resource when it is written.
///
/// `owner_account_id` and `actor_principal_id` are **required**. A resource
/// written without an owner is undiscoverable-but-also-unprotected, which is
/// the state HBR-PRIV-001 forbids.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceScope {
    pub owner_account_id: OwnerAccountId,
    pub actor_principal_id: ActorPrincipalId,
    pub authenticated_session: Option<AuthenticatedSessionRef>,
    pub access_space: Option<AccessSpaceRef>,
    pub workspace: Option<WorkspaceScopeRef>,
}

impl ResourceScope {
    pub fn new(owner_account_id: OwnerAccountId, actor_principal_id: ActorPrincipalId) -> Self {
        Self {
            owner_account_id,
            actor_principal_id,
            authenticated_session: None,
            access_space: None,
            workspace: None,
        }
    }

    pub fn with_session(mut self, session: AuthenticatedSessionRef) -> Self {
        self.authenticated_session = Some(session);
        self
    }

    pub fn with_access_space(mut self, access_space: AccessSpaceRef) -> Self {
        self.access_space = Some(access_space);
        self
    }

    pub fn with_workspace(mut self, workspace: WorkspaceScopeRef) -> Self {
        self.workspace = Some(workspace);
        self
    }

    /// Scope for a resource that a derived artifact inherits from its sources.
    ///
    /// HBR-PRIV-004: a derivative must inherit an access scope **no broader
    /// than all contributing sources**. Rather than trying to compute a union
    /// (which would widen), this refuses to derive across mixed owners, actor
    /// Principals, authenticated sessions, AccessSpaces, or workspaces and
    /// forces the caller to handle the mixed-scope case explicitly. The
    /// requested actor must equal the source actor because this seam carries no
    /// delegation chain or grant intersection capable of authorizing a
    /// retarget. AccessSpace semantics are intentionally opaque in this WP-1
    /// seam, so refusing mismatches is the only safe local operation. A refusal
    /// is recoverable; a silent widening is a leak.
    pub fn derive_from_sources<'a>(
        sources: impl IntoIterator<Item = &'a ResourceScope>,
        actor_principal_id: ActorPrincipalId,
    ) -> Result<Self, ResourceScopeError> {
        let mut iter = sources.into_iter();
        let first = iter.next().ok_or(ResourceScopeError::NoDerivationSources)?;

        if actor_principal_id != first.actor_principal_id {
            return Err(ResourceScopeError::DerivativeActorRetargetDenied);
        }

        let derived = first.clone();

        for source in iter {
            if source.owner_account_id != derived.owner_account_id {
                return Err(ResourceScopeError::MixedOwnerDerivation {
                    first: derived.owner_account_id,
                    conflicting: source.owner_account_id,
                });
            }
            if source.actor_principal_id != derived.actor_principal_id {
                return Err(ResourceScopeError::MixedActorPrincipalDerivation);
            }
            if source.authenticated_session != derived.authenticated_session {
                return Err(ResourceScopeError::MixedAuthenticatedSessionDerivation);
            }
            if source.access_space != derived.access_space {
                return Err(ResourceScopeError::MixedAccessSpaceDerivation);
            }
            if source.workspace != derived.workspace {
                return Err(ResourceScopeError::MixedWorkspaceDerivation);
            }
        }

        Ok(derived)
    }
}

/// Complete five-dimensional attribution carried by account-facing runtime
/// projections. Unlike [`ResourceScope`], none of the projection dimensions
/// are optional: emitting an identifier-bearing diagnostic without one of
/// these fields would make the projection impossible to authorize exactly.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExactResourceScopeAttribution {
    pub owner_account_id: OwnerAccountId,
    pub actor_principal_id: ActorPrincipalId,
    pub authenticated_session_id: AuthenticatedSessionRef,
    pub access_space_id: AccessSpaceRef,
    pub workspace_id: WorkspaceScopeRef,
}

impl ExactResourceScopeAttribution {
    /// Freeze the exact scope of a durable resource into a projection-safe
    /// value. Missing dimensions are rejected rather than silently omitted.
    pub fn try_from_resource_scope(scope: &ResourceScope) -> Result<Self, ResourceScopeError> {
        Ok(Self {
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session.ok_or(
                ResourceScopeError::IncompleteProjectionAttribution {
                    dimension: "authenticated_session_id",
                },
            )?,
            access_space_id: scope.access_space.ok_or(
                ResourceScopeError::IncompleteProjectionAttribution {
                    dimension: "access_space_id",
                },
            )?,
            workspace_id: scope.workspace.clone().ok_or(
                ResourceScopeError::IncompleteProjectionAttribution {
                    dimension: "workspace_id",
                },
            )?,
        })
    }

    pub fn as_stored_scope(&self) -> StoredResourceScope {
        StoredResourceScope {
            owner_account_id: Some(self.owner_account_id),
            actor_principal_id: Some(self.actor_principal_id),
            authenticated_session: Some(self.authenticated_session_id),
            access_space: Some(self.access_space_id),
            workspace: Some(self.workspace_id.clone()),
        }
    }

    pub fn authorize(&self, query: &ResourceScopeQuery) -> Result<(), ScopeDenied> {
        query.authorize_row(&self.as_stored_scope())
    }

    /// Stamp the five flat field names used by durable metadata and Flight
    /// Recorder JSON. A non-object payload is rejected instead of replaced.
    pub fn stamp_json_object(
        &self,
        payload: &mut serde_json::Value,
    ) -> Result<(), ResourceScopeError> {
        let object = payload
            .as_object_mut()
            .ok_or(ResourceScopeError::ProjectionPayloadNotObject)?;
        let fields = serde_json::to_value(self)
            .map_err(|_| ResourceScopeError::ProjectionAttributionSerialization)?;
        let fields = fields
            .as_object()
            .ok_or(ResourceScopeError::ProjectionAttributionSerialization)?;
        for (key, value) in fields {
            object.insert(key.clone(), value.clone());
        }
        Ok(())
    }
}

/// The scope a reader presents when asking for resources.
///
/// Constructed only from an authenticated context. There is intentionally no
/// `ResourceScopeQuery::unscoped()` constructor: an unscoped read is the thing
/// HBR-PRIV-002 forbids, so it must not be one function call away.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceScopeQuery {
    owner_account_id: OwnerAccountId,
    workspace: Option<WorkspaceScopeRef>,
}

impl ResourceScopeQuery {
    pub fn for_owner(owner_account_id: OwnerAccountId) -> Self {
        Self {
            owner_account_id,
            workspace: None,
        }
    }

    pub fn within_workspace(mut self, workspace: WorkspaceScopeRef) -> Self {
        self.workspace = Some(workspace);
        self
    }

    pub const fn owner_account_id(&self) -> OwnerAccountId {
        self.owner_account_id
    }

    pub fn workspace(&self) -> Option<&WorkspaceScopeRef> {
        self.workspace.as_ref()
    }

    /// Decide whether a stored row may be disclosed to this reader.
    ///
    /// Fails closed on missing ownership: a row whose `owner_account_id` is
    /// NULL (a pre-0363 legacy row) is **denied**, not grandfathered in. Legacy
    /// rows are exactly the ones with no provenance, so defaulting them to
    /// visible would reproduce the leak this boundary exists to close.
    pub fn authorize_row(&self, row: &StoredResourceScope) -> Result<(), ScopeDenied> {
        let Some(owner) = row.owner_account_id else {
            return Err(ScopeDenied::UnattributedResource);
        };

        if owner != self.owner_account_id {
            return Err(ScopeDenied::OwnerMismatch {
                requested: self.owner_account_id,
                stored: owner,
            });
        }

        if let Some(requested_workspace) = self.workspace.as_ref() {
            match row.workspace.as_ref() {
                Some(stored) if stored == requested_workspace => {}
                Some(stored) => {
                    return Err(ScopeDenied::WorkspaceMismatch {
                        requested: requested_workspace.clone(),
                        stored: Some(stored.clone()),
                    })
                }
                None => {
                    return Err(ScopeDenied::WorkspaceMismatch {
                        requested: requested_workspace.clone(),
                        stored: None,
                    })
                }
            }
        }

        Ok(())
    }
}

/// The scope decoded from durable transport. Every field is optional
/// because the columns are nullable until WP-KERNEL-006 MT-015 tightens them;
/// `authorize_row` is what turns that nullability into a denial rather than a
/// silent allow.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StoredResourceScope {
    pub owner_account_id: Option<OwnerAccountId>,
    pub actor_principal_id: Option<ActorPrincipalId>,
    pub authenticated_session: Option<AuthenticatedSessionRef>,
    pub access_space: Option<AccessSpaceRef>,
    pub workspace: Option<WorkspaceScopeRef>,
}

impl From<&ResourceScope> for StoredResourceScope {
    fn from(scope: &ResourceScope) -> Self {
        Self {
            owner_account_id: Some(scope.owner_account_id),
            actor_principal_id: Some(scope.actor_principal_id),
            authenticated_session: scope.authenticated_session,
            access_space: scope.access_space,
            workspace: scope.workspace.clone(),
        }
    }
}

/// Why a read was refused. Carries enough for an operator-facing denial reason
/// (HBR-PRIV-008) **without** disclosing the restricted resource's metadata:
/// the caller learns that the row is not theirs, not what the row contains.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ScopeDenied {
    #[error("resource has no owning account recorded; denied by default")]
    UnattributedResource,
    #[error("resource is owned by a different account")]
    OwnerMismatch {
        requested: OwnerAccountId,
        stored: OwnerAccountId,
    },
    #[error("resource is outside the requested workspace scope")]
    WorkspaceMismatch {
        requested: WorkspaceScopeRef,
        stored: Option<WorkspaceScopeRef>,
    },
    #[error("resource attribution does not match the exact server scope")]
    ExactAttributionMismatch,
    #[error("authenticated resource context is unknown")]
    LifecycleUnknown,
    #[error("authenticated resource context is stale")]
    LifecycleStale,
    #[error("authenticated resource context is revoked")]
    LifecycleRevoked,
    #[error("authenticated resource lifecycle authority is unavailable")]
    LifecycleAuthorityUnavailable,
}

impl ScopeDenied {
    /// Stable machine-readable denial reason for typed receipts and UI. The
    /// variants deliberately do not embed identifiers, so a denial surfaced to
    /// an operator cannot become a metadata side channel (HBR-PRIV-004).
    pub const fn reason_code(&self) -> &'static str {
        match self {
            Self::UnattributedResource => "RESOURCE_SCOPE_UNATTRIBUTED",
            Self::OwnerMismatch { .. } => "RESOURCE_SCOPE_OWNER_MISMATCH",
            Self::WorkspaceMismatch { .. } => "RESOURCE_SCOPE_WORKSPACE_MISMATCH",
            Self::ExactAttributionMismatch => "RESOURCE_SCOPE_EXACT_ATTRIBUTION_MISMATCH",
            Self::LifecycleUnknown => "RESOURCE_ACCESS_CONTEXT_UNKNOWN",
            Self::LifecycleStale => "RESOURCE_ACCESS_CONTEXT_STALE",
            Self::LifecycleRevoked => "RESOURCE_ACCESS_CONTEXT_REVOKED",
            Self::LifecycleAuthorityUnavailable => "RESOURCE_ACCESS_LIFECYCLE_UNAVAILABLE",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ResourceScopeError {
    #[error("workspace scope id must not be empty")]
    EmptyWorkspaceScope,
    #[error("cannot derive a resource scope from zero sources")]
    NoDerivationSources,
    #[error("cannot derive one resource across mixed owning accounts")]
    MixedOwnerDerivation {
        first: OwnerAccountId,
        conflicting: OwnerAccountId,
    },
    #[error("cannot retarget a derived resource to an actor without delegation authority")]
    DerivativeActorRetargetDenied,
    #[error("cannot derive one resource across mixed actor Principals")]
    MixedActorPrincipalDerivation,
    #[error("cannot derive one resource across mixed authenticated sessions")]
    MixedAuthenticatedSessionDerivation,
    #[error("cannot derive one resource across mixed AccessSpaces")]
    MixedAccessSpaceDerivation,
    #[error("cannot derive one resource across mixed workspaces")]
    MixedWorkspaceDerivation,
    #[error("runtime projection scope is missing required dimension {dimension}")]
    IncompleteProjectionAttribution { dimension: &'static str },
    #[error("runtime projection payload must be a JSON object")]
    ProjectionPayloadNotObject,
    #[error("runtime projection scope could not be serialized")]
    ProjectionAttributionSerialization,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> OwnerAccountId {
        OwnerAccountId::mint()
    }

    fn actor() -> ActorPrincipalId {
        ActorPrincipalId::mint()
    }

    #[test]
    fn minted_ids_are_uuid_v7_and_time_ordered() {
        // HBR-INT-008: new mint sites must be v7, and v7 must actually order.
        let first = OwnerAccountId::mint();
        let second = OwnerAccountId::mint();
        assert_eq!(first.as_uuid().get_version_num(), 7);
        assert_eq!(second.as_uuid().get_version_num(), 7);
        assert!(first.as_uuid() <= second.as_uuid());
    }

    #[test]
    fn unattributed_rows_are_denied_not_grandfathered() {
        let query = ResourceScopeQuery::for_owner(owner());
        let legacy_row = StoredResourceScope::default();

        let denied = query
            .authorize_row(&legacy_row)
            .expect_err("a legacy row with no owner must not be readable");
        assert_eq!(denied, ScopeDenied::UnattributedResource);
        assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_UNATTRIBUTED");
    }

    #[test]
    fn a_different_account_cannot_read_the_row() {
        let mine = owner();
        let theirs = owner();
        assert_ne!(mine, theirs);

        let their_row = StoredResourceScope::from(&ResourceScope::new(theirs, actor()));

        let denied = ResourceScopeQuery::for_owner(mine)
            .authorize_row(&their_row)
            .expect_err("cross-account read must be denied");
        assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_OWNER_MISMATCH");
    }

    #[test]
    fn the_owning_account_can_read_its_own_row() {
        let mine = owner();
        let my_row = StoredResourceScope::from(&ResourceScope::new(mine, actor()));

        ResourceScopeQuery::for_owner(mine)
            .authorize_row(&my_row)
            .expect("the owning account must still be able to read its own resource");
    }

    #[test]
    fn workspace_scope_narrows_within_one_account() {
        // Same owner, different workspace: still denied. This is the
        // same-project privacy case in HBR-PRIV-003 that role labels could
        // never express.
        let mine = owner();
        let alpha = WorkspaceScopeRef::new("ws-alpha").unwrap();
        let beta = WorkspaceScopeRef::new("ws-beta").unwrap();

        let beta_row = StoredResourceScope::from(
            &ResourceScope::new(mine, actor()).with_workspace(beta.clone()),
        );

        let denied = ResourceScopeQuery::for_owner(mine)
            .within_workspace(alpha)
            .authorize_row(&beta_row)
            .expect_err("a row in another workspace must not be disclosed");
        assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_WORKSPACE_MISMATCH");
    }

    #[test]
    fn a_workspace_scoped_read_denies_rows_with_no_workspace() {
        let mine = owner();
        let row = StoredResourceScope::from(&ResourceScope::new(mine, actor()));

        let denied = ResourceScopeQuery::for_owner(mine)
            .within_workspace(WorkspaceScopeRef::new("ws-alpha").unwrap())
            .authorize_row(&row)
            .expect_err("an unscoped row must not satisfy a workspace-scoped read");
        assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_WORKSPACE_MISMATCH");
    }

    #[test]
    fn empty_workspace_ids_are_rejected() {
        assert_eq!(
            WorkspaceScopeRef::new("   ").expect_err("blank workspace must be rejected"),
            ResourceScopeError::EmptyWorkspaceScope
        );
    }

    #[test]
    fn derivation_refuses_to_span_two_owners() {
        let source_actor = actor();
        let a = ResourceScope::new(owner(), source_actor);
        let b = ResourceScope::new(owner(), source_actor);

        let error = ResourceScope::derive_from_sources([&a, &b], source_actor)
            .expect_err("a derivative must not span two owning accounts");
        assert!(matches!(
            error,
            ResourceScopeError::MixedOwnerDerivation { .. }
        ));
    }

    #[test]
    fn derivation_refuses_mixed_workspaces_instead_of_widening_to_owner() {
        let mine = owner();
        let alpha = WorkspaceScopeRef::new("ws-alpha").unwrap();
        let beta = WorkspaceScopeRef::new("ws-beta").unwrap();
        let source_actor = actor();

        let a = ResourceScope::new(mine, source_actor).with_workspace(alpha.clone());
        let b = ResourceScope::new(mine, source_actor).with_workspace(beta);

        let error = ResourceScope::derive_from_sources([&a, &b], source_actor)
            .expect_err("mixed workspaces have no safe common derivative scope");
        assert!(matches!(
            error,
            ResourceScopeError::MixedWorkspaceDerivation
        ));
    }

    #[test]
    fn derivation_preserves_the_complete_exact_scope_common_to_all_sources() {
        let mine = owner();
        let source_actor = actor();
        let session = AuthenticatedSessionRef::mint();
        let access_space = AccessSpaceRef::mint();
        let alpha = WorkspaceScopeRef::new("ws-alpha").unwrap();

        let a = ResourceScope::new(mine, source_actor)
            .with_session(session)
            .with_access_space(access_space)
            .with_workspace(alpha);
        let b = a.clone();

        let derived = ResourceScope::derive_from_sources([&a, &b], source_actor).unwrap();
        assert_eq!(derived, a);
    }

    #[test]
    fn derivation_refuses_mixed_access_spaces_instead_of_widening_to_owner() {
        let mine = owner();
        let source_actor = actor();
        let a = ResourceScope::new(mine, source_actor).with_access_space(AccessSpaceRef::mint());
        let b = ResourceScope::new(mine, source_actor).with_access_space(AccessSpaceRef::mint());

        assert_eq!(
            ResourceScope::derive_from_sources([&a, &b], source_actor)
                .expect_err("mixed AccessSpaces have no safe local derivative scope"),
            ResourceScopeError::MixedAccessSpaceDerivation
        );
    }

    #[test]
    fn derivation_refuses_actor_retarget_without_delegation_authority() {
        let source = ResourceScope::new(owner(), actor())
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(WorkspaceScopeRef::new("ws-alpha").unwrap());

        assert_eq!(
            ResourceScope::derive_from_sources([&source], actor())
                .expect_err("an arbitrary derivative actor has no delegation authority"),
            ResourceScopeError::DerivativeActorRetargetDenied
        );
    }

    #[test]
    fn derivation_refuses_mixed_source_actors() {
        let mine = owner();
        let source_actor = actor();
        let a = ResourceScope::new(mine, source_actor);
        let b = ResourceScope::new(mine, actor());

        assert_eq!(
            ResourceScope::derive_from_sources([&a, &b], source_actor)
                .expect_err("mixed source actors have no validated delegation intersection"),
            ResourceScopeError::MixedActorPrincipalDerivation
        );
    }

    #[test]
    fn derivation_refuses_different_or_missing_authenticated_sessions() {
        let mine = owner();
        let source_actor = actor();
        let a =
            ResourceScope::new(mine, source_actor).with_session(AuthenticatedSessionRef::mint());
        let different =
            ResourceScope::new(mine, source_actor).with_session(AuthenticatedSessionRef::mint());
        let missing = ResourceScope::new(mine, source_actor);

        for conflicting in [&different, &missing] {
            for sources in [[&a, conflicting], [conflicting, &a]] {
                assert_eq!(
                    ResourceScope::derive_from_sources(sources, source_actor)
                        .expect_err("session mismatch must be denied in either source order"),
                    ResourceScopeError::MixedAuthenticatedSessionDerivation
                );
            }
        }
    }

    #[test]
    fn derivation_checks_every_contributing_source() {
        let mine = owner();
        let source_actor = actor();
        let session = AuthenticatedSessionRef::mint();
        let first = ResourceScope::new(mine, source_actor).with_session(session);
        let second = first.clone();
        let third =
            ResourceScope::new(mine, source_actor).with_session(AuthenticatedSessionRef::mint());

        assert_eq!(
            ResourceScope::derive_from_sources([&first, &second, &third], source_actor)
                .expect_err("a conflicting final source must not escape validation"),
            ResourceScopeError::MixedAuthenticatedSessionDerivation
        );
    }

    #[test]
    fn derivation_requires_at_least_one_source() {
        assert_eq!(
            ResourceScope::derive_from_sources([], actor())
                .expect_err("zero-source derivation must fail"),
            ResourceScopeError::NoDerivationSources
        );
    }

    #[test]
    fn denial_reasons_do_not_leak_identifiers() {
        // HBR-PRIV-004: a denial surfaced to an operator must not become a
        // metadata side channel for the restricted resource.
        let mine = owner();
        let theirs = owner();
        let their_row = StoredResourceScope::from(&ResourceScope::new(theirs, actor()));

        let denied = ResourceScopeQuery::for_owner(mine)
            .authorize_row(&their_row)
            .unwrap_err();

        let rendered = denied.to_string();
        assert!(
            !rendered.contains(&theirs.to_string()),
            "denial message must not disclose the owning account id: {rendered}"
        );
    }
}

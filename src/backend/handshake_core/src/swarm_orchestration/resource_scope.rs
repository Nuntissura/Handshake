//! Account-bound resource scope for WP-1 model-lane resources (HBR-PRIV).
//!
//! # Why this exists
//!
//! `HANDSHAKE_BUILD_RULES` v1.8.0 added the PRIV pillar: every durable product
//! resource must carry a stable resource identity plus an authoritative
//! owning-account / Principal / AccessSpace linkage *before* it is discoverable
//! or usable, and every read boundary must deny by default when that context is
//! missing, stale, or mismatched (HBR-PRIV-001, HBR-PRIV-002).
//!
//! The WP-1 model-lane tables predate that pillar. The only owner-ish column
//! they carried, `owner_session`, is **not an owner**: it is a governance role
//! label (literals such as `swarm_coordinator` / `KERNEL_BUILDER`), identical
//! for every operator on every machine. Treating it as an owner would have made
//! every isolation test pass vacuously, which is precisely the failure mode
//! HBR-PRIV exists to prevent.
//!
//! # Relationship to WP-KERNEL-006 / WP-KERNEL-007
//!
//! `WP-KERNEL-006` owns the real identity substrate (`LocalAccount`,
//! `Principal`, `AuthenticatedSession`, embedded storage predicates), and its MT-015
//! `AuthorityTableOwnershipColumns` is the declared owner of the ownership
//! columns on Kernel V1 authority tables. `WP-KERNEL-007` owns `ResourceGrant`
//! and `AccessSpace` semantics.
//!
//! The types here are therefore deliberately **WP-1-local and narrow**:
//!
//! * they are named so they will *not* collide with the `LocalAccountId` /
//!   `PrincipalId` / `AccessSpaceId` types KERNEL-006 will introduce;
//! * they carry exactly the identifiers that migration
//!   `0363_model_lane_account_resource_scope.sql` persists, using the same
//!   column names KERNEL-006 MT-015 will take over, so the takeover is a type
//!   swap and a `NOT NULL` tightening rather than a rename across 21 tables;
//! * they define **no** AccessSpace semantics. `access_space_id` is carried as
//!   an opaque seam only; interpreting it belongs to KERNEL-007.
//!
//! Enforcement built on these types is explicitly **application-layer and
//! pre-RLS**. It is a real boundary today, not a stand-in for the RLS that
//! KERNEL-006 MT-014/016 will add underneath it. Both layers are wanted:
//! HBR-PRIV-002 requires enforcement at *every* applicable boundary and states
//! that hiding a row in one layer is never sufficient.

use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, RwLock},
};

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

// ---------------------------------------------------------------------------
// Authorization attribution (HBR-PRIV-005 / HBR-PRIV-007)
// ---------------------------------------------------------------------------

/// WHO authorized a durable authorization artifact, as a **typed value** instead
/// of a formatted string.
///
/// # The concrete defect this type replaces
///
/// The cloud consent path used to record its approver as
/// `format!("operator://{}/cloud-selection", selection.owner_session)` next to
/// `approved: true`. `owner_session` is a governance ROLE LABEL (literals such as
/// `swarm_coordinator` / `KERNEL_BUILDER`), identical for every operator on every
/// machine, so the receipt was **self-issued**: it recorded that "the role that
/// asked for the export approved the export". A receipt like that proves nothing
/// about who consented, on the exact path that authorizes sending operator data
/// to a third-party cloud provider.
///
/// A `String` cannot be fixed by convention, because nothing stops the next call
/// site from formatting whatever it likes into it. So the authorization surface
/// is this enum, it is REQUIRED on the receipt, and it can only be built from an
/// account context ([`Self::from_scope`] / [`Self::from_access`]) or from an
/// explicit, named admission that no account context existed
/// ([`Self::unattributed`]).
///
/// # Why the `Unattributed` variant exists and why it is safe
///
/// Handshake has no authentication layer yet (`WP-KERNEL-006` owns it), so some
/// call sites genuinely have no account to bind to. The honest record for those
/// is "nobody authenticated approved this", not a fabricated operator identity.
/// [`Self::authorizes`] routes that variant to
/// [`ScopeDenied::UnattributedResource`], i.e. an `Unattributed` approval can
/// never satisfy an account-scoped authorization check — it is durable
/// provenance, never authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "authority_kind", rename_all = "snake_case")]
pub enum AccountBoundAuthority {
    /// A real, account-bound authorization.
    Account {
        owner_account_id: OwnerAccountId,
        actor_principal_id: ActorPrincipalId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        authenticated_session: Option<AuthenticatedSessionRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        access_space: Option<AccessSpaceRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workspace: Option<WorkspaceScopeRef>,
    },
    /// **Not** an authorization. Recorded when the writing call site had no
    /// authenticated account context at all.
    Unattributed { reason: String },
}

impl AccountBoundAuthority {
    /// Bind to a concrete write scope. This is the only way to produce an
    /// authority that can actually satisfy [`Self::authorizes`].
    pub fn from_scope(scope: &ResourceScope) -> Self {
        Self::Account {
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session: scope.authenticated_session,
            access_space: scope.access_space,
            workspace: scope.workspace.clone(),
        }
    }

    /// Explicitly record that no authenticated account existed. `reason` must be
    /// stable and machine-readable so an auditor can enumerate every such row.
    pub fn unattributed(reason: impl Into<String>) -> Self {
        Self::Unattributed {
            reason: reason.into(),
        }
    }

    /// Derive from a store/request access context: account-bound when the
    /// context can write on behalf of an account, explicitly unattributed
    /// otherwise. A read-only or system context therefore CANNOT mint an
    /// account-bound approval.
    pub fn from_access(access: &ResourceAccessContext) -> Self {
        match access.write_scope() {
            Some(scope) => Self::from_scope(scope),
            None => Self::unattributed(
                access
                    .system_authority()
                    .map(|authority| authority.reason())
                    .unwrap_or("SYSTEM_SCOPE_READ_ONLY_CONTEXT_CANNOT_APPROVE"),
            ),
        }
    }

    pub const fn owner_account_id(&self) -> Option<OwnerAccountId> {
        match self {
            Self::Account {
                owner_account_id, ..
            } => Some(*owner_account_id),
            Self::Unattributed { .. } => None,
        }
    }

    pub const fn actor_principal_id(&self) -> Option<ActorPrincipalId> {
        match self {
            Self::Account {
                actor_principal_id, ..
            } => Some(*actor_principal_id),
            Self::Unattributed { .. } => None,
        }
    }

    pub const fn is_account_bound(&self) -> bool {
        matches!(self, Self::Account { .. })
    }

    /// Render as a [`StoredResourceScope`] so authorization reuses exactly the
    /// same comparison the storage read boundary uses. There is deliberately no
    /// second, parallel matching rule.
    pub fn as_stored_scope(&self) -> StoredResourceScope {
        match self {
            Self::Account {
                owner_account_id,
                actor_principal_id,
                authenticated_session,
                access_space,
                workspace,
            } => StoredResourceScope {
                owner_account_id: Some(*owner_account_id),
                actor_principal_id: Some(*actor_principal_id),
                authenticated_session: *authenticated_session,
                access_space: *access_space,
                workspace: workspace.clone(),
            },
            Self::Unattributed { .. } => StoredResourceScope::default(),
        }
    }

    /// Decide whether this authorization satisfies a reader/launcher operating
    /// under `query`. `Unattributed` always fails.
    pub fn authorizes(&self, query: &ResourceScopeQuery) -> Result<(), ScopeDenied> {
        query.authorize_row(&self.as_stored_scope())
    }

    /// True when both authorities name the same owning account, including the
    /// "both unattributed" case. Used to prove a projection's source scope and
    /// its consent receipt's approver are the same account.
    pub fn same_owner_as(&self, other: &Self) -> bool {
        self.owner_account_id() == other.owner_account_id()
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

// ---------------------------------------------------------------------------
// Store-level access context (write stamping + read enforcement)
// ---------------------------------------------------------------------------

/// Server-owned lifecycle decision for one exact authenticated resource
/// context. `Stale` and `Revoked` are terminal for that exact session identity;
/// callers must register a new authenticated session instead of reactivating it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAccessLifecycleState {
    Active,
    Stale,
    Revoked,
}

/// Observable decision returned without echoing any account, session, Space,
/// or workspace identifier into denial surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceAccessLifecycleDecision {
    Unknown,
    Known {
        state: ResourceAccessLifecycleState,
        transition_sequence: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResourceAccessLifecycleEntry {
    state: ResourceAccessLifecycleState,
    transition_sequence: u64,
}

#[derive(Debug, Default)]
struct ResourceAccessLifecycleRegistryState {
    entries: BTreeMap<ExactResourceScopeAttribution, ResourceAccessLifecycleEntry>,
    next_transition_sequence: u64,
}

/// Central in-process authentication/session authority shared by every store
/// and route serving one runtime composition root. It is intentionally not
/// durable product data: durable ModelLane attribution remains in SurrealDB,
/// while current authentication lifecycle is guarded by this concurrency-safe
/// server-owned registry.
#[derive(Clone, Debug, Default)]
pub struct ResourceAccessLifecycleRegistry {
    state: Arc<RwLock<ResourceAccessLifecycleRegistryState>>,
}

impl PartialEq for ResourceAccessLifecycleRegistry {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }
}

impl Eq for ResourceAccessLifecycleRegistry {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ResourceAccessLifecycleTransitionError {
    #[error("authenticated resource context is unknown")]
    UnknownContext,
    #[error("authenticated resource context cannot be reactivated")]
    TerminalContext,
    #[error("authenticated resource lifecycle authority is unavailable")]
    AuthorityUnavailable,
}

impl ResourceAccessLifecycleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a newly authenticated exact context. Repeating registration
    /// while it is active is idempotent; stale/revoked identities cannot be
    /// reactivated and require a newly minted session identity.
    pub fn register_active(
        &self,
        exact: ExactResourceScopeAttribution,
    ) -> Result<ResourceAccessLifecycleDecision, ResourceAccessLifecycleTransitionError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| ResourceAccessLifecycleTransitionError::AuthorityUnavailable)?;
        if let Some(entry) = state.entries.get(&exact).copied() {
            return match entry.state {
                ResourceAccessLifecycleState::Active => {
                    Ok(ResourceAccessLifecycleDecision::Known {
                        state: entry.state,
                        transition_sequence: entry.transition_sequence,
                    })
                }
                ResourceAccessLifecycleState::Stale | ResourceAccessLifecycleState::Revoked => {
                    Err(ResourceAccessLifecycleTransitionError::TerminalContext)
                }
            };
        }
        state.next_transition_sequence = state.next_transition_sequence.saturating_add(1);
        let entry = ResourceAccessLifecycleEntry {
            state: ResourceAccessLifecycleState::Active,
            transition_sequence: state.next_transition_sequence,
        };
        state.entries.insert(exact, entry);
        Ok(ResourceAccessLifecycleDecision::Known {
            state: entry.state,
            transition_sequence: entry.transition_sequence,
        })
    }

    pub fn mark_stale(
        &self,
        exact: &ExactResourceScopeAttribution,
    ) -> Result<ResourceAccessLifecycleDecision, ResourceAccessLifecycleTransitionError> {
        self.transition(exact, ResourceAccessLifecycleState::Stale)
    }

    pub fn revoke(
        &self,
        exact: &ExactResourceScopeAttribution,
    ) -> Result<ResourceAccessLifecycleDecision, ResourceAccessLifecycleTransitionError> {
        self.transition(exact, ResourceAccessLifecycleState::Revoked)
    }

    fn transition(
        &self,
        exact: &ExactResourceScopeAttribution,
        target: ResourceAccessLifecycleState,
    ) -> Result<ResourceAccessLifecycleDecision, ResourceAccessLifecycleTransitionError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| ResourceAccessLifecycleTransitionError::AuthorityUnavailable)?;
        let current = state
            .entries
            .get(exact)
            .copied()
            .ok_or(ResourceAccessLifecycleTransitionError::UnknownContext)?;
        if current.state == target
            || (current.state == ResourceAccessLifecycleState::Revoked
                && target == ResourceAccessLifecycleState::Stale)
        {
            return Ok(ResourceAccessLifecycleDecision::Known {
                state: current.state,
                transition_sequence: current.transition_sequence,
            });
        }
        state.next_transition_sequence = state.next_transition_sequence.saturating_add(1);
        let entry = ResourceAccessLifecycleEntry {
            state: target,
            transition_sequence: state.next_transition_sequence,
        };
        state.entries.insert(exact.clone(), entry);
        Ok(ResourceAccessLifecycleDecision::Known {
            state: entry.state,
            transition_sequence: entry.transition_sequence,
        })
    }

    pub fn decision(
        &self,
        exact: &ExactResourceScopeAttribution,
    ) -> Result<ResourceAccessLifecycleDecision, ResourceAccessLifecycleTransitionError> {
        let state = self
            .state
            .read()
            .map_err(|_| ResourceAccessLifecycleTransitionError::AuthorityUnavailable)?;
        Ok(match state.entries.get(exact) {
            Some(entry) => ResourceAccessLifecycleDecision::Known {
                state: entry.state,
                transition_sequence: entry.transition_sequence,
            },
            None => ResourceAccessLifecycleDecision::Unknown,
        })
    }

    pub fn authorize(&self, exact: &ExactResourceScopeAttribution) -> Result<(), ScopeDenied> {
        match self.decision(exact) {
            Ok(ResourceAccessLifecycleDecision::Known {
                state: ResourceAccessLifecycleState::Active,
                ..
            }) => Ok(()),
            Ok(ResourceAccessLifecycleDecision::Known {
                state: ResourceAccessLifecycleState::Stale,
                ..
            }) => Err(ScopeDenied::LifecycleStale),
            Ok(ResourceAccessLifecycleDecision::Known {
                state: ResourceAccessLifecycleState::Revoked,
                ..
            }) => Err(ScopeDenied::LifecycleRevoked),
            Ok(ResourceAccessLifecycleDecision::Unknown) => Err(ScopeDenied::LifecycleUnknown),
            Err(_) => Err(ScopeDenied::LifecycleAuthorityUnavailable),
        }
    }
}

/// Authority for a read or write that is **intentionally cross-owner**.
///
/// This type exists so that "no account filter" can never be an accident. Boot
/// restart recovery genuinely has to enumerate every owner's restartable run
/// before any account context exists; that is a legitimate system operation and
/// it is named here rather than left as an unscoped query. Every construction
/// site carries a stable machine-readable reason so the bypass is auditable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemScopeAuthority {
    reason: &'static str,
}

impl SystemScopeAuthority {
    /// Startup restart-recovery. Deliberately spans every owning account: a run
    /// abandoned by a crashed process must be reclaimed before anyone has
    /// authenticated. See `ModelLaneStore::recover_restartable_runs_at_boot`.
    pub const fn boot_recovery() -> Self {
        Self {
            reason: "SYSTEM_SCOPE_BOOT_RESTART_RECOVERY",
        }
    }

    /// An in-process subsystem that operates on behalf of the whole node rather
    /// than on behalf of one account (schema proofs, harness fixtures, internal
    /// reconcilers). `reason` MUST be a stable literal, not a formatted string.
    pub const fn internal_subsystem(reason: &'static str) -> Self {
        Self { reason }
    }

    /// The residual pre-WP-KERNEL-006 bypass: a call site that predates account
    /// identity and has no authenticated context to derive a scope from.
    ///
    /// Rows written through a store holding this authority are stamped with a
    /// NULL `owner_account_id`, i.e. they are **unattributed** and therefore
    /// unreadable by every account-scoped reader (`ScopeDenied::
    /// UnattributedResource`). That is deliberate: fail-closed for readers,
    /// visibly wrong for auditors, and impossible to mistake for ownership.
    pub const fn legacy_unscoped_call_site() -> Self {
        Self {
            reason: "SYSTEM_SCOPE_LEGACY_UNSCOPED_CALL_SITE",
        }
    }

    pub const fn reason(&self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for SystemScopeAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason)
    }
}

/// An account-bound access context: what a store stamps on writes and what it
/// filters reads by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountAccessContext {
    query: ResourceScopeQuery,
    write: Option<ResourceScope>,
    exact_read: Option<ExactResourceScopeAttribution>,
    /// `None` is an explicit legacy context for consumers not yet migrated to
    /// authenticated-session lifecycle. Protected ModelLane/operator-chat
    /// boundaries reject it through [`ResourceAccessContext::require_lifecycle_active`].
    lifecycle: Option<ResourceAccessLifecycleRegistry>,
}

impl AccountAccessContext {
    pub fn query(&self) -> &ResourceScopeQuery {
        &self.query
    }

    pub fn write_scope(&self) -> Option<&ResourceScope> {
        self.write.as_ref()
    }
}

/// How one store instance is authorized. Constructed explicitly at every call
/// site; there is intentionally no `Default`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceAccessContext {
    /// Bound to exactly one owning account (and optionally one workspace).
    Account(AccountAccessContext),
    /// Explicitly cross-owner. Reads are unfiltered; writes are unattributed.
    System(SystemScopeAuthority),
}

impl ResourceAccessContext {
    /// Read+write context. Writes are stamped with `scope`; reads are filtered
    /// to `scope`'s owning account and (when set) its workspace. This preserves
    /// the pre-lifecycle behavior for unrelated consumers; protected boundaries
    /// must use [`Self::for_account_with_lifecycle`].
    pub fn for_account(scope: ResourceScope) -> Self {
        let exact_read = ExactResourceScopeAttribution::try_from_resource_scope(&scope).ok();
        let mut query = ResourceScopeQuery::for_owner(scope.owner_account_id);
        if let Some(workspace) = scope.workspace.clone() {
            query = query.within_workspace(workspace);
        }
        Self::Account(AccountAccessContext {
            query,
            write: Some(scope),
            exact_read,
            lifecycle: None,
        })
    }

    /// Construct an account context against the server-owned lifecycle
    /// authority. The exact tuple must already be registered active.
    pub fn for_account_with_lifecycle(
        scope: ResourceScope,
        lifecycle: ResourceAccessLifecycleRegistry,
    ) -> Self {
        let exact_read = ExactResourceScopeAttribution::try_from_resource_scope(&scope).ok();
        let mut query = ResourceScopeQuery::for_owner(scope.owner_account_id);
        if let Some(workspace) = scope.workspace.clone() {
            query = query.within_workspace(workspace);
        }
        Self::Account(AccountAccessContext {
            query,
            write: Some(scope),
            exact_read,
            lifecycle: Some(lifecycle),
        })
    }

    /// Read-only context. Used by API read boundaries, which carry an owning
    /// account but no actor Principal, so they must not be able to write.
    pub fn for_reader(query: ResourceScopeQuery) -> Self {
        Self::Account(AccountAccessContext {
            query,
            write: None,
            exact_read: None,
            lifecycle: None,
        })
    }

    /// Read-only context bound to all five resource-attribution dimensions.
    /// Unlike [`Self::for_account`], this context cannot stamp writes.
    pub fn for_exact_reader(exact: ExactResourceScopeAttribution) -> Self {
        let query = ResourceScopeQuery::for_owner(exact.owner_account_id)
            .within_workspace(exact.workspace_id.clone());
        Self::Account(AccountAccessContext {
            query,
            write: None,
            exact_read: Some(exact),
            lifecycle: None,
        })
    }

    pub fn for_exact_reader_with_lifecycle(
        exact: ExactResourceScopeAttribution,
        lifecycle: ResourceAccessLifecycleRegistry,
    ) -> Self {
        let query = ResourceScopeQuery::for_owner(exact.owner_account_id)
            .within_workspace(exact.workspace_id.clone());
        Self::Account(AccountAccessContext {
            query,
            write: None,
            exact_read: Some(exact),
            lifecycle: Some(lifecycle),
        })
    }

    pub fn system(authority: SystemScopeAuthority) -> Self {
        Self::System(authority)
    }

    pub fn read_query(&self) -> Option<&ResourceScopeQuery> {
        match self {
            Self::Account(account) if self.require_active().is_ok() => Some(&account.query),
            Self::Account(_) => None,
            Self::System(_) => None,
        }
    }

    pub fn write_scope(&self) -> Option<&ResourceScope> {
        match self {
            Self::Account(account) if self.require_active().is_ok() => account.write.as_ref(),
            Self::Account(_) => None,
            Self::System(_) => None,
        }
    }

    /// Exact five-dimensional read authority, when this context has one.
    /// Callers whose resources require every attribution dimension use this to
    /// reject a coarse owner/workspace context before issuing any query.
    pub fn exact_read_scope(&self) -> Option<&ExactResourceScopeAttribution> {
        match self {
            Self::Account(account) if self.require_active().is_ok() => account.exact_read.as_ref(),
            Self::Account(_) => None,
            Self::System(_) => None,
        }
    }

    pub const fn system_authority(&self) -> Option<SystemScopeAuthority> {
        match self {
            Self::Account(_) => None,
            Self::System(authority) => Some(*authority),
        }
    }

    pub const fn is_system(&self) -> bool {
        matches!(self, Self::System(_))
    }

    pub fn lifecycle_authority(&self) -> Option<&ResourceAccessLifecycleRegistry> {
        match self {
            Self::Account(account) => account.lifecycle.as_ref(),
            Self::System(_) => None,
        }
    }

    /// Preserve legacy context behavior while enforcing lifecycle whenever an
    /// authority was explicitly injected.
    pub fn require_active(&self) -> Result<(), ScopeDenied> {
        match self {
            Self::Account(AccountAccessContext {
                lifecycle: Some(lifecycle),
                exact_read,
                ..
            }) => exact_read
                .as_ref()
                .ok_or(ScopeDenied::LifecycleUnknown)
                .and_then(|exact| lifecycle.authorize(exact)),
            Self::Account(AccountAccessContext {
                lifecycle: None, ..
            }) => Ok(()),
            Self::System(_) => Ok(()),
        }
    }

    /// Protected product boundary: an explicit shared lifecycle authority is
    /// mandatory, and legacy/system contexts cannot satisfy it.
    pub fn require_lifecycle_active(&self) -> Result<(), ScopeDenied> {
        match self {
            Self::Account(AccountAccessContext {
                lifecycle: Some(lifecycle),
                exact_read: Some(exact),
                ..
            }) => lifecycle.authorize(exact),
            Self::Account(_) | Self::System(_) => Err(ScopeDenied::LifecycleUnknown),
        }
    }

    /// Validate both the current server-owned lifecycle and the exact tuple
    /// asserted by an account-facing request. The single mismatch reason is
    /// deliberately constant-shape and never reveals the bound tuple.
    pub fn authorize_exact_request(
        &self,
        request: &ExactResourceScopeAttribution,
    ) -> Result<(), ScopeDenied> {
        self.require_lifecycle_active()?;
        match self {
            Self::Account(account) if account.exact_read.as_ref() == Some(request) => Ok(()),
            Self::Account(_) | Self::System(_) => Err(ScopeDenied::ExactAttributionMismatch),
        }
    }

    /// Second enforcement layer. HBR-PRIV-002: hiding a row in one layer is
    /// never sufficient, so every read path calls this on the scope fields it
    /// read back, even though the durable query predicate should already have
    /// excluded the row.
    pub fn authorize_row(&self, row: &StoredResourceScope) -> Result<(), ScopeDenied> {
        self.require_active()?;
        match self {
            Self::Account(account) => match account.exact_read.as_ref() {
                Some(exact) if &exact.as_stored_scope() == row => Ok(()),
                Some(_) => Err(ScopeDenied::ExactAttributionMismatch),
                None => account.query.authorize_row(row),
            },
            Self::System(_) => Ok(()),
        }
    }
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

    fn exact_scope(workspace: &str) -> ResourceScope {
        ResourceScope::new(owner(), actor())
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(WorkspaceScopeRef::new(workspace).unwrap())
    }

    fn active_access(scope: ResourceScope) -> ResourceAccessContext {
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope).unwrap();
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        lifecycle.register_active(exact).unwrap();
        ResourceAccessContext::for_account_with_lifecycle(scope, lifecycle)
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

    #[test]
    fn account_context_retains_its_write_scope() {
        let access = active_access(exact_scope("ws-write"));
        assert!(access.write_scope().is_some());
        assert!(access.read_query().is_some());
    }

    #[test]
    fn account_context_retains_workspace_narrowing() {
        let scope = exact_scope("ws-alpha");
        let access = active_access(scope);
        assert_eq!(
            access.read_query().and_then(ResourceScopeQuery::workspace),
            Some(&WorkspaceScopeRef::new("ws-alpha").unwrap())
        );
    }

    #[test]
    fn complete_account_context_reads_with_all_five_exact_dimensions() {
        let scope = ResourceScope::new(owner(), actor())
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(WorkspaceScopeRef::new("ws-exact").unwrap());
        let access = active_access(scope.clone());
        assert_eq!(
            access.exact_read_scope(),
            ExactResourceScopeAttribution::try_from_resource_scope(&scope)
                .ok()
                .as_ref()
        );

        access
            .authorize_row(&StoredResourceScope::from(&scope))
            .expect("the exact stored scope must be readable");
        for denied in [
            ResourceScope::new(scope.owner_account_id, actor())
                .with_session(scope.authenticated_session.unwrap())
                .with_access_space(scope.access_space.unwrap())
                .with_workspace(scope.workspace.clone().unwrap()),
            ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
                .with_session(AuthenticatedSessionRef::mint())
                .with_access_space(scope.access_space.unwrap())
                .with_workspace(scope.workspace.clone().unwrap()),
            ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
                .with_session(scope.authenticated_session.unwrap())
                .with_access_space(AccessSpaceRef::mint())
                .with_workspace(scope.workspace.clone().unwrap()),
        ] {
            assert_eq!(
                access
                    .authorize_row(&StoredResourceScope::from(&denied))
                    .expect_err("one wrong exact dimension must deny")
                    .reason_code(),
                "RESOURCE_SCOPE_EXACT_ATTRIBUTION_MISMATCH"
            );
        }
    }

    #[test]
    fn system_context_has_no_account_write_scope() {
        let access =
            ResourceAccessContext::system(SystemScopeAuthority::legacy_unscoped_call_site());
        assert!(access.write_scope().is_none());
        assert_eq!(
            access.system_authority().map(|a| a.reason()),
            Some("SYSTEM_SCOPE_LEGACY_UNSCOPED_CALL_SITE")
        );
    }

    #[test]
    fn a_reader_context_can_never_write() {
        // API read boundaries carry an owning account but no actor Principal.
        // If they could write, an unauthenticated header would mint ownership.
        let access = ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(owner()));
        assert!(access.write_scope().is_none());
        assert!(access.read_query().is_some());
        access
            .require_active()
            .expect("legacy reader semantics remain available outside protected boundaries");
        assert_eq!(
            access.require_lifecycle_active().unwrap_err().reason_code(),
            "RESOURCE_ACCESS_CONTEXT_UNKNOWN"
        );
    }

    #[test]
    fn an_account_context_exposes_all_exact_dimensions() {
        let scope = ResourceScope::new(owner(), actor())
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(WorkspaceScopeRef::new("ws-alpha").unwrap());
        let access = active_access(scope.clone());
        assert_eq!(
            access.exact_read_scope(),
            ExactResourceScopeAttribution::try_from_resource_scope(&scope)
                .ok()
                .as_ref()
        );
    }

    #[test]
    fn an_unattributed_authority_can_never_authorize_anything() {
        // This is the whole point of the type: the pre-fix cloud consent path
        // recorded a self-minted `operator://<role_label>/...` string next to
        // `approved: true`. Its typed replacement for "no authenticated account"
        // must be structurally incapable of satisfying an account check.
        let authority = AccountBoundAuthority::unattributed("NO_AUTHENTICATED_ACCOUNT");
        assert!(!authority.is_account_bound());
        assert_eq!(authority.owner_account_id(), None);

        let denied = authority
            .authorizes(&ResourceScopeQuery::for_owner(owner()))
            .expect_err("an unattributed approval must never authorize an account-scoped action");
        assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_UNATTRIBUTED");
    }

    #[test]
    fn an_account_authority_authorizes_only_its_own_account() {
        let mine = owner();
        let theirs = owner();
        let authority = AccountBoundAuthority::from_scope(&ResourceScope::new(mine, actor()));

        authority
            .authorizes(&ResourceScopeQuery::for_owner(mine))
            .expect("the approving account must be able to use its own approval");

        let denied = authority
            .authorizes(&ResourceScopeQuery::for_owner(theirs))
            .expect_err("another account must not be able to reuse this approval");
        assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_OWNER_MISMATCH");
    }

    #[test]
    fn a_read_only_or_system_context_cannot_mint_an_account_bound_approval() {
        // A reader carries an owning account but no actor Principal. If it could
        // mint an approval, an unauthenticated header would become consent.
        let reader = ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(owner()));
        assert!(!AccountBoundAuthority::from_access(&reader).is_account_bound());

        let system =
            ResourceAccessContext::system(SystemScopeAuthority::legacy_unscoped_call_site());
        let authority = AccountBoundAuthority::from_access(&system);
        assert!(!authority.is_account_bound());
        assert_eq!(
            authority,
            AccountBoundAuthority::unattributed("SYSTEM_SCOPE_LEGACY_UNSCOPED_CALL_SITE"),
            "the bypass reason must survive into the durable record so it is auditable"
        );

        let account = active_access(exact_scope("ws-authority"));
        assert!(AccountBoundAuthority::from_access(&account).is_account_bound());
    }

    #[test]
    fn a_system_context_authorizes_rows_but_an_account_context_does_not() {
        let theirs = owner();
        let their_row = StoredResourceScope::from(&ResourceScope::new(theirs, actor()));

        ResourceAccessContext::system(SystemScopeAuthority::boot_recovery())
            .authorize_row(&their_row)
            .expect("boot recovery is intentionally cross-owner");

        let denied = ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(owner()))
            .authorize_row(&their_row)
            .expect_err("an account reader must not see another account's row");
        assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_OWNER_MISMATCH");
    }

    #[test]
    fn legacy_account_context_is_explicit_but_cannot_enter_protected_boundaries() {
        let scope = exact_scope("ws-legacy");
        let access = ResourceAccessContext::for_account(scope.clone());
        assert!(access.lifecycle_authority().is_none());
        assert_eq!(access.write_scope(), Some(&scope));
        assert!(access.exact_read_scope().is_some());
        assert_eq!(
            access.require_lifecycle_active().unwrap_err().reason_code(),
            "RESOURCE_ACCESS_CONTEXT_UNKNOWN"
        );
    }

    #[test]
    fn lifecycle_is_shared_immediate_and_terminal_for_one_exact_session() {
        let scope = exact_scope("ws-lifecycle");
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope).unwrap();
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        lifecycle.register_active(exact.clone()).unwrap();
        let first =
            ResourceAccessContext::for_account_with_lifecycle(scope.clone(), lifecycle.clone());
        let second = ResourceAccessContext::for_account_with_lifecycle(scope, lifecycle.clone());
        first.require_active().unwrap();
        second.require_active().unwrap();

        lifecycle.revoke(&exact).unwrap();
        for access in [&first, &second] {
            assert_eq!(
                access.require_active().unwrap_err().reason_code(),
                "RESOURCE_ACCESS_CONTEXT_REVOKED"
            );
            assert!(access.write_scope().is_none());
            assert!(access.exact_read_scope().is_none());
        }
        assert_eq!(
            lifecycle.register_active(exact).unwrap_err(),
            ResourceAccessLifecycleTransitionError::TerminalContext
        );
    }

    #[test]
    fn stale_and_unknown_decisions_are_distinct_and_non_leaking() {
        let scope = exact_scope("ws-stale");
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope).unwrap();
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        assert_eq!(
            lifecycle.authorize(&exact).unwrap_err().reason_code(),
            "RESOURCE_ACCESS_CONTEXT_UNKNOWN"
        );
        lifecycle.register_active(exact.clone()).unwrap();
        lifecycle.mark_stale(&exact).unwrap();
        let denied = lifecycle.authorize(&exact).unwrap_err();
        assert_eq!(denied.reason_code(), "RESOURCE_ACCESS_CONTEXT_STALE");
        assert!(!denied
            .to_string()
            .contains(&exact.authenticated_session_id.to_string()));
    }
}

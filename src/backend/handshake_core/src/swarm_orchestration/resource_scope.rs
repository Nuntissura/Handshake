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
//! `Principal`, `AuthenticatedSession`, PostgreSQL RLS), and its MT-015
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

use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgArguments, postgres::PgRow, query::Query, Postgres, Row};
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
    /// (which would widen), this refuses to derive across mixed owners at all
    /// and forces the caller to handle the mixed-scope case explicitly. That is
    /// the conservative direction: a refusal is recoverable, a silent widening
    /// is a leak.
    pub fn derive_from_sources<'a>(
        sources: impl IntoIterator<Item = &'a ResourceScope>,
        actor_principal_id: ActorPrincipalId,
    ) -> Result<Self, ResourceScopeError> {
        let mut iter = sources.into_iter();
        let first = iter.next().ok_or(ResourceScopeError::NoDerivationSources)?;

        let mut derived = Self::new(first.owner_account_id, actor_principal_id);
        derived.access_space = first.access_space;
        derived.workspace = first.workspace.clone();

        for source in iter {
            if source.owner_account_id != derived.owner_account_id {
                return Err(ResourceScopeError::MixedOwnerDerivation {
                    first: derived.owner_account_id,
                    conflicting: source.owner_account_id,
                });
            }
            // A narrower (absent) value on any source narrows the result: an
            // AccessSpace or workspace that is not common to every source
            // cannot be claimed by the derivative.
            if source.access_space != derived.access_space {
                derived.access_space = None;
            }
            if source.workspace != derived.workspace {
                derived.workspace = None;
            }
        }

        Ok(derived)
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

/// The scope as it was read back out of PostgreSQL. Every field is optional
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
}

// ---------------------------------------------------------------------------
// Store-level access context (write stamping + read enforcement)
// ---------------------------------------------------------------------------

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
    /// to `scope`'s owning account and (when set) its workspace.
    pub fn for_account(scope: ResourceScope) -> Self {
        let mut query = ResourceScopeQuery::for_owner(scope.owner_account_id);
        if let Some(workspace) = scope.workspace.clone() {
            query = query.within_workspace(workspace);
        }
        Self::Account(AccountAccessContext {
            query,
            write: Some(scope),
        })
    }

    /// Read-only context. Used by API read boundaries, which carry an owning
    /// account but no actor Principal, so they must not be able to write.
    pub fn for_reader(query: ResourceScopeQuery) -> Self {
        Self::Account(AccountAccessContext { query, write: None })
    }

    pub fn system(authority: SystemScopeAuthority) -> Self {
        Self::System(authority)
    }

    pub fn read_query(&self) -> Option<&ResourceScopeQuery> {
        match self {
            Self::Account(account) => Some(&account.query),
            Self::System(_) => None,
        }
    }

    pub fn write_scope(&self) -> Option<&ResourceScope> {
        match self {
            Self::Account(account) => account.write.as_ref(),
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

    /// Second enforcement layer. HBR-PRIV-002: hiding a row in one layer is
    /// never sufficient, so every read path calls this on the scope columns it
    /// read back, even though the SQL predicate should already have excluded
    /// the row.
    pub fn authorize_row(&self, row: &StoredResourceScope) -> Result<(), ScopeDenied> {
        match self {
            Self::Account(account) => account.query.authorize_row(row),
            Self::System(_) => Ok(()),
        }
    }

    /// Build the SQL fragment that keeps denied rows inside PostgreSQL.
    ///
    /// `first_placeholder` is the next free `$n` in the statement being built.
    /// The returned clause always starts with ` AND `, so callers append it to a
    /// statement that already has a `WHERE` (use `WHERE TRUE` when there is no
    /// other predicate).
    pub fn sql_predicate(&self, first_placeholder: usize) -> ScopeSqlPredicate {
        match self {
            Self::System(_) => ScopeSqlPredicate {
                clause: String::new(),
                owner: None,
                workspace: None,
            },
            Self::Account(account) => {
                let mut clause =
                    format!(" AND {OWNER_ACCOUNT_COLUMN} = ${first_placeholder}::uuid");
                let workspace = account.query.workspace().map(|ws| ws.as_str().to_owned());
                if workspace.is_some() {
                    clause.push_str(&format!(
                        " AND {WORKSPACE_COLUMN} = ${}",
                        first_placeholder + 1
                    ));
                }
                ScopeSqlPredicate {
                    clause,
                    owner: Some(account.query.owner_account_id().as_uuid()),
                    workspace,
                }
            }
        }
    }

    /// The column values to stamp on an INSERT. A `System` context yields all
    /// NULLs — an unattributed row — which no account-scoped reader can read.
    pub fn insert_columns(&self) -> ScopeColumnValues<'_> {
        ScopeColumnValues::from_scope(self.write_scope())
    }
}

pub const OWNER_ACCOUNT_COLUMN: &str = "owner_account_id";
pub const ACTOR_PRINCIPAL_COLUMN: &str = "actor_principal_id";
pub const AUTHENTICATED_SESSION_COLUMN: &str = "authenticated_session_id";
pub const ACCESS_SPACE_COLUMN: &str = "access_space_id";
pub const WORKSPACE_COLUMN: &str = "workspace_id";

/// The five scope columns migration 0363 adds, in the order
/// [`stored_resource_scope_from_row`] and [`ScopeColumnValues::bind`] expect.
pub const RESOURCE_SCOPE_SELECT_COLUMNS: &str =
    "owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id";

/// The same five columns as an INSERT column list fragment.
pub const RESOURCE_SCOPE_INSERT_COLUMNS: &str = RESOURCE_SCOPE_SELECT_COLUMNS;

/// A rendered owner predicate plus the values that fill its placeholders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeSqlPredicate {
    clause: String,
    owner: Option<Uuid>,
    workspace: Option<String>,
}

impl ScopeSqlPredicate {
    /// `""` for a system context, otherwise ` AND owner_account_id = $n::uuid[ AND workspace_id = $n+1]`.
    pub fn clause(&self) -> &str {
        &self.clause
    }

    /// How many placeholders this predicate consumed.
    pub fn placeholder_count(&self) -> usize {
        usize::from(self.owner.is_some()) + usize::from(self.workspace.is_some())
    }

    /// Bind the predicate values, in the same order the clause references them.
    pub fn bind<'q>(
        &'q self,
        mut query: Query<'q, Postgres, PgArguments>,
    ) -> Query<'q, Postgres, PgArguments> {
        if let Some(owner) = self.owner {
            query = query.bind(owner);
        }
        if let Some(workspace) = self.workspace.as_deref() {
            query = query.bind(workspace);
        }
        query
    }

    /// `query_scalar` variant of [`Self::bind`].
    pub fn bind_scalar<'q, O>(
        &'q self,
        mut query: sqlx::query::QueryScalar<'q, Postgres, O, PgArguments>,
    ) -> sqlx::query::QueryScalar<'q, Postgres, O, PgArguments> {
        if let Some(owner) = self.owner {
            query = query.bind(owner);
        }
        if let Some(workspace) = self.workspace.as_deref() {
            query = query.bind(workspace);
        }
        query
    }
}

/// The five scope column values for one INSERT.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScopeColumnValues<'a> {
    pub owner_account_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub authenticated_session_id: Option<Uuid>,
    pub access_space_id: Option<Uuid>,
    pub workspace_id: Option<&'a str>,
}

impl<'a> ScopeColumnValues<'a> {
    pub fn from_scope(scope: Option<&'a ResourceScope>) -> Self {
        match scope {
            None => Self::default(),
            Some(scope) => Self {
                owner_account_id: Some(scope.owner_account_id.as_uuid()),
                actor_principal_id: Some(scope.actor_principal_id.as_uuid()),
                authenticated_session_id: scope.authenticated_session.map(|s| s.as_uuid()),
                access_space_id: scope.access_space.map(|s| s.as_uuid()),
                workspace_id: scope.workspace.as_ref().map(|w| w.as_str()),
            },
        }
    }

    /// True when this write leaves the row unattributed (NULL owner). Callers
    /// that must not silently drop attribution check this.
    pub const fn is_unattributed(&self) -> bool {
        self.owner_account_id.is_none()
    }

    /// Bind the five values in `RESOURCE_SCOPE_INSERT_COLUMNS` order.
    pub fn bind<'q>(
        self,
        query: Query<'q, Postgres, PgArguments>,
    ) -> Query<'q, Postgres, PgArguments>
    where
        'a: 'q,
    {
        query
            .bind(self.owner_account_id)
            .bind(self.actor_principal_id)
            .bind(self.authenticated_session_id)
            .bind(self.access_space_id)
            .bind(self.workspace_id)
    }
}

/// Read the five scope columns back out of a row so the post-deserialization
/// authorization layer has something to judge.
pub fn stored_resource_scope_from_row(row: &PgRow) -> Result<StoredResourceScope, sqlx::Error> {
    Ok(StoredResourceScope {
        owner_account_id: row
            .try_get::<Option<Uuid>, _>(OWNER_ACCOUNT_COLUMN)?
            .map(OwnerAccountId::from_uuid),
        actor_principal_id: row
            .try_get::<Option<Uuid>, _>(ACTOR_PRINCIPAL_COLUMN)?
            .map(ActorPrincipalId::from_uuid),
        authenticated_session: row
            .try_get::<Option<Uuid>, _>(AUTHENTICATED_SESSION_COLUMN)?
            .map(AuthenticatedSessionRef::from_uuid),
        access_space: row
            .try_get::<Option<Uuid>, _>(ACCESS_SPACE_COLUMN)?
            .map(AccessSpaceRef::from_uuid),
        workspace: row
            .try_get::<Option<String>, _>(WORKSPACE_COLUMN)?
            .and_then(|value| WorkspaceScopeRef::new(value).ok()),
    })
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
        let a = ResourceScope::new(owner(), actor());
        let b = ResourceScope::new(owner(), actor());

        let error = ResourceScope::derive_from_sources([&a, &b], actor())
            .expect_err("a derivative must not span two owning accounts");
        assert!(matches!(error, ResourceScopeError::MixedOwnerDerivation { .. }));
    }

    #[test]
    fn derivation_narrows_to_the_common_workspace_and_never_widens() {
        let mine = owner();
        let alpha = WorkspaceScopeRef::new("ws-alpha").unwrap();
        let beta = WorkspaceScopeRef::new("ws-beta").unwrap();

        let a = ResourceScope::new(mine, actor()).with_workspace(alpha.clone());
        let b = ResourceScope::new(mine, actor()).with_workspace(beta);

        let derived = ResourceScope::derive_from_sources([&a, &b], actor())
            .expect("same-owner derivation is allowed");

        // Sources disagree on workspace, so the derivative claims neither.
        assert_eq!(derived.workspace, None);
        assert_eq!(derived.owner_account_id, mine);

        // And the narrowed derivative is not readable under either workspace.
        let stored = StoredResourceScope::from(&derived);
        ResourceScopeQuery::for_owner(mine)
            .within_workspace(alpha)
            .authorize_row(&stored)
            .expect_err("a narrowed derivative must not be claimable by a source workspace");
    }

    #[test]
    fn derivation_preserves_a_workspace_common_to_all_sources() {
        let mine = owner();
        let alpha = WorkspaceScopeRef::new("ws-alpha").unwrap();

        let a = ResourceScope::new(mine, actor()).with_workspace(alpha.clone());
        let b = ResourceScope::new(mine, actor()).with_workspace(alpha.clone());

        let derived = ResourceScope::derive_from_sources([&a, &b], actor()).unwrap();
        assert_eq!(derived.workspace, Some(alpha));
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
    fn account_context_renders_an_owner_only_predicate() {
        let access = ResourceAccessContext::for_account(ResourceScope::new(owner(), actor()));
        let predicate = access.sql_predicate(2);
        assert_eq!(predicate.clause(), " AND owner_account_id = $2::uuid");
        assert_eq!(predicate.placeholder_count(), 1);
    }

    #[test]
    fn account_context_renders_a_workspace_narrowed_predicate() {
        let scope = ResourceScope::new(owner(), actor())
            .with_workspace(WorkspaceScopeRef::new("ws-alpha").unwrap());
        let access = ResourceAccessContext::for_account(scope);
        let predicate = access.sql_predicate(3);
        assert_eq!(
            predicate.clause(),
            " AND owner_account_id = $3::uuid AND workspace_id = $4"
        );
        assert_eq!(predicate.placeholder_count(), 2);
    }

    #[test]
    fn system_context_renders_no_predicate_and_stamps_nothing() {
        let access =
            ResourceAccessContext::system(SystemScopeAuthority::legacy_unscoped_call_site());
        let predicate = access.sql_predicate(1);
        assert_eq!(predicate.clause(), "");
        assert_eq!(predicate.placeholder_count(), 0);
        assert!(access.insert_columns().is_unattributed());
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
        assert!(access.insert_columns().is_unattributed());
        assert!(access.read_query().is_some());
    }

    #[test]
    fn an_account_context_stamps_every_column_it_carries() {
        let scope = ResourceScope::new(owner(), actor())
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(WorkspaceScopeRef::new("ws-alpha").unwrap());
        let access = ResourceAccessContext::for_account(scope);
        let columns = access.insert_columns();
        assert!(!columns.is_unattributed());
        assert!(columns.actor_principal_id.is_some());
        assert!(columns.authenticated_session_id.is_some());
        assert!(columns.access_space_id.is_some());
        assert_eq!(columns.workspace_id, Some("ws-alpha"));
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

        let account = ResourceAccessContext::for_account(ResourceScope::new(owner(), actor()));
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
}

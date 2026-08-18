//! MT-177 Role Mailbox SurrealDB repository with transactional lifecycle
//! enforcement.
//!
//! # Porting notes (PostgreSQL -> embedded SurrealDB)
//!
//! The original repository leaned on three PostgreSQL constructs. Each one has
//! a direct SurrealDB counterpart and the mapping is deliberate:
//!
//! 1. **`SELECT ... FOR UPDATE` + read-modify-write.** Replaced by a single
//!    guarded statement wherever the guard is expressible as a predicate:
//!    `UPDATE ... WHERE <guard> RETURN AFTER`. SurrealDB reports affected rows
//!    by returning them, so an affected count of zero means the guard did not
//!    hold, which is exactly the "someone else won" signal the row lock used to
//!    provide. The precise typed error (`NotFound` vs `InvalidTransition` vs
//!    `TerminalState`) is then derived from a follow-up read, which is
//!    diagnostic only — correctness never depends on it.
//!
//! 2. **The partial unique index `UNIQUE (thread_id) WHERE released_at_utc IS
//!    NULL`.** Replaced by the stored discriminator field `active_thread_key`
//!    plus `idx_role_mailbox_claim_lease_active UNIQUE` in `schema.surql`. This
//!    is still a *write-time database constraint*, so exactly-one-active-lease
//!    -per-thread is enforced by the store and not by application checks. The
//!    application-level checks below remain, as they did before, only to turn
//!    the common case into a specific error.
//!
//! 3. **Multi-statement transactions.** Written as
//!    `BEGIN TRANSACTION; ...; COMMIT TRANSACTION;` in one round trip. The
//!    response is checked across every statement, so a failure in any statement
//!    (including a unique-index violation on the last one) aborts the whole
//!    block and surfaces as an error here.
//!
//! DISCLOSED NARROWING: PostgreSQL's `FOR UPDATE` pessimistic row lock
//! serialised concurrent callers so that an application-level check could not
//! be raced. The replacements above keep every *correctness* invariant, because
//! each one is now carried by a guard inside the write itself or by a unique
//! index. What can degrade under a genuine race is error *specificity*: two
//! simultaneous acquirers may both pass the advisory check and the loser then
//! sees `LeaseError::Conflict` (from the unique index) where it previously saw
//! `LeaseError::LeaseHeldByOther`. No caller is permitted to treat either as
//! success.

use chrono::{DateTime, Utc};
use serde_json::Value;
use surrealdb::types::{RecordId, SurrealValue};
use thiserror::Error;
use uuid::Uuid;

use super::handoff::{MailboxHandoffBundleV1, TranscriptPointer};
use super::lease::{LeaseError, LeaseRequest, RoleMailboxClaimLeaseV1, TakeoverPolicy};
use super::lifecycle::{
    transition_message_state, transition_thread_state, InvalidTransition, MessageDeliveryState,
    ThreadLifecycleState,
};
use super::message::{MessageType, RoleMailboxMessage, RoleMailboxMessageId};
use super::router::ExecutorKind;
use super::thread::{
    ClaimMode, LinkedRecordKind, ResponseAuthorityScope, RoleMailboxThread, RoleMailboxThreadId,
};
use crate::role_mailbox::RoleId;
use crate::storage::surreal::{SurrealStorage, SurrealStorageError};

const THREAD_TABLE: &str = "role_mailbox_thread";
const MESSAGE_TABLE: &str = "role_mailbox_message";
const LEASE_TABLE: &str = "role_mailbox_claim_lease";
const BUNDLE_TABLE: &str = "role_mailbox_handoff_bundle";

const ALL_THREAD_STATES: [ThreadLifecycleState; 7] = [
    ThreadLifecycleState::Open,
    ThreadLifecycleState::AwaitingResponse,
    ThreadLifecycleState::WaitingOnLinkedAuthority,
    ThreadLifecycleState::Escalated,
    ThreadLifecycleState::Resolved,
    ThreadLifecycleState::Expired,
    ThreadLifecycleState::Archived,
];

const ALL_MESSAGE_STATES: [MessageDeliveryState; 7] = [
    MessageDeliveryState::Queued,
    MessageDeliveryState::Delivered,
    MessageDeliveryState::Acknowledged,
    MessageDeliveryState::Replied,
    MessageDeliveryState::Ignored,
    MessageDeliveryState::Failed,
    MessageDeliveryState::DeadLettered,
];

#[derive(Debug, Error)]
pub enum MailboxError {
    #[error("invalid transition: {0}")]
    InvalidTransition(#[from] InvalidTransition),
    #[error("thread not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("thread in terminal lifecycle state")]
    TerminalState,
    #[error("storage error: {0}")]
    Storage(#[from] SurrealStorageError),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("parse error: {0}")]
    Parse(String),
    /// MT-183: caller-supplied `content_hash` did not match the canonical-JSON
    /// recomputed hash for the handoff bundle. The repo recomputes on insert
    /// and refuses tampered input per `red_team.minimum_controls`.
    #[error("handoff bundle content_hash mismatch (expected {expected}, got {got})")]
    HashMismatch { expected: String, got: String },
}

// ── record shapes ───────────────────────────────────────────────────────────

/// `role_mailbox_thread`. The table asserts `thread_id = record::id($this.id)`,
/// so the record key must be a UUID key, never a string key.
#[derive(Debug, Clone, SurrealValue)]
struct ThreadRow {
    thread_id: Uuid,
    title: String,
    linked_record_kind: String,
    linked_record_id: Option<String>,
    lifecycle_state: String,
    executor_kind_allowlist: Vec<String>,
    claim_mode: String,
    lease_duration_secs: Option<i64>,
    takeover_policy: String,
    response_authority_scope: String,
    created_at_utc: DateTime<Utc>,
    updated_at_utc: DateTime<Utc>,
    expires_at_utc: Option<DateTime<Utc>>,
    archived_at_utc: Option<DateTime<Utc>>,
}

impl TryFrom<ThreadRow> for RoleMailboxThread {
    type Error = MailboxError;

    fn try_from(row: ThreadRow) -> Result<Self, Self::Error> {
        let allowlist = row
            .executor_kind_allowlist
            .iter()
            .map(|raw| {
                parse_executor_kind(raw)
                    .ok_or_else(|| MailboxError::Parse(format!("executor_kind: {raw}")))
            })
            .collect::<Result<Vec<ExecutorKind>, MailboxError>>()?;
        let linked: LinkedRecordKind =
            serde_json::from_value(Value::String(row.linked_record_kind))?;
        let lifecycle: ThreadLifecycleState =
            serde_json::from_value(Value::String(row.lifecycle_state))?;
        let claim_mode: ClaimMode = serde_json::from_value(Value::String(row.claim_mode))?;
        let takeover: TakeoverPolicy = serde_json::from_value(Value::String(row.takeover_policy))?;
        let scope: ResponseAuthorityScope =
            serde_json::from_value(Value::String(row.response_authority_scope))?;
        Ok(Self {
            thread_id: RoleMailboxThreadId(row.thread_id),
            title: row.title,
            linked_record_kind: linked,
            linked_record_id: row.linked_record_id,
            lifecycle_state: lifecycle,
            executor_kind_allowlist: allowlist,
            claim_mode,
            lease_duration_secs: row.lease_duration_secs.map(|value| value as u32),
            takeover_policy: takeover,
            response_authority_scope: scope,
            created_at_utc: row.created_at_utc,
            updated_at_utc: row.updated_at_utc,
            expires_at_utc: row.expires_at_utc,
            archived_at_utc: row.archived_at_utc,
        })
    }
}

/// `role_mailbox_message`. `thread_id` is a `record<role_mailbox_thread>`
/// REFERENCE with `ASSERT record::exists`, so a message can never be written
/// against a thread that does not exist — the foreign key the PostgreSQL
/// version relied on is preserved by the schema rather than by this code.
#[derive(Debug, Clone, SurrealValue)]
struct MessageRow {
    message_id: Uuid,
    thread_id: RecordId,
    message_type: String,
    from_role: String,
    to_roles: Vec<String>,
    delivery_state: String,
    body: Value,
    parent_message_id: Option<Uuid>,
    created_at_utc: DateTime<Utc>,
}

impl TryFrom<MessageRow> for RoleMailboxMessage {
    type Error = MailboxError;

    fn try_from(row: MessageRow) -> Result<Self, Self::Error> {
        let message_type: MessageType = serde_json::from_value(Value::String(row.message_type))?;
        let from_role = RoleId::parse(&row.from_role)
            .map_err(|error| MailboxError::Parse(format!("from_role: {error}")))?;
        let to_roles = row
            .to_roles
            .iter()
            .map(|raw| RoleId::parse(raw))
            .collect::<Result<Vec<RoleId>, _>>()
            .map_err(|error| MailboxError::Parse(format!("to_role: {error}")))?;
        let delivery: MessageDeliveryState =
            serde_json::from_value(Value::String(row.delivery_state))?;
        Ok(Self {
            message_id: RoleMailboxMessageId(row.message_id),
            thread_id: RoleMailboxThreadId(record_key_uuid(&row.thread_id)?),
            message_type,
            from_role,
            to_roles,
            expected_response: None,
            expires_at_utc: None,
            delivery_state: delivery,
            body: row.body,
            parent_message_id: row.parent_message_id.map(RoleMailboxMessageId),
            created_at_utc: row.created_at_utc,
        })
    }
}

/// `role_mailbox_claim_lease`. `active_thread_key` carries a `VALUE` clause and
/// is computed by the store, so it is deliberately absent here — writing it
/// would fight the schema.
#[derive(Debug, Clone, SurrealValue)]
struct LeaseRow {
    lease_id: Uuid,
    thread_id: RecordId,
    holder_executor_kind: String,
    holder_role_id: String,
    holder_session_id: Uuid,
    acquired_at_utc: DateTime<Utc>,
    expires_at_utc: DateTime<Utc>,
    released_at_utc: Option<DateTime<Utc>>,
    takeover_of: Option<RecordId>,
    takeover_reason: Option<String>,
}

impl TryFrom<LeaseRow> for RoleMailboxClaimLeaseV1 {
    type Error = LeaseError;

    fn try_from(row: LeaseRow) -> Result<Self, Self::Error> {
        let holder_executor_kind =
            parse_executor_kind(&row.holder_executor_kind).ok_or(LeaseError::Conflict)?;
        let holder_role_id = RoleId::parse(&row.holder_role_id).map_err(|_| LeaseError::Conflict)?;
        let takeover_of = match &row.takeover_of {
            Some(record) => Some(record_key_uuid(record).map_err(|_| LeaseError::Conflict)?),
            None => None,
        };
        Ok(Self {
            lease_id: row.lease_id,
            thread_id: record_key_uuid(&row.thread_id).map_err(|_| LeaseError::Conflict)?,
            holder_executor_kind,
            holder_role_id,
            holder_session_id: row.holder_session_id,
            acquired_at_utc: row.acquired_at_utc,
            expires_at_utc: row.expires_at_utc,
            released_at_utc: row.released_at_utc,
            takeover_of,
            takeover_reason: row.takeover_reason,
        })
    }
}

/// `role_mailbox_handoff_bundle`.
#[derive(Debug, Clone, SurrealValue)]
struct BundleRow {
    bundle_id: Uuid,
    source_thread_id: RecordId,
    source_message_id: Uuid,
    target_role: String,
    target_executor_kind: String,
    context_summary: String,
    linked_artifacts: Vec<Value>,
    transcript_pointer: Option<Value>,
    capability_grants: Vec<Value>,
    expires_at_utc: Option<DateTime<Utc>>,
    content_hash: String,
    created_at_utc: DateTime<Utc>,
    created_by_session: Uuid,
}

impl TryFrom<BundleRow> for MailboxHandoffBundleV1 {
    type Error = MailboxError;

    fn try_from(row: BundleRow) -> Result<Self, Self::Error> {
        let target_role = RoleId::parse(&row.target_role)
            .map_err(|error| MailboxError::Parse(format!("target_role: {error}")))?;
        let target_executor_kind = parse_executor_kind(&row.target_executor_kind).ok_or_else(|| {
            MailboxError::Parse(format!("target_executor_kind: {}", row.target_executor_kind))
        })?;
        let linked_artifacts = serde_json::from_value(Value::Array(row.linked_artifacts))?;
        let transcript_pointer = match row.transcript_pointer {
            Some(value) => Some(serde_json::from_value::<TranscriptPointer>(value)?),
            None => None,
        };
        let capability_grants = serde_json::from_value(Value::Array(row.capability_grants))?;
        Ok(Self {
            bundle_id: row.bundle_id,
            source_thread_id: record_key_uuid(&row.source_thread_id)?,
            source_message_id: row.source_message_id,
            target_role,
            target_executor_kind,
            context_summary: row.context_summary,
            linked_artifacts,
            transcript_pointer,
            capability_grants,
            expires_at_utc: row.expires_at_utc,
            content_hash: row.content_hash,
            created_at_utc: row.created_at_utc,
            created_by_session: row.created_by_session,
        })
    }
}

// ── bindings ────────────────────────────────────────────────────────────────

/// `CREATE $record CONTENT $content` bindings.
///
/// `content` is a pre-converted [`surrealdb::types::Value`] rather than a
/// generic parameter: the `SurrealValue` derive expands to unqualified trait
/// references and a generic field would need its own bound threaded through it,
/// so the seam's own `create_if_absent` uses the same shape.
#[derive(SurrealValue)]
struct CreateBindings {
    record: RecordId,
    content: surrealdb::types::Value,
}

impl CreateBindings {
    fn new<C: SurrealValue>(record: RecordId, content: C) -> Self {
        Self {
            record,
            content: content.into_value(),
        }
    }
}

#[derive(SurrealValue)]
struct ThreadIdBinding {
    thread_id: Uuid,
}

#[derive(SurrealValue)]
struct ThreadRecordBinding {
    thread: RecordId,
}

#[derive(SurrealValue)]
struct ThreadTransitionBindings {
    thread_id: Uuid,
    next: String,
    allowed_from: Vec<String>,
    now: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct MessageCreateBindings {
    record: RecordId,
    thread_id: Uuid,
    open_states: Vec<String>,
    message_id: Uuid,
    message_type: String,
    from_role: String,
    to_roles: Vec<String>,
    delivery_state: String,
    body: Value,
    created_at_utc: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct MessageTransitionBindings {
    message_id: Uuid,
    next: String,
    allowed_from: Vec<String>,
    reason: String,
}

#[derive(SurrealValue)]
struct MessageIdBinding {
    message_id: Uuid,
}

#[derive(SurrealValue)]
struct ThreadPageBindings {
    state: String,
    limit: i64,
    start: i64,
}

#[derive(SurrealValue)]
struct PendingCountBindings {
    role: String,
    states: Vec<String>,
}

#[derive(SurrealValue)]
struct CountRow {
    total: i64,
}

#[derive(SurrealValue)]
struct SweepBindings {
    thread: RecordId,
    now: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct ActiveLeaseBindings {
    thread: RecordId,
    now: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct LeaseIdBinding {
    lease_id: Uuid,
}

#[derive(SurrealValue)]
struct ExtendLeaseBindings {
    lease_id: Uuid,
    current_expires: DateTime<Utc>,
    new_expires: DateTime<Utc>,
    now: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct ReleaseLeaseBindings {
    lease_id: Uuid,
    now: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct PredecessorBindings {
    lease_id: Uuid,
    thread: RecordId,
}

#[derive(SurrealValue)]
struct TakeoverBindings {
    predecessor: Uuid,
    now: DateTime<Utc>,
    record: RecordId,
    content: LeaseRow,
}

#[derive(SurrealValue)]
struct BundleIdBinding {
    bundle_id: Uuid,
}

// ── statements ──────────────────────────────────────────────────────────────

const CREATE_THREAD_QUERY: &str = "CREATE $record CONTENT $content RETURN AFTER;";

const GET_THREAD_QUERY: &str = "SELECT thread_id, title, linked_record_kind, linked_record_id, \
     lifecycle_state, executor_kind_allowlist, claim_mode, lease_duration_secs, takeover_policy, \
     response_authority_scope, created_at_utc, updated_at_utc, expires_at_utc, archived_at_utc \
     FROM role_mailbox_thread WHERE thread_id = $thread_id;";

/// Conditional lifecycle transition.
///
/// `$allowed_from` is derived from [`transition_thread_state`] itself (see
/// [`allowed_thread_from_states`]), so the transition matrix is never
/// duplicated here — the guard and the Rust state machine cannot drift apart.
/// A zero affected count means the row's state was not in the allowed set,
/// which is the exactly-one-winner signal `FOR UPDATE` used to provide.
const TRANSITION_THREAD_QUERY: &str = "UPDATE role_mailbox_thread SET \
     lifecycle_state = $next, updated_at_utc = $now \
     WHERE thread_id = $thread_id AND $allowed_from CONTAINS lifecycle_state RETURN AFTER;";

/// Append a message, guarded on the thread being non-terminal.
///
/// The guard lives INSIDE the write: `thread_id` resolves through a filtered
/// sub-select, so a terminal (or missing) thread yields `NONE` for a field the
/// schema types as a non-optional `record<role_mailbox_thread>` and the CREATE
/// fails. Nothing is inserted against a thread that closed concurrently.
const APPEND_MESSAGE_QUERY: &str = "CREATE $record CONTENT { \
     message_id: $message_id, \
     thread_id: (SELECT VALUE id FROM role_mailbox_thread \
       WHERE thread_id = $thread_id AND $open_states CONTAINS lifecycle_state LIMIT 1)[0], \
     message_type: $message_type, \
     from_role: $from_role, \
     to_roles: $to_roles, \
     delivery_state: $delivery_state, \
     body: $body, \
     created_at_utc: $created_at_utc \
     } RETURN AFTER;";

const LIST_THREAD_MESSAGES_QUERY: &str = "SELECT message_id, thread_id, message_type, from_role, \
     to_roles, delivery_state, body, parent_message_id, created_at_utc \
     FROM role_mailbox_message WHERE thread_id = $thread \
     ORDER BY created_at_utc ASC, message_id ASC;";

const LIST_THREADS_BY_STATE_QUERY: &str = "SELECT VALUE thread_id FROM role_mailbox_thread \
     WHERE lifecycle_state = $state ORDER BY updated_at_utc DESC LIMIT $limit START $start;";

const DEAD_LETTER_MESSAGE_QUERY: &str = "UPDATE role_mailbox_message SET \
     delivery_state = $next, audit_reason = $reason \
     WHERE message_id = $message_id AND $allowed_from CONTAINS delivery_state RETURN AFTER;";

const MESSAGE_DELIVERY_STATE_QUERY: &str =
    "SELECT VALUE delivery_state FROM role_mailbox_message WHERE message_id = $message_id;";

/// `to_roles @> to_jsonb(ARRAY[$1])` becomes `to_roles CONTAINS $role`: both ask
/// whether the role appears in the recipient array.
const COUNT_PENDING_MESSAGES_QUERY: &str = "SELECT count() AS total FROM role_mailbox_message \
     WHERE $states CONTAINS delivery_state AND to_roles CONTAINS $role GROUP ALL;";

/// Mark expired-but-unreleased leases released so the `active_thread_key`
/// unique index admits a new lease. Idempotent.
const SWEEP_EXPIRED_LEASES_QUERY: &str = "UPDATE role_mailbox_claim_lease SET \
     released_at_utc = $now \
     WHERE thread_id = $thread AND released_at_utc = NONE AND expires_at_utc <= $now \
     RETURN AFTER;";

const ACTIVE_LEASE_HOLDER_QUERY: &str = "SELECT VALUE holder_session_id \
     FROM role_mailbox_claim_lease \
     WHERE thread_id = $thread AND released_at_utc = NONE AND expires_at_utc > $now LIMIT 1;";

const CREATE_LEASE_QUERY: &str = "CREATE $record CONTENT $content RETURN AFTER;";

const GET_LEASE_QUERY: &str = "SELECT lease_id, thread_id, holder_executor_kind, holder_role_id, \
     holder_session_id, acquired_at_utc, expires_at_utc, released_at_utc, takeover_of, \
     takeover_reason FROM role_mailbox_claim_lease WHERE lease_id = $lease_id;";

/// Compare-and-swap extension.
///
/// The predicate carries every rule the PostgreSQL transaction enforced after
/// its `FOR UPDATE`: the lease must be unreleased, must not already have
/// expired, and must still hold the exact `expires_at_utc` the caller read.
/// The last clause is what makes the update atomic without a row lock — a
/// concurrent extension or release changes `expires_at_utc` or
/// `released_at_utc` and this statement then affects zero rows.
const EXTEND_LEASE_QUERY: &str = "UPDATE role_mailbox_claim_lease SET expires_at_utc = $new_expires \
     WHERE lease_id = $lease_id AND released_at_utc = NONE \
     AND expires_at_utc = $current_expires AND expires_at_utc > $now RETURN AFTER;";

/// Idempotent release. An already-released lease keeps its original timestamp
/// instead of being pushed forward, so a repeated release is a true no-op.
const RELEASE_LEASE_QUERY: &str = "UPDATE role_mailbox_claim_lease SET \
     released_at_utc = IF released_at_utc = NONE { $now } ELSE { released_at_utc } \
     WHERE lease_id = $lease_id RETURN AFTER;";

const PREDECESSOR_LEASE_QUERY: &str = "SELECT expires_at_utc, released_at_utc \
     FROM role_mailbox_claim_lease WHERE lease_id = $lease_id AND thread_id = $thread;";

/// Force-release the predecessor and write the successor in ONE transaction.
///
/// Both statements commit together, so the window in which a thread has a
/// released predecessor and no successor does not exist. If the CREATE trips
/// `idx_role_mailbox_claim_lease_active`, the response check fails and the
/// release is rolled back with it.
const TAKEOVER_LEASE_QUERY: &str = "BEGIN TRANSACTION; \
     UPDATE role_mailbox_claim_lease SET \
       released_at_utc = IF released_at_utc = NONE { $now } ELSE { released_at_utc } \
       WHERE lease_id = $predecessor RETURN AFTER; \
     CREATE $record CONTENT $content RETURN AFTER; \
     COMMIT TRANSACTION;";

const CREATE_BUNDLE_QUERY: &str = "CREATE $record CONTENT $content RETURN AFTER;";

const GET_BUNDLE_QUERY: &str = "SELECT bundle_id, source_thread_id, source_message_id, \
     target_role, target_executor_kind, context_summary, linked_artifacts, transcript_pointer, \
     capability_grants, expires_at_utc, content_hash, created_at_utc, created_by_session \
     FROM role_mailbox_handoff_bundle WHERE bundle_id = $bundle_id;";

const LIST_BUNDLES_FOR_THREAD_QUERY: &str = "SELECT bundle_id, source_thread_id, \
     source_message_id, target_role, target_executor_kind, context_summary, linked_artifacts, \
     transcript_pointer, capability_grants, expires_at_utc, content_hash, created_at_utc, \
     created_by_session FROM role_mailbox_handoff_bundle WHERE source_thread_id = $thread \
     ORDER BY created_at_utc ASC, bundle_id ASC;";

const LIST_LEASES_FOR_THREAD_QUERY: &str = "SELECT lease_id, thread_id, holder_executor_kind, \
     holder_role_id, holder_session_id, acquired_at_utc, expires_at_utc, released_at_utc, \
     takeover_of, takeover_reason FROM role_mailbox_claim_lease WHERE thread_id = $thread \
     ORDER BY acquired_at_utc ASC, lease_id ASC;";

// ── repository ──────────────────────────────────────────────────────────────

/// SurrealDB-backed transactional repository.
///
/// CX-503R / operator directive: the only storage binding is the Handshake-
/// managed embedded SurrealDB store. No SQL pool type is reachable from here.
pub struct RoleMailboxRepository {
    storage: SurrealStorage,
}

impl RoleMailboxRepository {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    async fn query<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<Vec<R>, SurrealStorageError>
    where
        R: SurrealValue + Send + 'static,
        B: SurrealValue + Send + 'static,
    {
        self.storage
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_values(statement, bindings).await })
            })
            .await
    }

    async fn execute<B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<usize, SurrealStorageError>
    where
        B: SurrealValue + Send + 'static,
    {
        self.storage
            .with_data_operation(move |database| {
                Box::pin(async move { database.execute_returning(statement, bindings).await })
            })
            .await
    }

    pub async fn create_thread(
        &self,
        thread: RoleMailboxThread,
    ) -> Result<RoleMailboxThread, MailboxError> {
        let content = ThreadRow {
            thread_id: thread.thread_id.as_uuid(),
            title: thread.title.clone(),
            linked_record_kind: encode_enum(&thread.linked_record_kind, "linked")?,
            linked_record_id: thread.linked_record_id.clone(),
            lifecycle_state: thread.lifecycle_state.as_str().to_string(),
            executor_kind_allowlist: thread
                .executor_kind_allowlist
                .iter()
                .map(|kind| executor_kind_str(*kind).to_string())
                .collect(),
            claim_mode: encode_enum(&thread.claim_mode, "claim_mode")?,
            lease_duration_secs: thread.lease_duration_secs.map(|value| value as i64),
            takeover_policy: encode_enum(&thread.takeover_policy, "takeover")?,
            response_authority_scope: encode_enum(&thread.response_authority_scope, "scope")?,
            created_at_utc: thread.created_at_utc,
            updated_at_utc: thread.updated_at_utc,
            expires_at_utc: thread.expires_at_utc,
            archived_at_utc: thread.archived_at_utc,
        };
        // CREATE (not UPSERT) so a duplicate thread id fails exactly as the
        // original INSERT did instead of silently overwriting a live thread.
        let _: Vec<ThreadRow> = self
            .query(
                CREATE_THREAD_QUERY,
                CreateBindings::new(thread_record(thread.thread_id.as_uuid()), content),
            )
            .await?;
        Ok(thread)
    }

    pub async fn get_thread(
        &self,
        thread_id: RoleMailboxThreadId,
    ) -> Result<Option<RoleMailboxThread>, MailboxError> {
        let rows: Vec<ThreadRow> = self
            .query(
                GET_THREAD_QUERY,
                ThreadIdBinding {
                    thread_id: thread_id.as_uuid(),
                },
            )
            .await?;
        rows.into_iter().next().map(TryInto::try_into).transpose()
    }

    /// Transition `thread_id`'s lifecycle to `requested`. Concurrent callers see
    /// exactly-one-winner semantics: the guard is evaluated inside the update,
    /// so the loser affects zero rows.
    pub async fn update_thread_lifecycle(
        &self,
        thread_id: RoleMailboxThreadId,
        requested: ThreadLifecycleState,
    ) -> Result<RoleMailboxThread, MailboxError> {
        let updated: Vec<ThreadRow> = self
            .query(
                TRANSITION_THREAD_QUERY,
                ThreadTransitionBindings {
                    thread_id: thread_id.as_uuid(),
                    next: requested.as_str().to_string(),
                    allowed_from: allowed_thread_from_states(requested),
                    now: Utc::now(),
                },
            )
            .await?;
        if let Some(row) = updated.into_iter().next() {
            return row.try_into();
        }
        // The write did not apply. Re-read purely to produce the precise typed
        // error the caller expects; the guard above already decided the outcome.
        let Some(current) = self.get_thread(thread_id).await? else {
            return Err(MailboxError::NotFound);
        };
        transition_thread_state(current.lifecycle_state, requested)?;
        // The re-read state WOULD be a legal source, so the row moved between
        // the guarded update and this read: another writer won the race.
        Err(MailboxError::Conflict)
    }

    /// Append a new message. Rejects with `TerminalState` if the thread is
    /// resolved/expired/archived.
    pub async fn append_message(
        &self,
        thread_id: RoleMailboxThreadId,
        message_type: MessageType,
        from_role: RoleId,
        to_roles: Vec<RoleId>,
        body: Value,
    ) -> Result<RoleMailboxMessage, MailboxError> {
        let msg_id = RoleMailboxMessageId::new_v7();
        let now = Utc::now();
        let created: Result<Vec<MessageRow>, SurrealStorageError> = self
            .query(
                APPEND_MESSAGE_QUERY,
                MessageCreateBindings {
                    record: message_record(msg_id.as_uuid()),
                    thread_id: thread_id.as_uuid(),
                    open_states: non_terminal_thread_states(),
                    message_id: msg_id.as_uuid(),
                    message_type: message_type.as_str().to_string(),
                    from_role: from_role.to_string(),
                    to_roles: to_roles.iter().map(ToString::to_string).collect(),
                    delivery_state: MessageDeliveryState::Queued.as_str().to_string(),
                    body: body.clone(),
                    created_at_utc: now,
                },
            )
            .await;
        if let Err(error) = created {
            // The guarded CREATE refused. Distinguish "no such thread" from
            // "thread already terminal" for the caller; a storage failure that
            // is neither is propagated unchanged.
            return Err(match self.get_thread(thread_id).await? {
                None => MailboxError::NotFound,
                Some(thread) if thread.lifecycle_state.is_terminal() => MailboxError::TerminalState,
                Some(_) => MailboxError::Storage(error),
            });
        }
        Ok(RoleMailboxMessage {
            message_id: msg_id,
            thread_id,
            message_type,
            from_role,
            to_roles,
            expected_response: None,
            expires_at_utc: None,
            delivery_state: MessageDeliveryState::Queued,
            body,
            parent_message_id: None,
            created_at_utc: now,
        })
    }

    /// List messages for a thread, chronological order.
    pub async fn list_thread_messages(
        &self,
        thread_id: RoleMailboxThreadId,
    ) -> Result<Vec<RoleMailboxMessage>, MailboxError> {
        let rows: Vec<MessageRow> = self
            .query(
                LIST_THREAD_MESSAGES_QUERY,
                ThreadRecordBinding {
                    thread: thread_record(thread_id.as_uuid()),
                },
            )
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn list_threads_by_state(
        &self,
        state: ThreadLifecycleState,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RoleMailboxThread>, MailboxError> {
        let ids: Vec<Uuid> = self
            .query(
                LIST_THREADS_BY_STATE_QUERY,
                ThreadPageBindings {
                    state: state.as_str().to_string(),
                    limit,
                    start: offset,
                },
            )
            .await?;
        let mut threads = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(thread) = self.get_thread(RoleMailboxThreadId(id)).await? {
                threads.push(thread);
            }
        }
        Ok(threads)
    }

    /// Dead-letter a message: transition delivery_state to DeadLettered with
    /// `reason` recorded in `audit_reason`.
    pub async fn dead_letter_message(
        &self,
        message_id: RoleMailboxMessageId,
        reason: String,
    ) -> Result<(), MailboxError> {
        let affected = self
            .execute(
                DEAD_LETTER_MESSAGE_QUERY,
                MessageTransitionBindings {
                    message_id: message_id.as_uuid(),
                    next: MessageDeliveryState::DeadLettered.as_str().to_string(),
                    allowed_from: allowed_message_from_states(
                        MessageDeliveryState::DeadLettered,
                    ),
                    reason,
                },
            )
            .await?;
        if affected > 0 {
            return Ok(());
        }
        let states: Vec<String> = self
            .query(
                MESSAGE_DELIVERY_STATE_QUERY,
                MessageIdBinding {
                    message_id: message_id.as_uuid(),
                },
            )
            .await?;
        let Some(current_str) = states.into_iter().next() else {
            return Err(MailboxError::NotFound);
        };
        let current: MessageDeliveryState = serde_json::from_value(Value::String(current_str))?;
        transition_message_state(current, MessageDeliveryState::DeadLettered)?;
        Err(MailboxError::Conflict)
    }

    /// Count pending (queued / delivered) messages for `to_role` — used by
    /// MT-182 backpressure inbox-cap check.
    pub async fn count_pending_messages_for_role(
        &self,
        role: &RoleId,
    ) -> Result<u32, MailboxError> {
        let rows: Vec<CountRow> = self
            .query(
                COUNT_PENDING_MESSAGES_QUERY,
                PendingCountBindings {
                    role: role.to_string(),
                    states: vec![
                        MessageDeliveryState::Queued.as_str().to_string(),
                        MessageDeliveryState::Delivered.as_str().to_string(),
                    ],
                },
            )
            .await?;
        // `GROUP ALL` yields no row when nothing matches, which is a count of 0.
        Ok(rows.into_iter().next().map_or(0, |row| row.total as u32))
    }

    // ---------- MT-180 Lease primitive ----------

    /// MT-180: Acquire a `RoleMailboxClaimLeaseV1` for `thread_id`.
    ///
    /// Per spec v02.186 §02-system-architecture.md role mailbox subsection
    /// [ADD v02.176]:
    ///   1. Read the thread and verify `claim_mode` allows
    ///      `request.executor_kind` via the `executor_kind_allowlist`.
    ///   2. Reject if the thread is in a terminal lifecycle state.
    ///   3. Sweep expired-but-unreleased leases by marking them released. This
    ///      keeps the `active_thread_key` unique index admitting the new row.
    ///   4. If an unexpired non-released lease exists, return
    ///      `LeaseError::LeaseHeldByOther` (except for `ClaimMode::Open`).
    ///      For `ClaimMode::Handoff`, callers should use [`Self::takeover_lease`]
    ///      with the explicit predecessor lease id.
    ///   5. CREATE the new lease row. `idx_role_mailbox_claim_lease_active`
    ///      enforces exactly-one-active-lease-per-thread at the database level
    ///      (spec line 6156), so a concurrent caller that bypasses step 4 —
    ///      including one going straight to the store — is rejected by the
    ///      index and surfaces here as `LeaseError::Conflict`.
    pub async fn acquire_lease(
        &self,
        thread_id: RoleMailboxThreadId,
        request: LeaseRequest,
    ) -> Result<RoleMailboxClaimLeaseV1, LeaseError> {
        let thread = self
            .get_thread(thread_id)
            .await
            .map_err(|_| LeaseError::Conflict)?
            .ok_or(LeaseError::NotFound)?;
        if thread.lifecycle_state.is_terminal() {
            return Err(LeaseError::ThreadInTerminalState);
        }
        if !thread
            .executor_kind_allowlist
            .contains(&request.executor_kind)
        {
            return Err(LeaseError::ExecutorKindNotAllowed);
        }
        let now = Utc::now();
        self.execute(
            SWEEP_EXPIRED_LEASES_QUERY,
            SweepBindings {
                thread: thread_record(thread_id.as_uuid()),
                now,
            },
        )
        .await
        .map_err(|_| LeaseError::Conflict)?;
        if !matches!(thread.claim_mode, ClaimMode::Open) {
            let holders: Vec<Uuid> = self
                .query(
                    ACTIVE_LEASE_HOLDER_QUERY,
                    ActiveLeaseBindings {
                        thread: thread_record(thread_id.as_uuid()),
                        now,
                    },
                )
                .await
                .map_err(|_| LeaseError::Conflict)?;
            if let Some(current_holder) = holders.into_iter().next() {
                return Err(LeaseError::LeaseHeldByOther { current_holder });
            }
        }
        let lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id: thread_id.as_uuid(),
            holder_executor_kind: request.executor_kind,
            holder_role_id: request.role_id.clone(),
            holder_session_id: request.session_id,
            acquired_at_utc: now,
            expires_at_utc: now + chrono::Duration::seconds(request.lease_duration_secs as i64),
            released_at_utc: None,
            takeover_of: None,
            takeover_reason: None,
        };
        let _: Vec<LeaseRow> = self
            .query(
                CREATE_LEASE_QUERY,
                CreateBindings::new(lease_record(lease.lease_id), lease_row(&lease)),
            )
            .await
            .map_err(|_| LeaseError::Conflict)?;
        Ok(lease)
    }

    /// MT-180: Extend a lease by `extra_secs`. Idempotent in the sense that
    /// calling extend after a successful release returns `AlreadyReleased`
    /// and never silently extends a dead lease.
    ///
    /// Per `red_team.minimum_controls` #3: extension cannot bypass expiry.
    /// If the current `expires_at_utc <= now()` the lease is dead and the
    /// caller must `acquire` afresh.
    pub async fn extend_lease(
        &self,
        lease_id: Uuid,
        extra_secs: u32,
    ) -> Result<RoleMailboxClaimLeaseV1, LeaseError> {
        let current = self
            .load_lease(lease_id)
            .await?
            .ok_or(LeaseError::NotFound)?;
        if current.released_at_utc.is_some() {
            return Err(LeaseError::AlreadyReleased);
        }
        let now = Utc::now();
        if current.expires_at_utc <= now {
            return Err(LeaseError::Expired);
        }
        let new_expires =
            current.expires_at_utc + chrono::Duration::seconds(extra_secs as i64);
        let updated: Vec<LeaseRow> = self
            .query(
                EXTEND_LEASE_QUERY,
                ExtendLeaseBindings {
                    lease_id,
                    current_expires: current.expires_at_utc,
                    new_expires,
                    now,
                },
            )
            .await
            .map_err(|_| LeaseError::Conflict)?;
        let Some(row) = updated.into_iter().next() else {
            // The compare-and-swap lost: the lease was released, expired or
            // extended by someone else between the read and the write.
            let latest = self.load_lease(lease_id).await?.ok_or(LeaseError::NotFound)?;
            if latest.released_at_utc.is_some() {
                return Err(LeaseError::AlreadyReleased);
            }
            if latest.expires_at_utc <= Utc::now() {
                return Err(LeaseError::Expired);
            }
            return Err(LeaseError::Conflict);
        };
        row.try_into()
    }

    /// MT-180: Release a lease. Idempotent — releasing an already-released
    /// lease is a no-op that returns `Ok(())` (mirrors the in-process
    /// LeaseManager's contract and the spec's "release() ... is a no-op if
    /// already released").
    pub async fn release_lease(&self, lease_id: Uuid) -> Result<(), LeaseError> {
        let affected = self
            .execute(
                RELEASE_LEASE_QUERY,
                ReleaseLeaseBindings {
                    lease_id,
                    now: Utc::now(),
                },
            )
            .await
            .map_err(|_| LeaseError::Conflict)?;
        if affected == 0 {
            return Err(LeaseError::NotFound);
        }
        Ok(())
    }

    /// MT-180: Take over a thread's lease per the thread's `takeover_policy`.
    /// Atomically force-releases the predecessor and writes a new lease row
    /// with `takeover_of` and `takeover_reason` populated.
    pub async fn takeover_lease(
        &self,
        thread_id: RoleMailboxThreadId,
        request: LeaseRequest,
        predecessor_lease_id: Uuid,
        reason: String,
    ) -> Result<RoleMailboxClaimLeaseV1, LeaseError> {
        let thread = self
            .get_thread(thread_id)
            .await
            .map_err(|_| LeaseError::Conflict)?
            .ok_or(LeaseError::NotFound)?;
        if thread.lifecycle_state.is_terminal() {
            return Err(LeaseError::ThreadInTerminalState);
        }
        if !thread
            .executor_kind_allowlist
            .contains(&request.executor_kind)
        {
            return Err(LeaseError::ExecutorKindNotAllowed);
        }
        if matches!(thread.takeover_policy, TakeoverPolicy::Never) {
            return Err(LeaseError::TakeoverNotPermitted);
        }
        if matches!(thread.takeover_policy, TakeoverPolicy::OperatorOnly)
            && request.executor_kind != ExecutorKind::Operator
        {
            return Err(LeaseError::TakeoverNotPermitted);
        }
        let predecessor: Vec<PredecessorRow> = self
            .query(
                PREDECESSOR_LEASE_QUERY,
                PredecessorBindings {
                    lease_id: predecessor_lease_id,
                    thread: thread_record(thread_id.as_uuid()),
                },
            )
            .await
            .map_err(|_| LeaseError::Conflict)?;
        let Some(predecessor) = predecessor.into_iter().next() else {
            return Err(LeaseError::NotFound);
        };
        let now = Utc::now();
        if matches!(thread.takeover_policy, TakeoverPolicy::OnLeaseExpiry)
            && predecessor.expires_at_utc > now
        {
            return Err(LeaseError::TakeoverNotPermitted);
        }
        let lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id: thread_id.as_uuid(),
            holder_executor_kind: request.executor_kind,
            holder_role_id: request.role_id.clone(),
            holder_session_id: request.session_id,
            acquired_at_utc: now,
            expires_at_utc: now + chrono::Duration::seconds(request.lease_duration_secs as i64),
            released_at_utc: None,
            takeover_of: Some(predecessor_lease_id),
            takeover_reason: Some(reason),
        };
        self.execute(
            TAKEOVER_LEASE_QUERY,
            TakeoverBindings {
                predecessor: predecessor_lease_id,
                now,
                record: lease_record(lease.lease_id),
                content: lease_row(&lease),
            },
        )
        .await
        .map_err(|_| LeaseError::Conflict)?;
        Ok(lease)
    }

    async fn load_lease(&self, lease_id: Uuid) -> Result<Option<LeaseRow>, LeaseError> {
        let rows: Vec<LeaseRow> = self
            .query(GET_LEASE_QUERY, LeaseIdBinding { lease_id })
            .await
            .map_err(|_| LeaseError::Conflict)?;
        Ok(rows.into_iter().next())
    }

    /// MT-180: Look up the currently-active lease for `thread_id`. Returns
    /// `None` if no unreleased unexpired lease exists. Useful for routing
    /// decisions (MT-181 ExecutorRouter).
    pub async fn get_active_lease_for_thread(
        &self,
        thread_id: RoleMailboxThreadId,
    ) -> Result<Option<RoleMailboxClaimLeaseV1>, LeaseError> {
        let now = Utc::now();
        let holders: Vec<Uuid> = self
            .query(
                ACTIVE_LEASE_HOLDER_QUERY,
                ActiveLeaseBindings {
                    thread: thread_record(thread_id.as_uuid()),
                    now,
                },
            )
            .await
            .map_err(|_| LeaseError::Conflict)?;
        if holders.is_empty() {
            return Ok(None);
        }
        let rows: Vec<LeaseRow> = self
            .query(
                LIST_LEASES_FOR_THREAD_QUERY,
                ThreadRecordBinding {
                    thread: thread_record(thread_id.as_uuid()),
                },
            )
            .await
            .map_err(|_| LeaseError::Conflict)?;
        rows.into_iter()
            .find(|row| row.released_at_utc.is_none() && row.expires_at_utc > now)
            .map(TryInto::try_into)
            .transpose()
    }

    /// MT-180: Return the full ancestry chain of leases for `thread_id`
    /// ordered chronologically. Satisfies `red_team.minimum_controls` #2:
    /// the takeover audit chain must be queryable so an auditor can
    /// reconstruct the full ownership history.
    ///
    /// The PostgreSQL version expressed this as a `WITH RECURSIVE` walk from
    /// the roots (`takeover_of IS NULL`) along `takeover_of`, restricted to the
    /// thread. SurrealDB has no recursive CTE, so the same walk runs here over
    /// ONE consistent read of the thread's leases. The reachability filter is
    /// preserved deliberately rather than flattened to "all rows for the
    /// thread": a lease whose `takeover_of` points outside the chain is
    /// excluded exactly as the CTE excluded it.
    pub async fn list_lease_chain_for_thread(
        &self,
        thread_id: RoleMailboxThreadId,
    ) -> Result<Vec<RoleMailboxClaimLeaseV1>, LeaseError> {
        let rows: Vec<LeaseRow> = self
            .query(
                LIST_LEASES_FOR_THREAD_QUERY,
                ThreadRecordBinding {
                    thread: thread_record(thread_id.as_uuid()),
                },
            )
            .await
            .map_err(|_| LeaseError::Conflict)?;
        let mut leases = Vec::with_capacity(rows.len());
        for row in rows {
            leases.push(RoleMailboxClaimLeaseV1::try_from(row)?);
        }
        let mut reachable: Vec<Uuid> = leases
            .iter()
            .filter(|lease| lease.takeover_of.is_none())
            .map(|lease| lease.lease_id)
            .collect();
        let mut frontier = reachable.clone();
        while !frontier.is_empty() {
            let mut next = Vec::new();
            for lease in &leases {
                let Some(parent) = lease.takeover_of else {
                    continue;
                };
                if frontier.contains(&parent) && !reachable.contains(&lease.lease_id) {
                    reachable.push(lease.lease_id);
                    next.push(lease.lease_id);
                }
            }
            frontier = next;
        }
        // `leases` is already ordered by (acquired_at_utc, lease_id) from the
        // query, so retaining in place preserves the CTE's ORDER BY.
        leases.retain(|lease| reachable.contains(&lease.lease_id));
        Ok(leases)
    }

    // ------------------------------------------------------------------
    // MT-183 handoff bundle persistence
    // ------------------------------------------------------------------

    /// MT-183: Insert a `MailboxHandoffBundleV1` row. Recomputes the
    /// canonical-JSON content_hash and rejects the insert with
    /// `MailboxError::HashMismatch` if the caller-supplied hash does not
    /// match. This satisfies `red_team.minimum_controls`:
    ///   1. tampered-bundle insert returns a typed error rather than a soft
    ///      warning.
    ///   2. defense-in-depth — even if the application layer skips
    ///      `recompute_hash`, the repo re-runs it before persisting.
    pub async fn insert_handoff_bundle(
        &self,
        bundle: &MailboxHandoffBundleV1,
    ) -> Result<(), MailboxError> {
        let recomputed = bundle.recompute_hash();
        if recomputed != bundle.content_hash {
            return Err(MailboxError::HashMismatch {
                expected: recomputed,
                got: bundle.content_hash.clone(),
            });
        }
        let content = BundleRow {
            bundle_id: bundle.bundle_id,
            source_thread_id: thread_record(bundle.source_thread_id),
            source_message_id: bundle.source_message_id,
            target_role: bundle.target_role.to_string(),
            target_executor_kind: executor_kind_str(bundle.target_executor_kind).to_string(),
            context_summary: bundle.context_summary.clone(),
            linked_artifacts: json_array(&bundle.linked_artifacts)?,
            transcript_pointer: match &bundle.transcript_pointer {
                Some(pointer) => Some(serde_json::to_value(pointer)?),
                None => None,
            },
            capability_grants: json_array(&bundle.capability_grants)?,
            expires_at_utc: bundle.expires_at_utc,
            content_hash: recomputed,
            created_at_utc: bundle.created_at_utc,
            created_by_session: bundle.created_by_session,
        };
        let _: Vec<BundleRow> = self
            .query(
                CREATE_BUNDLE_QUERY,
                CreateBindings::new(bundle_record(bundle.bundle_id), content),
            )
            .await?;
        Ok(())
    }

    /// MT-183: Fetch a stored handoff bundle by `bundle_id`. Returns `None`
    /// when no row matches. The returned bundle includes the
    /// stored-as-canonical `content_hash` so callers can re-verify if they
    /// want defence-in-depth.
    pub async fn get_handoff_bundle(
        &self,
        bundle_id: Uuid,
    ) -> Result<Option<MailboxHandoffBundleV1>, MailboxError> {
        let rows: Vec<BundleRow> = self
            .query(GET_BUNDLE_QUERY, BundleIdBinding { bundle_id })
            .await?;
        rows.into_iter().next().map(TryInto::try_into).transpose()
    }

    /// MT-183: List handoff bundles for a given thread, chronological by
    /// `created_at_utc`. Useful for auditing the handoff chain a thread
    /// produced.
    pub async fn list_handoff_bundles_for_thread(
        &self,
        thread_id: RoleMailboxThreadId,
    ) -> Result<Vec<MailboxHandoffBundleV1>, MailboxError> {
        let rows: Vec<BundleRow> = self
            .query(
                LIST_BUNDLES_FOR_THREAD_QUERY,
                ThreadRecordBinding {
                    thread: thread_record(thread_id.as_uuid()),
                },
            )
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[derive(Debug, Clone, SurrealValue)]
struct PredecessorRow {
    expires_at_utc: DateTime<Utc>,
    released_at_utc: Option<DateTime<Utc>>,
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn thread_record(id: Uuid) -> RecordId {
    RecordId::new(THREAD_TABLE, surrealdb::types::Uuid::from(id))
}

fn message_record(id: Uuid) -> RecordId {
    RecordId::new(MESSAGE_TABLE, surrealdb::types::Uuid::from(id))
}

fn lease_record(id: Uuid) -> RecordId {
    RecordId::new(LEASE_TABLE, surrealdb::types::Uuid::from(id))
}

fn bundle_record(id: Uuid) -> RecordId {
    RecordId::new(BUNDLE_TABLE, surrealdb::types::Uuid::from(id))
}

/// The UUID behind a record link. Every mailbox table keys its records by UUID
/// (the schema asserts it for threads and leases), so a non-UUID key is a
/// corrupt link rather than a case to tolerate.
fn record_key_uuid(record: &RecordId) -> Result<Uuid, MailboxError> {
    match &record.key {
        surrealdb::types::RecordIdKey::Uuid(value) => Ok(value.into_inner()),
        other => Err(MailboxError::Parse(format!(
            "record id key is not a uuid: {other:?}"
        ))),
    }
}

fn lease_row(lease: &RoleMailboxClaimLeaseV1) -> LeaseRow {
    LeaseRow {
        lease_id: lease.lease_id,
        thread_id: thread_record(lease.thread_id),
        holder_executor_kind: executor_kind_str(lease.holder_executor_kind).to_string(),
        holder_role_id: lease.holder_role_id.to_string(),
        holder_session_id: lease.holder_session_id,
        acquired_at_utc: lease.acquired_at_utc,
        expires_at_utc: lease.expires_at_utc,
        released_at_utc: lease.released_at_utc,
        takeover_of: lease.takeover_of.map(lease_record),
        takeover_reason: lease.takeover_reason.clone(),
    }
}

fn encode_enum<T: serde::Serialize>(value: &T, label: &str) -> Result<String, MailboxError> {
    serde_json::to_value(value)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| MailboxError::Parse(format!("{label} encode")))
}

fn json_array<T: serde::Serialize>(values: &[T]) -> Result<Vec<Value>, MailboxError> {
    values
        .iter()
        .map(|value| serde_json::to_value(value).map_err(MailboxError::from))
        .collect()
}

/// The `from` states from which `requested` is a legal thread transition.
///
/// Derived by asking [`transition_thread_state`] itself, so the SurrealQL guard
/// and the Rust state machine are the same matrix by construction.
fn allowed_thread_from_states(requested: ThreadLifecycleState) -> Vec<String> {
    ALL_THREAD_STATES
        .iter()
        .copied()
        .filter(|from| transition_thread_state(*from, requested).is_ok())
        .map(|from| from.as_str().to_string())
        .collect()
}

/// The `from` states from which `requested` is a legal message transition.
fn allowed_message_from_states(requested: MessageDeliveryState) -> Vec<String> {
    ALL_MESSAGE_STATES
        .iter()
        .copied()
        .filter(|from| transition_message_state(*from, requested).is_ok())
        .map(|from| from.as_str().to_string())
        .collect()
}

fn non_terminal_thread_states() -> Vec<String> {
    ALL_THREAD_STATES
        .iter()
        .copied()
        .filter(|state| !state.is_terminal())
        .map(|state| state.as_str().to_string())
        .collect()
}

fn executor_kind_str(kind: ExecutorKind) -> &'static str {
    match kind {
        ExecutorKind::LocalSmallModel => "local_small_model",
        ExecutorKind::CloudModel => "cloud_model",
        ExecutorKind::Reviewer => "reviewer",
        ExecutorKind::Validator => "validator",
        ExecutorKind::Operator => "operator",
        ExecutorKind::WorkflowAutomation => "workflow_automation",
    }
}

fn parse_executor_kind(s: &str) -> Option<ExecutorKind> {
    match s {
        "local_small_model" => Some(ExecutorKind::LocalSmallModel),
        "cloud_model" => Some(ExecutorKind::CloudModel),
        "reviewer" => Some(ExecutorKind::Reviewer),
        "validator" => Some(ExecutorKind::Validator),
        "operator" => Some(ExecutorKind::Operator),
        "workflow_automation" => Some(ExecutorKind::WorkflowAutomation),
        _ => None,
    }
}

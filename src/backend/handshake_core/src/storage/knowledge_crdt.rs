//! WP-KERNEL-009 CRDTAndConcurrencyCore storage (MT-065..MT-080).
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11. This
//! module is the storage surface for the WP-009 CRDT support tables
//! (`knowledge_crdt_*`, migrations 0150-0159): denial receipts, graph
//! mutation proposals, promoted facts, AI edit proposals, agent lane leases,
//! swarm checkpoints and recovery receipts.
//!
//! Pattern follows `storage/knowledge.rs` (MT-049 precedent): free async
//! functions over the embedded SurrealDB store (`&SurrealStorage`) rather than
//! widening the legacy `Database` trait. There is NO in-memory, SQLite, or
//! fixture fallback: without the durable store every function fails closed with
//! a typed `StorageError`.
//!
//! WP-KERNEL-012 MT-136 notes on what the port had to preserve:
//!
//! * Foreign keys are RECORD LINKS (`record<table>` with
//!   `ASSERT record::exists($value)`), so a proposal citing a missing ledger
//!   event, or a checkpoint citing a missing lease, is refused by the store -
//!   the guarantee the relational FKs gave.
//! * `is_expired` is still evaluated against the DATABASE clock
//!   (`expires_at_utc < time::now()` inside the statement), never the client
//!   clock, which is what makes server-side lease expiry enforceable.
//! * The single-holder lease guard is still server-side: the
//!   `uq_knowledge_crdt_lease_active_scope` UNIQUE index over the computed
//!   `active_scope_key` replaces the relational partial unique index, and the
//!   claim/takeover paths run their check and their write inside ONE statement
//!   so two claimants cannot both see the scope as free.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use crate::kernel::crdt::actor_site::KnowledgeActorIdV1;

use super::surreal::{SurrealStorage, SurrealStorageError};
use super::{StorageError, StorageResult};

const LEASES_TABLE: &str = "knowledge_crdt_agent_lane_leases";
const GRAPH_PROPOSALS_TABLE: &str = "knowledge_crdt_graph_proposals";
const CHECKPOINTS_TABLE: &str = "knowledge_crdt_swarm_checkpoints";
const KERNEL_EVENT_LEDGER_TABLE: &str = "kernel_event_ledger";

fn map_err(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

/// The plain id behind a record link.
///
/// `RecordIdKey` has no `Display` that yields a bare id (its SurrealQL
/// rendering would quote the value), so the key is destructured instead.
fn record_key(record_id: RecordId, reason: &'static str) -> StorageResult<String> {
    let RecordIdKey::String(key) = record_id.key else {
        return Err(StorageError::Conflict(reason));
    };
    Ok(key)
}

fn optional_record_key(
    record_id: Option<RecordId>,
    reason: &'static str,
) -> StorageResult<Option<String>> {
    record_id.map(|id| record_key(id, reason)).transpose()
}

fn link(table: &'static str, id: &str) -> RecordId {
    RecordId::new(table, id)
}

fn optional_link(table: &'static str, id: Option<&str>) -> Option<RecordId> {
    id.map(|value| RecordId::new(table, value))
}

// ---------------------------------------------------------------------------
// MT-070 denial receipts (shared by MT-069/071/074/076 denial paths).
// ---------------------------------------------------------------------------

/// Durable typed denial receipt (row of `knowledge_crdt_denial_receipts`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeCrdtDenialReceiptRow {
    pub receipt_id: String,
    pub receipt_kind: String,
    pub workspace_id: String,
    pub document_id: Option<String>,
    pub crdt_document_id: Option<String>,
    pub scope_ref: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub denial_payload: Value,
    pub event_ledger_event_id: String,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
}

/// Allowed receipt kinds (mirrors the migration CHECK; kept in Rust so
/// callers fail closed before touching the database).
pub const KNOWLEDGE_CRDT_DENIAL_KINDS: [&str; 12] = [
    "stale_draft_save",
    "concurrent_draft_fork",
    "ahead_of_head_save",
    "update_content_mismatch",
    "sequence_slot_race",
    "lease_write_denied",
    "index_run_slot_rejected",
    "graph_promotion_denied",
    "ai_edit_promotion_denied",
    // Authority-hardening #5: an applied update did not hash to the approved
    // proposal's diff_sha256 (approved-vs-applied binding violation).
    "ai_edit_applied_mismatch",
    // MT-074 V1 FAIL remediation: an applied-binding cited an update id with no
    // matching kernel_crdt_updates row (or a row whose stored content hash
    // disagreed with the presented content).
    "ai_edit_applied_update_missing",
    // MT-260 AI Loom jobs: a promote attempt on a pending/rejected suggestion,
    // or by a non-operator/non-validator actor, leaves this durable receipt.
    "loom_ai_promotion_denied",
];

/// Generate a new denial receipt id (`KCDR-<32 hex>`, time-ordered v7 per
/// HBR-INT-008).
pub fn new_denial_receipt_id() -> String {
    format!("KCDR-{}", Uuid::now_v7().simple())
}

/// Input for [`insert_denial_receipt`].
#[derive(Clone, Debug)]
pub struct NewKnowledgeCrdtDenialReceipt {
    pub receipt_id: String,
    pub receipt_kind: String,
    pub workspace_id: String,
    pub document_id: Option<String>,
    pub crdt_document_id: Option<String>,
    pub scope_ref: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub denial_payload: Value,
    pub event_ledger_event_id: String,
    pub idempotency_key: String,
}

/// Insert a denial receipt; idempotent on `idempotency_key` (replays return
/// the previously stored row).
/// Stored `knowledge_crdt_denial_receipts` projection.
#[derive(SurrealValue)]
struct DenialReceiptRecord {
    receipt_id: String,
    receipt_kind: String,
    workspace_id: String,
    document_id: Option<String>,
    crdt_document_id: Option<String>,
    scope_ref: String,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    correlation_id: String,
    denial_payload: Value,
    event_ledger_event_id: RecordId,
    idempotency_key: String,
    created_at: Datetime,
}

impl DenialReceiptRecord {
    fn into_row(self) -> StorageResult<KnowledgeCrdtDenialReceiptRow> {
        Ok(KnowledgeCrdtDenialReceiptRow {
            receipt_id: self.receipt_id,
            receipt_kind: self.receipt_kind,
            workspace_id: self.workspace_id,
            document_id: self.document_id,
            crdt_document_id: self.crdt_document_id,
            scope_ref: self.scope_ref,
            actor_id: self.actor_id,
            actor_kind: self.actor_kind,
            session_id: self.session_id,
            correlation_id: self.correlation_id,
            denial_payload: self.denial_payload,
            event_ledger_event_id: record_key(
                self.event_ledger_event_id,
                "denial receipt ledger link is not a string key",
            )?,
            idempotency_key: self.idempotency_key,
            created_at: self.created_at.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct DenialReceiptCreate {
    receipt_id: String,
    receipt_kind: String,
    workspace_id: String,
    document_id: Option<String>,
    crdt_document_id: Option<String>,
    scope_ref: String,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    correlation_id: String,
    denial_payload: Value,
    event_ledger_event_id: RecordId,
    idempotency_key: String,
}

#[derive(SurrealValue)]
struct IdempotencyKeyBindings {
    idempotency_key: String,
}

#[derive(SurrealValue)]
struct CrdtDocumentBindings {
    crdt_document_id: String,
}

#[derive(SurrealValue)]
struct ScopeRefBindings {
    scope_ref: String,
}

/// Insert a denial receipt; idempotent on `idempotency_key` (replays return
/// the previously stored row).
///
/// The `ON CONFLICT (idempotency_key) DO NOTHING` plus read-back becomes ONE
/// statement: the existence test on the idempotency key and the create cannot
/// interleave, so two concurrent denials for the same key still converge on the
/// first stored row instead of both believing they inserted.
pub async fn insert_denial_receipt(
    storage: &SurrealStorage,
    receipt: NewKnowledgeCrdtDenialReceipt,
) -> StorageResult<KnowledgeCrdtDenialReceiptRow> {
    if !KNOWLEDGE_CRDT_DENIAL_KINDS.contains(&receipt.receipt_kind.as_str()) {
        return Err(StorageError::Validation(
            "unknown knowledge CRDT denial receipt kind",
        ));
    }
    let actor = KnowledgeActorIdV1::parse(&receipt.actor_id).map_err(|_| {
        StorageError::Validation("knowledge CRDT denial receipt actor id is not typed")
    })?;
    if actor.kind().as_str() != receipt.actor_kind {
        return Err(StorageError::Validation(
            "knowledge CRDT denial receipt actor kind does not match actor id",
        ));
    }
    let content = DenialReceiptCreate {
        receipt_id: receipt.receipt_id.clone(),
        receipt_kind: receipt.receipt_kind.clone(),
        workspace_id: receipt.workspace_id.clone(),
        document_id: receipt.document_id.clone(),
        crdt_document_id: receipt.crdt_document_id.clone(),
        scope_ref: receipt.scope_ref.clone(),
        actor_id: receipt.actor_id.clone(),
        actor_kind: receipt.actor_kind.clone(),
        session_id: receipt.session_id.clone(),
        correlation_id: receipt.correlation_id.clone(),
        denial_payload: receipt.denial_payload.clone(),
        event_ledger_event_id: link(KERNEL_EVENT_LEDGER_TABLE, &receipt.event_ledger_event_id),
        idempotency_key: receipt.idempotency_key.clone(),
    };
    let rows: Vec<DenialReceiptRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "{ \
                           LET $existing = (SELECT * FROM knowledge_crdt_denial_receipts \
                             WHERE idempotency_key = $idempotency_key); \
                           IF array::len($existing) > 0 { \
                             RETURN $existing \
                           } ELSE { \
                             RETURN CREATE \
                               type::record('knowledge_crdt_denial_receipts', $receipt_id) \
                               CONTENT { receipt_id: $receipt_id, receipt_kind: $receipt_kind, \
                                 workspace_id: $workspace_id, document_id: $document_id, \
                                 crdt_document_id: $crdt_document_id, scope_ref: $scope_ref, \
                                 actor_id: $actor_id, actor_kind: $actor_kind, \
                                 session_id: $session_id, correlation_id: $correlation_id, \
                                 denial_payload: $denial_payload, \
                                 event_ledger_event_id: $event_ledger_event_id, \
                                 idempotency_key: $idempotency_key } \
                           }; \
                         };",
                        content,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::NotFound(
            "knowledge CRDT denial receipt after insert",
        ))?
        .into_row()
}

pub async fn get_denial_receipt_by_idempotency_key(
    storage: &SurrealStorage,
    idempotency_key: &str,
) -> StorageResult<Option<KnowledgeCrdtDenialReceiptRow>> {
    let bindings = IdempotencyKeyBindings {
        idempotency_key: idempotency_key.to_owned(),
    };
    let record: Option<DenialReceiptRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM knowledge_crdt_denial_receipts \
                         WHERE idempotency_key = $idempotency_key;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(DenialReceiptRecord::into_row).transpose()
}

/// All denial receipts for a CRDT document, oldest first (MT-075 input).
pub async fn list_denial_receipts_for_document(
    storage: &SurrealStorage,
    crdt_document_id: &str,
) -> StorageResult<Vec<KnowledgeCrdtDenialReceiptRow>> {
    let bindings = CrdtDocumentBindings {
        crdt_document_id: crdt_document_id.to_owned(),
    };
    let records: Vec<DenialReceiptRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM knowledge_crdt_denial_receipts \
                         WHERE crdt_document_id = $crdt_document_id \
                         ORDER BY created_at ASC, receipt_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(DenialReceiptRecord::into_row)
        .collect()
}

/// All denial receipts for an arbitrary scope ref (lease scopes, proposals).
pub async fn list_denial_receipts_for_scope(
    storage: &SurrealStorage,
    scope_ref: &str,
) -> StorageResult<Vec<KnowledgeCrdtDenialReceiptRow>> {
    let bindings = ScopeRefBindings {
        scope_ref: scope_ref.to_owned(),
    };
    let records: Vec<DenialReceiptRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM knowledge_crdt_denial_receipts \
                         WHERE scope_ref = $scope_ref \
                         ORDER BY created_at ASC, receipt_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(DenialReceiptRecord::into_row)
        .collect()
}

// ---------------------------------------------------------------------------
// MT-076 agent lane leases (MT-041 seed AgentLaneLease semantics).
// ---------------------------------------------------------------------------

/// One agent lane lease (row of `knowledge_crdt_agent_lane_leases`).
/// `is_expired` is evaluated against the DATABASE clock at read time, never
/// the client clock (server-side expiry enforcement, MT-041 seed).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentLaneLeaseRow {
    pub lease_id: String,
    pub lane_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub claimed_at_utc: DateTime<Utc>,
    pub expires_at_utc: DateTime<Utc>,
    pub renewal_count: i64,
    pub released_at_utc: Option<DateTime<Utc>>,
    pub expired_at_utc: Option<DateTime<Utc>>,
    pub takeover_of: Option<String>,
    /// `expires_at_utc < NOW()` per the database clock when the row was read.
    pub is_expired: bool,
}

impl AgentLaneLeaseRow {
    pub fn scope_ref(&self) -> String {
        format!("{}:{}", self.scope_kind, self.scope_id)
    }

    pub fn is_active(&self) -> bool {
        self.released_at_utc.is_none() && !self.is_expired
    }
}

#[derive(Clone, Debug)]
pub struct NewAgentLaneLease {
    pub lease_id: String,
    pub lane_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub ttl_seconds: i64,
    pub takeover_of: Option<String>,
}

/// Stored `knowledge_crdt_agent_lane_leases` projection.
///
/// `is_expired` is NOT a stored column: every statement below computes it as
/// `expires_at_utc < time::now()` inside the query, so the value is the
/// database clock's verdict, exactly as `(expires_at_utc < NOW())` was.
#[derive(SurrealValue)]
struct LeaseRecord {
    lease_id: String,
    lane_id: String,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    correlation_id: String,
    scope_kind: String,
    scope_id: String,
    claimed_at_utc: Datetime,
    expires_at_utc: Datetime,
    renewal_count: i64,
    released_at_utc: Option<Datetime>,
    expired_at_utc: Option<Datetime>,
    takeover_of: Option<RecordId>,
    is_expired: bool,
}

impl LeaseRecord {
    fn into_row(self) -> StorageResult<AgentLaneLeaseRow> {
        Ok(AgentLaneLeaseRow {
            lease_id: self.lease_id,
            lane_id: self.lane_id,
            actor_id: self.actor_id,
            actor_kind: self.actor_kind,
            session_id: self.session_id,
            correlation_id: self.correlation_id,
            scope_kind: self.scope_kind,
            scope_id: self.scope_id,
            claimed_at_utc: self.claimed_at_utc.into_inner(),
            expires_at_utc: self.expires_at_utc.into_inner(),
            renewal_count: self.renewal_count,
            released_at_utc: self.released_at_utc.map(Datetime::into_inner),
            expired_at_utc: self.expired_at_utc.map(Datetime::into_inner),
            takeover_of: optional_record_key(
                self.takeover_of,
                "lease takeover link is not a string key",
            )?,
            is_expired: self.is_expired,
        })
    }
}

#[derive(SurrealValue)]
struct LeaseCreateBindings {
    lease_id: String,
    lane_id: String,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    correlation_id: String,
    scope_kind: String,
    scope_id: String,
    ttl_seconds: i64,
    takeover_of: Option<RecordId>,
}

#[derive(SurrealValue)]
struct LeaseIdBindings {
    lease_id: String,
}

#[derive(SurrealValue)]
struct LeaseScopeBindings {
    scope_kind: String,
    scope_id: String,
}

#[derive(SurrealValue)]
struct LeaseRenewBindings {
    lease_id: String,
    actor_id: String,
    ttl_seconds: i64,
}

#[derive(SurrealValue)]
struct LeaseReleaseBindings {
    lease_id: String,
    actor_id: String,
}

#[derive(SurrealValue)]
struct LeaseSweepBindings {
    unused: bool,
}

#[derive(SurrealValue)]
struct LeaseTakeoverBindings {
    prior_lease_id: String,
    lease_id: String,
    lane_id: String,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    correlation_id: String,
    scope_kind: String,
    scope_id: String,
    ttl_seconds: i64,
}

/// Result of the single-statement claim: which branch the store took, plus the
/// row that branch produced.
#[derive(SurrealValue)]
struct LeaseClaimOutcome {
    inserted: bool,
    lease: LeaseRecord,
}

/// Result of the single-statement takeover: a typed failure discriminant plus
/// the row when the takeover succeeded.
#[derive(SurrealValue)]
struct LeaseTakeoverOutcome {
    status: String,
    expires_at_utc: Option<Datetime>,
    lease: Option<LeaseRecord>,
}

/// Typed insertion failure: another unreleased lease holds the scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaseInsertOutcome {
    Inserted(Box<AgentLaneLeaseRow>),
    ScopeHeld { holder: Box<AgentLaneLeaseRow> },
}

/// Claim a lease. The partial unique index on (scope_kind, scope_id) WHERE
/// released_at_utc IS NULL is the server-side single-holder guard; a unique
/// violation is surfaced as `ScopeHeld` with the holder row.
pub async fn insert_lease(
    storage: &SurrealStorage,
    lease: NewAgentLaneLease,
) -> StorageResult<LeaseInsertOutcome> {
    if lease.ttl_seconds <= 0 {
        return Err(StorageError::Validation("lease ttl must be positive"));
    }
    // The relational path relied on catching the unique violation and then
    // re-reading the holder, which left a window where the holder could vanish
    // between the two round trips (hence its "holder vanished; retry claim"
    // arm). One block statement removes that window: the holder lookup and the
    // create run in one transaction, so `ScopeHeld` always carries a real
    // holder. The UNIQUE index on `active_scope_key` remains the backstop.
    let bindings = LeaseCreateBindings {
        lease_id: lease.lease_id.clone(),
        lane_id: lease.lane_id.clone(),
        actor_id: lease.actor_id.clone(),
        actor_kind: lease.actor_kind.clone(),
        session_id: lease.session_id.clone(),
        correlation_id: lease.correlation_id.clone(),
        scope_kind: lease.scope_kind.clone(),
        scope_id: lease.scope_id.clone(),
        ttl_seconds: lease.ttl_seconds,
        takeover_of: optional_link(LEASES_TABLE, lease.takeover_of.as_deref()),
    };
    let outcomes: Vec<LeaseClaimOutcome> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "{ \
                           LET $now = time::now(); \
                           LET $held = (SELECT lease_id, lane_id, actor_id, actor_kind, \
                             session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, \
                             expires_at_utc, renewal_count, released_at_utc, expired_at_utc, \
                             takeover_of, expires_at_utc < $now AS is_expired \
                             FROM knowledge_crdt_agent_lane_leases \
                             WHERE scope_kind = $scope_kind AND scope_id = $scope_id \
                               AND released_at_utc = NONE); \
                           IF array::len($held) > 0 { \
                             RETURN { inserted: false, lease: $held[0] } \
                           } ELSE { \
                             LET $created = (CREATE \
                               type::record('knowledge_crdt_agent_lane_leases', $lease_id) \
                               CONTENT { lease_id: $lease_id, lane_id: $lane_id, \
                                 actor_id: $actor_id, actor_kind: $actor_kind, \
                                 session_id: $session_id, correlation_id: $correlation_id, \
                                 scope_kind: $scope_kind, scope_id: $scope_id, \
                                 claimed_at_utc: $now, \
                                 expires_at_utc: $now + duration::from::secs($ttl_seconds), \
                                 renewal_count: 0, takeover_of: $takeover_of })[0]; \
                             RETURN { inserted: true, lease: { \
                               lease_id: $created.lease_id, lane_id: $created.lane_id, \
                               actor_id: $created.actor_id, actor_kind: $created.actor_kind, \
                               session_id: $created.session_id, \
                               correlation_id: $created.correlation_id, \
                               scope_kind: $created.scope_kind, scope_id: $created.scope_id, \
                               claimed_at_utc: $created.claimed_at_utc, \
                               expires_at_utc: $created.expires_at_utc, \
                               renewal_count: $created.renewal_count, \
                               released_at_utc: $created.released_at_utc, \
                               expired_at_utc: $created.expired_at_utc, \
                               takeover_of: $created.takeover_of, \
                               is_expired: $created.expires_at_utc < $now } } \
                           }; \
                         };",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    let outcome = outcomes.into_iter().next().ok_or(StorageError::Database(
        "lease claim produced no outcome".to_owned(),
    ))?;
    let row = outcome.lease.into_row()?;
    if outcome.inserted {
        Ok(LeaseInsertOutcome::Inserted(Box::new(row)))
    } else {
        Ok(LeaseInsertOutcome::ScopeHeld {
            holder: Box::new(row),
        })
    }
}

pub async fn get_lease(
    storage: &SurrealStorage,
    lease_id: &str,
) -> StorageResult<Option<AgentLaneLeaseRow>> {
    let bindings = LeaseIdBindings {
        lease_id: lease_id.to_owned(),
    };
    let record: Option<LeaseRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT lease_id, lane_id, actor_id, actor_kind, session_id, \
                         correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, \
                         renewal_count, released_at_utc, expired_at_utc, takeover_of, \
                         expires_at_utc < time::now() AS is_expired \
                         FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(LeaseRecord::into_row).transpose()
}

/// The unreleased lease for a scope, if any (may be expired; `is_expired`
/// tells, per the database clock).
pub async fn find_unreleased_lease_for_scope(
    storage: &SurrealStorage,
    scope_kind: &str,
    scope_id: &str,
) -> StorageResult<Option<AgentLaneLeaseRow>> {
    let bindings = LeaseScopeBindings {
        scope_kind: scope_kind.to_owned(),
        scope_id: scope_id.to_owned(),
    };
    let record: Option<LeaseRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT lease_id, lane_id, actor_id, actor_kind, session_id, \
                         correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, \
                         renewal_count, released_at_utc, expired_at_utc, takeover_of, \
                         expires_at_utc < time::now() AS is_expired \
                         FROM knowledge_crdt_agent_lane_leases \
                         WHERE scope_kind = $scope_kind AND scope_id = $scope_id \
                         AND released_at_utc = NONE;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(LeaseRecord::into_row).transpose()
}

/// Renew: extends expiry by `ttl_seconds` from NOW() without changing lease
/// identity. Server-side guards: own lease, unreleased, NOT expired.
pub async fn renew_lease(
    storage: &SurrealStorage,
    lease_id: &str,
    actor_id: &str,
    ttl_seconds: i64,
) -> StorageResult<Option<AgentLaneLeaseRow>> {
    if ttl_seconds <= 0 {
        return Err(StorageError::Validation("lease ttl must be positive"));
    }
    // The guards (own lease, unreleased, NOT expired per the database clock)
    // stay in the WHERE clause, so a renewal of an expired lease still matches
    // nothing and returns `None` rather than being decided client-side. The
    // block pins ONE `$now` so the guard and the new expiry use the same
    // database instant, and the re-projection recomputes `is_expired` against
    // that same instant inside the transaction.
    let bindings = LeaseRenewBindings {
        lease_id: lease_id.to_owned(),
        actor_id: actor_id.to_owned(),
        ttl_seconds,
    };
    let records: Vec<LeaseRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "{ \
                           LET $now = time::now(); \
                           LET $ids = (UPDATE knowledge_crdt_agent_lane_leases \
                             SET expires_at_utc = $now + duration::from::secs($ttl_seconds), \
                                 renewal_count = renewal_count + 1 \
                             WHERE lease_id = $lease_id AND actor_id = $actor_id \
                               AND released_at_utc = NONE AND expires_at_utc > $now \
                             RETURN AFTER).lease_id; \
                           RETURN SELECT lease_id, lane_id, actor_id, actor_kind, session_id, \
                             correlation_id, scope_kind, scope_id, claimed_at_utc, \
                             expires_at_utc, renewal_count, released_at_utc, expired_at_utc, \
                             takeover_of, expires_at_utc < $now AS is_expired \
                             FROM knowledge_crdt_agent_lane_leases WHERE lease_id IN $ids; \
                         };",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .next()
        .map(LeaseRecord::into_row)
        .transpose()
}

/// Release an own lease (allowed after expiry as cleanup; expiry only blocks
/// writes and renewals).
pub async fn release_lease(
    storage: &SurrealStorage,
    lease_id: &str,
    actor_id: &str,
) -> StorageResult<Option<AgentLaneLeaseRow>> {
    let bindings = LeaseReleaseBindings {
        lease_id: lease_id.to_owned(),
        actor_id: actor_id.to_owned(),
    };
    let records: Vec<LeaseRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "{ \
                           LET $now = time::now(); \
                           LET $ids = (UPDATE knowledge_crdt_agent_lane_leases \
                             SET released_at_utc = $now \
                             WHERE lease_id = $lease_id AND actor_id = $actor_id \
                               AND released_at_utc = NONE \
                             RETURN AFTER).lease_id; \
                           RETURN SELECT lease_id, lane_id, actor_id, actor_kind, session_id, \
                             correlation_id, scope_kind, scope_id, claimed_at_utc, \
                             expires_at_utc, renewal_count, released_at_utc, expired_at_utc, \
                             takeover_of, expires_at_utc < $now AS is_expired \
                             FROM knowledge_crdt_agent_lane_leases WHERE lease_id IN $ids; \
                         };",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .next()
        .map(LeaseRecord::into_row)
        .transpose()
}

/// Server-side expiry sweep: stamp every overdue unreleased lease exactly
/// once (expired_at_utc) and return the stamped rows so the kernel layer
/// can append the KNOWLEDGE_CRDT_LEASE_EXPIRED events.
pub async fn sweep_expired_leases(
    storage: &SurrealStorage,
) -> StorageResult<Vec<AgentLaneLeaseRow>> {
    // "Exactly once" still comes from the `expired_at_utc = NONE` guard inside
    // the statement, not from a client-side check, so two concurrent sweeps
    // cannot both stamp and both emit the expiry event.
    let bindings = LeaseSweepBindings { unused: true };
    let records: Vec<LeaseRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "{ \
                           LET $now = time::now(); \
                           LET $ids = (UPDATE knowledge_crdt_agent_lane_leases \
                             SET expired_at_utc = $now \
                             WHERE released_at_utc = NONE AND expired_at_utc = NONE \
                               AND expires_at_utc < $now \
                             RETURN AFTER).lease_id; \
                           RETURN SELECT lease_id, lane_id, actor_id, actor_kind, session_id, \
                             correlation_id, scope_kind, scope_id, claimed_at_utc, \
                             expires_at_utc, renewal_count, released_at_utc, expired_at_utc, \
                             takeover_of, expires_at_utc < $now AS is_expired \
                             FROM knowledge_crdt_agent_lane_leases WHERE lease_id IN $ids \
                             ORDER BY lease_id ASC; \
                         };",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records.into_iter().map(LeaseRecord::into_row).collect()
}

/// Typed takeover failure reasons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaseTakeoverFailure {
    PriorLeaseNotFound,
    PriorLeaseNotExpired { expires_at_utc: DateTime<Utc> },
    PriorLeaseAlreadyReleased,
}

/// Take over an EXPIRED lease's scope: atomically release the prior lease
/// (stamping expired_at_utc if the sweep has not run) and insert the new
/// lease with `takeover_of` lineage. Server-side: the prior lease MUST be
/// past expiry on the database clock.
pub async fn takeover_lease(
    storage: &SurrealStorage,
    prior_lease_id: &str,
    new_lease: NewAgentLaneLease,
) -> StorageResult<Result<AgentLaneLeaseRow, LeaseTakeoverFailure>> {
    if new_lease.ttl_seconds <= 0 {
        return Err(StorageError::Validation("lease ttl must be positive"));
    }
    // The relational version held `SELECT ... FOR UPDATE` across the release and
    // the insert. The block statement is the equivalent: the prior lease is
    // inspected, released and superseded inside ONE transaction against ONE
    // `$now`, so no other claimant can take the scope between the expiry
    // verdict and the new claim. The three typed failures are carried out as a
    // discriminant rather than as three separate round trips.
    let bindings = LeaseTakeoverBindings {
        prior_lease_id: prior_lease_id.to_owned(),
        lease_id: new_lease.lease_id.clone(),
        lane_id: new_lease.lane_id.clone(),
        actor_id: new_lease.actor_id.clone(),
        actor_kind: new_lease.actor_kind.clone(),
        session_id: new_lease.session_id.clone(),
        correlation_id: new_lease.correlation_id.clone(),
        scope_kind: new_lease.scope_kind.clone(),
        scope_id: new_lease.scope_id.clone(),
        ttl_seconds: new_lease.ttl_seconds,
    };
    let outcomes: Vec<LeaseTakeoverOutcome> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "{ \
                           LET $now = time::now(); \
                           LET $prior = (SELECT * FROM knowledge_crdt_agent_lane_leases \
                             WHERE lease_id = $prior_lease_id)[0]; \
                           IF $prior = NONE { \
                             RETURN { status: 'not_found', expires_at_utc: NONE, lease: NONE } \
                           } ELSE IF $prior.released_at_utc != NONE { \
                             RETURN { status: 'already_released', expires_at_utc: NONE, \
                                      lease: NONE } \
                           } ELSE IF $prior.expires_at_utc >= $now { \
                             RETURN { status: 'not_expired', \
                                      expires_at_utc: $prior.expires_at_utc, lease: NONE } \
                           } ELSE { \
                             UPDATE knowledge_crdt_agent_lane_leases \
                               SET released_at_utc = $now, \
                                   expired_at_utc = IF expired_at_utc = NONE { $now } \
                                                    ELSE { expired_at_utc } \
                               WHERE lease_id = $prior_lease_id; \
                             LET $created = (CREATE \
                               type::record('knowledge_crdt_agent_lane_leases', $lease_id) \
                               CONTENT { lease_id: $lease_id, lane_id: $lane_id, \
                                 actor_id: $actor_id, actor_kind: $actor_kind, \
                                 session_id: $session_id, correlation_id: $correlation_id, \
                                 scope_kind: $scope_kind, scope_id: $scope_id, \
                                 claimed_at_utc: $now, \
                                 expires_at_utc: $now + duration::from::secs($ttl_seconds), \
                                 renewal_count: 0, \
                                 takeover_of: type::record( \
                                   'knowledge_crdt_agent_lane_leases', $prior_lease_id) })[0]; \
                             RETURN { status: 'taken_over', expires_at_utc: NONE, lease: { \
                               lease_id: $created.lease_id, lane_id: $created.lane_id, \
                               actor_id: $created.actor_id, actor_kind: $created.actor_kind, \
                               session_id: $created.session_id, \
                               correlation_id: $created.correlation_id, \
                               scope_kind: $created.scope_kind, scope_id: $created.scope_id, \
                               claimed_at_utc: $created.claimed_at_utc, \
                               expires_at_utc: $created.expires_at_utc, \
                               renewal_count: $created.renewal_count, \
                               released_at_utc: $created.released_at_utc, \
                               expired_at_utc: $created.expired_at_utc, \
                               takeover_of: $created.takeover_of, \
                               is_expired: $created.expires_at_utc < $now } } \
                           }; \
                         };",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    let outcome = outcomes.into_iter().next().ok_or(StorageError::Database(
        "lease takeover produced no outcome".to_owned(),
    ))?;
    match outcome.status.as_str() {
        "not_found" => Ok(Err(LeaseTakeoverFailure::PriorLeaseNotFound)),
        "already_released" => Ok(Err(LeaseTakeoverFailure::PriorLeaseAlreadyReleased)),
        "not_expired" => Ok(Err(LeaseTakeoverFailure::PriorLeaseNotExpired {
            expires_at_utc: outcome
                .expires_at_utc
                .ok_or(StorageError::Database(
                    "lease takeover reported not-expired without an expiry".to_owned(),
                ))?
                .into_inner(),
        })),
        "taken_over" => Ok(Ok(outcome
            .lease
            .ok_or(StorageError::Database(
                "lease takeover reported success without a lease".to_owned(),
            ))?
            .into_row()?)),
        _ => Err(StorageError::Database(
            "lease takeover returned an unknown status".to_owned(),
        )),
    }
}

/// Walk the takeover lineage from `lease_id` back to the root claim
/// (newest first). Chains are short (one row per takeover).
pub async fn lease_lineage(
    storage: &SurrealStorage,
    lease_id: &str,
) -> StorageResult<Vec<AgentLaneLeaseRow>> {
    let mut lineage = Vec::new();
    let mut cursor = Some(lease_id.to_string());
    while let Some(current) = cursor {
        let Some(lease) = get_lease(storage, &current).await? else {
            break;
        };
        cursor = lease.takeover_of.clone();
        lineage.push(lease);
        if lineage.len() > 256 {
            return Err(StorageError::Validation(
                "lease lineage exceeds 256 links; data corruption suspected",
            ));
        }
    }
    if lineage.is_empty() {
        return Err(StorageError::NotFound("lease lineage root"));
    }
    Ok(lineage)
}

// ---------------------------------------------------------------------------
// MT-068 graph mutation proposals.
// ---------------------------------------------------------------------------

/// One graph mutation proposal (row of `knowledge_crdt_graph_proposals`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphMutationProposalRow {
    pub proposal_id: String,
    pub workspace_id: String,
    pub mutation_kind: String,
    pub mutation_payload: Value,
    pub source_span_refs: Value,
    pub confidence: f64,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub lease_id: Option<String>,
    pub review_state: String,
    pub decided_by: Option<String>,
    pub decided_at_utc: Option<DateTime<Utc>>,
    pub decision_reason: Option<String>,
    pub recorded_event_id: String,
    pub decided_event_id: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

/// Stored `knowledge_crdt_graph_proposals` projection.
#[derive(SurrealValue)]
struct GraphProposalRecord {
    proposal_id: String,
    workspace_id: String,
    mutation_kind: String,
    mutation_payload: Value,
    source_span_refs: Vec<String>,
    confidence: f64,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    correlation_id: String,
    lease_id: Option<RecordId>,
    review_state: String,
    decided_by: Option<String>,
    decided_at_utc: Option<Datetime>,
    decision_reason: Option<String>,
    recorded_event_id: RecordId,
    decided_event_id: Option<RecordId>,
    created_at_utc: Datetime,
}

impl GraphProposalRecord {
    fn into_row(self) -> StorageResult<GraphMutationProposalRow> {
        Ok(GraphMutationProposalRow {
            proposal_id: self.proposal_id,
            workspace_id: self.workspace_id,
            mutation_kind: self.mutation_kind,
            mutation_payload: self.mutation_payload,
            // The public shape has always been a JSON array; the store now
            // types the column as `array<string>`, so it is rebuilt rather than
            // passed through as opaque JSONB.
            source_span_refs: Value::from(self.source_span_refs),
            confidence: self.confidence,
            actor_id: self.actor_id,
            actor_kind: self.actor_kind,
            session_id: self.session_id,
            correlation_id: self.correlation_id,
            lease_id: optional_record_key(
                self.lease_id,
                "graph proposal lease link is not a string key",
            )?,
            review_state: self.review_state,
            decided_by: self.decided_by,
            decided_at_utc: self.decided_at_utc.map(Datetime::into_inner),
            decision_reason: self.decision_reason,
            recorded_event_id: record_key(
                self.recorded_event_id,
                "graph proposal recorded-event link is not a string key",
            )?,
            decided_event_id: optional_record_key(
                self.decided_event_id,
                "graph proposal decided-event link is not a string key",
            )?,
            created_at_utc: self.created_at_utc.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct GraphProposalCreate {
    proposal_id: String,
    workspace_id: String,
    mutation_kind: String,
    mutation_payload: Value,
    source_span_refs: Vec<String>,
    confidence: f64,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    correlation_id: String,
    lease_id: Option<RecordId>,
    recorded_event_id: RecordId,
}

#[derive(SurrealValue)]
struct ProposalIdBindings {
    proposal_id: String,
}

#[derive(SurrealValue)]
struct ProposalStateBindings {
    workspace_id: String,
    review_state: String,
}

#[derive(SurrealValue)]
struct ProposalDecisionBindings {
    proposal_id: String,
    review_state: String,
    decided_by: String,
    decision_reason: String,
    decided_event_id: RecordId,
}

#[derive(Clone, Debug)]
pub struct NewGraphMutationProposal {
    pub proposal_id: String,
    pub workspace_id: String,
    pub mutation_kind: String,
    pub mutation_payload: Value,
    pub source_span_refs: Vec<String>,
    pub confidence: f64,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub lease_id: Option<String>,
    pub recorded_event_id: String,
}

pub async fn insert_graph_proposal(
    storage: &SurrealStorage,
    proposal: NewGraphMutationProposal,
) -> StorageResult<GraphMutationProposalRow> {
    if proposal.source_span_refs.is_empty()
        || proposal
            .source_span_refs
            .iter()
            .any(|span| span.trim().is_empty())
    {
        return Err(StorageError::Validation(
            "graph proposal requires at least one non-empty source span ref",
        ));
    }
    let content = GraphProposalCreate {
        proposal_id: proposal.proposal_id.clone(),
        workspace_id: proposal.workspace_id.clone(),
        mutation_kind: proposal.mutation_kind.clone(),
        mutation_payload: proposal.mutation_payload.clone(),
        source_span_refs: proposal.source_span_refs.clone(),
        confidence: proposal.confidence,
        actor_id: proposal.actor_id.clone(),
        actor_kind: proposal.actor_kind.clone(),
        session_id: proposal.session_id.clone(),
        correlation_id: proposal.correlation_id.clone(),
        lease_id: optional_link(LEASES_TABLE, proposal.lease_id.as_deref()),
        recorded_event_id: link(KERNEL_EVENT_LEDGER_TABLE, &proposal.recorded_event_id),
    };
    let rows: Vec<GraphProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "CREATE type::record('knowledge_crdt_graph_proposals', $proposal_id) \
                         CONTENT { proposal_id: $proposal_id, workspace_id: $workspace_id, \
                           mutation_kind: $mutation_kind, \
                           mutation_payload: $mutation_payload, \
                           source_span_refs: $source_span_refs, confidence: $confidence, \
                           actor_id: $actor_id, actor_kind: $actor_kind, \
                           session_id: $session_id, correlation_id: $correlation_id, \
                           lease_id: $lease_id, recorded_event_id: $recorded_event_id };",
                        content,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "graph proposal insert produced no row".to_owned(),
        ))?
        .into_row()
}

pub async fn get_graph_proposal(
    storage: &SurrealStorage,
    proposal_id: &str,
) -> StorageResult<Option<GraphMutationProposalRow>> {
    let bindings = ProposalIdBindings {
        proposal_id: proposal_id.to_owned(),
    };
    let record: Option<GraphProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM knowledge_crdt_graph_proposals \
                         WHERE proposal_id = $proposal_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(GraphProposalRecord::into_row).transpose()
}

pub async fn list_graph_proposals_by_state(
    storage: &SurrealStorage,
    workspace_id: &str,
    review_state: &str,
) -> StorageResult<Vec<GraphMutationProposalRow>> {
    let bindings = ProposalStateBindings {
        workspace_id: workspace_id.to_owned(),
        review_state: review_state.to_owned(),
    };
    let records: Vec<GraphProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM knowledge_crdt_graph_proposals \
                         WHERE workspace_id = $workspace_id AND review_state = $review_state \
                         ORDER BY created_at_utc ASC, proposal_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(GraphProposalRecord::into_row)
        .collect()
}

/// Atomic review decision: proposed -> approved|rejected. Returns None when
/// the proposal is not in 'proposed' (no lost-update double decisions).
pub async fn decide_graph_proposal(
    storage: &SurrealStorage,
    proposal_id: &str,
    new_state: &str,
    decided_by: &str,
    decision_reason: &str,
    decided_event_id: &str,
) -> StorageResult<Option<GraphMutationProposalRow>> {
    if !matches!(new_state, "approved" | "rejected") {
        return Err(StorageError::Validation(
            "graph proposal decision must be approved or rejected",
        ));
    }
    // The `review_state = 'proposed'` guard stays inside the statement, so a
    // second decision matches nothing and returns `None` - the same
    // no-lost-update contract, enforced by the store rather than by a re-read.
    let bindings = ProposalDecisionBindings {
        proposal_id: proposal_id.to_owned(),
        review_state: new_state.to_owned(),
        decided_by: decided_by.to_owned(),
        decision_reason: decision_reason.to_owned(),
        decided_event_id: link(KERNEL_EVENT_LEDGER_TABLE, decided_event_id),
    };
    let records: Vec<GraphProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "UPDATE knowledge_crdt_graph_proposals SET review_state = $review_state, \
                         decided_by = $decided_by, decided_at_utc = time::now(), \
                         decision_reason = $decision_reason, \
                         decided_event_id = $decided_event_id \
                         WHERE proposal_id = $proposal_id AND review_state = 'proposed' \
                         RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .next()
        .map(GraphProposalRecord::into_row)
        .transpose()
}

/// approved -> promoted (MT-069 bridge finalization).
pub async fn mark_graph_proposal_promoted(
    storage: &SurrealStorage,
    proposal_id: &str,
) -> StorageResult<Option<GraphMutationProposalRow>> {
    let bindings = ProposalIdBindings {
        proposal_id: proposal_id.to_owned(),
    };
    let records: Vec<GraphProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "UPDATE knowledge_crdt_graph_proposals SET review_state = 'promoted' \
                         WHERE proposal_id = $proposal_id AND review_state = 'approved' \
                         RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .next()
        .map(GraphProposalRecord::into_row)
        .transpose()
}

// ---------------------------------------------------------------------------
// MT-069 promotion span-evidence validation (authority-hardening #1).
//
// Spec 02-system-architecture.md 2.3.13.11: "KnowledgeClaim ... Claims MUST
// carry ... evidence spans". A graph proposal may cite soft `pending:<...>`
// markers or `KSP-` ids that a later re-index retires, so the proposal table
// only CHECKs non-emptiness (0152). The promotion bridge (MT-069) is the
// authority gate: before a proposal becomes a durable `authority` fact every
// cited span MUST resolve to a real, live, same-workspace `knowledge_spans`
// row. This module is the resolver; `claim_promotion.rs` denies on any
// failure and migration 0190 is the schema backstop (a fact may only carry
// KSP- refs that exist in the same workspace and whose source is not stale).
// ---------------------------------------------------------------------------

/// One cited span ref classified for promotion-time validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromotionSpanRefKind {
    /// A concrete `KSP-<32hex>` id that must resolve against knowledge_spans.
    KnowledgeSpan,
    /// A `pending:<source>:<range>` soft marker — never durable authority.
    PendingMarker,
    /// Anything else (malformed / unknown scheme) — never durable authority.
    Unrecognized,
}

/// Classify a single span ref string for promotion validation.
pub fn classify_promotion_span_ref(span_ref: &str) -> PromotionSpanRefKind {
    let trimmed = span_ref.trim();
    if trimmed.starts_with("pending:") {
        PromotionSpanRefKind::PendingMarker
    } else if is_canonical_ksp_id(trimmed) {
        PromotionSpanRefKind::KnowledgeSpan
    } else {
        PromotionSpanRefKind::Unrecognized
    }
}

/// `KSP-` followed by exactly 32 lowercase hex chars (mirrors the
/// `chk_knowledge_spans_id` CHECK in migration 0134).
fn is_canonical_ksp_id(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("KSP-") else {
        return false;
    };
    hex.len() == 32
        && hex
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Why a single cited span ref is not promotable into authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum PromotionSpanRejection {
    /// A `pending:<...>` soft marker reached the authority gate; it must be
    /// resolved to a real span (re-index) before promotion.
    PendingMarker { span_ref: String },
    /// Malformed / unknown span-ref scheme.
    Unrecognized { span_ref: String },
    /// A `KSP-` id that does not exist in knowledge_spans at all.
    SpanNotFound { span_ref: String },
    /// The span exists but belongs to a different workspace than the proposal
    /// (cross-workspace evidence leak).
    SpanForeignWorkspace {
        span_ref: String,
        span_workspace_id: String,
        proposal_workspace_id: String,
    },
    /// The span's source has been superseded by a newer index run
    /// (`knowledge_sources.stale = true`): retired evidence.
    SpanRetired { span_ref: String },
}

/// Outcome of validating every cited span ref of a proposal against the live
/// span graph. `Ok` carries the de-duplicated, validated `KSP-` ids that may
/// be frozen into the authority fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromotionSpanValidation {
    Ok {
        validated_span_ids: Vec<String>,
    },
    Rejected {
        rejections: Vec<PromotionSpanRejection>,
    },
}

/// Resolve + validate every cited span ref of a proposal before it becomes
/// durable authority. A ref is promotable only when it is a canonical `KSP-`
/// id whose `knowledge_spans` row (a) exists, (b) is anchored to a
/// `knowledge_sources` row in the SAME `workspace_id`, and (c) whose source
/// is not `stale` (not superseded by a newer index run). `pending:` markers,
/// malformed refs, missing/foreign/retired spans all reject — the caller
/// turns any rejection into a durable promotion-denial receipt.
pub async fn validate_promotion_span_refs(
    storage: &SurrealStorage,
    proposal_workspace_id: &str,
    span_refs: &[String],
) -> StorageResult<PromotionSpanValidation> {
    let mut rejections = Vec::new();
    let mut validated: Vec<String> = Vec::new();

    for span_ref in span_refs {
        match classify_promotion_span_ref(span_ref) {
            PromotionSpanRefKind::PendingMarker => {
                rejections.push(PromotionSpanRejection::PendingMarker {
                    span_ref: span_ref.clone(),
                });
            }
            PromotionSpanRefKind::Unrecognized => {
                rejections.push(PromotionSpanRejection::Unrecognized {
                    span_ref: span_ref.clone(),
                });
            }
            PromotionSpanRefKind::KnowledgeSpan => {
                let span_id = span_ref.trim().to_string();
                // Resolve span -> source -> workspace + stale in one query. The
                // relational JOIN becomes a link traversal: `source_id` IS the
                // source record, so `source_id.workspace_id` reaches the owning
                // workspace without a join key.
                let bindings = SpanIdBindings {
                    span_id: span_id.clone(),
                };
                let row: Option<SpanOwnerRow> = storage
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .query_first(
                                    "SELECT source_id.workspace_id AS workspace_id, \
                                     source_id.stale AS stale FROM knowledge_spans \
                                     WHERE span_id = $span_id;",
                                    bindings,
                                )
                                .await
                        })
                    })
                    .await
                    .map_err(map_err)?;
                match row {
                    None => {
                        rejections.push(PromotionSpanRejection::SpanNotFound { span_ref: span_id })
                    }
                    Some(row) => {
                        let span_workspace_id: String = record_key(
                            row.workspace_id,
                            "knowledge source workspace link is not a string key",
                        )?;
                        let stale: bool = row.stale;
                        if span_workspace_id != proposal_workspace_id {
                            rejections.push(PromotionSpanRejection::SpanForeignWorkspace {
                                span_ref: span_id,
                                span_workspace_id,
                                proposal_workspace_id: proposal_workspace_id.to_string(),
                            });
                        } else if stale {
                            rejections
                                .push(PromotionSpanRejection::SpanRetired { span_ref: span_id });
                        } else if !validated.contains(&span_id) {
                            validated.push(span_id);
                        }
                    }
                }
            }
        }
    }

    if rejections.is_empty() {
        Ok(PromotionSpanValidation::Ok {
            validated_span_ids: validated,
        })
    } else {
        Ok(PromotionSpanValidation::Rejected { rejections })
    }
}

// ---------------------------------------------------------------------------
// MT-069 promoted facts.
// ---------------------------------------------------------------------------

#[derive(SurrealValue)]
struct SpanIdBindings {
    span_id: String,
}

#[derive(SurrealValue)]
struct SpanOwnerRow {
    workspace_id: RecordId,
    stale: bool,
}

/// One promoted fact (row of `knowledge_crdt_promoted_facts`, authority).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PromotedFactRow {
    pub fact_id: String,
    pub proposal_id: String,
    pub workspace_id: String,
    pub mutation_kind: String,
    pub fact_payload: Value,
    pub source_span_refs: Value,
    pub confidence: f64,
    pub proposed_by: String,
    pub promoted_by: String,
    pub promotion_requested_event_id: String,
    pub promotion_accepted_event_id: String,
    pub promoted_at_utc: DateTime<Utc>,
}

/// Stored `knowledge_crdt_promoted_facts` projection. `pub(crate)` so the
/// atomic-promotion helper can decode the row it re-selects.
#[derive(SurrealValue)]
pub(crate) struct PromotedFactRecord {
    fact_id: String,
    proposal_id: RecordId,
    workspace_id: String,
    mutation_kind: String,
    fact_payload: Value,
    source_span_refs: Vec<String>,
    confidence: f64,
    proposed_by: String,
    promoted_by: String,
    promotion_requested_event_id: RecordId,
    promotion_accepted_event_id: RecordId,
    promoted_at_utc: Datetime,
}

impl PromotedFactRecord {
    pub(crate) fn into_row(self) -> StorageResult<PromotedFactRow> {
        Ok(PromotedFactRow {
            fact_id: self.fact_id,
            proposal_id: record_key(
                self.proposal_id,
                "promoted fact proposal link is not a string key",
            )?,
            workspace_id: self.workspace_id,
            mutation_kind: self.mutation_kind,
            fact_payload: self.fact_payload,
            source_span_refs: Value::from(self.source_span_refs),
            confidence: self.confidence,
            proposed_by: self.proposed_by,
            promoted_by: self.promoted_by,
            promotion_requested_event_id: record_key(
                self.promotion_requested_event_id,
                "promoted fact promotion-requested link is not a string key",
            )?,
            promotion_accepted_event_id: record_key(
                self.promotion_accepted_event_id,
                "promoted fact promotion-accepted link is not a string key",
            )?,
            promoted_at_utc: self.promoted_at_utc.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct PromotedFactCreate {
    fact_id: String,
    proposal_id: RecordId,
    workspace_id: String,
    mutation_kind: String,
    fact_payload: Value,
    source_span_refs: Vec<String>,
    confidence: f64,
    proposed_by: String,
    promoted_by: String,
    promotion_requested_event_id: RecordId,
    promotion_accepted_event_id: RecordId,
}

#[derive(SurrealValue)]
struct PromotedFactProposalBindings {
    proposal: RecordId,
}

/// `source_span_refs` reaches this module as opaque JSON on the public
/// `NewPromotedFact`; the store types the column as `array<string>`, so the
/// JSON is decoded here rather than being written as an untyped blob. A
/// non-string-array value is refused instead of silently degrading the
/// evidence the 0190 span-evidence guard depends on.
pub(crate) fn span_refs_from_json(value: &Value) -> StorageResult<Vec<String>> {
    let array = value.as_array().ok_or(StorageError::Validation(
        "promoted fact source_span_refs must be a JSON array",
    ))?;
    array
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or(StorageError::Validation(
                    "promoted fact source_span_refs must contain only strings",
                ))
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct NewPromotedFact {
    pub fact_id: String,
    pub proposal_id: String,
    pub workspace_id: String,
    pub mutation_kind: String,
    pub fact_payload: Value,
    pub source_span_refs: Value,
    pub confidence: f64,
    pub proposed_by: String,
    pub promoted_by: String,
    pub promotion_requested_event_id: String,
    pub promotion_accepted_event_id: String,
}

/// Insert a promoted fact; idempotent on proposal_id (re-promotion returns
/// the existing fact row untouched).
pub async fn insert_promoted_fact_idempotent(
    storage: &SurrealStorage,
    fact: NewPromotedFact,
) -> StorageResult<PromotedFactRow> {
    // Idempotency on `proposal_id` is enforced inside one statement: the
    // existence probe and the create cannot interleave, so a re-promotion
    // returns the existing fact untouched instead of racing a second insert
    // against `uq_knowledge_crdt_promoted_facts_proposal`.
    let content = PromotedFactCreate {
        fact_id: fact.fact_id.clone(),
        proposal_id: link(GRAPH_PROPOSALS_TABLE, &fact.proposal_id),
        workspace_id: fact.workspace_id.clone(),
        mutation_kind: fact.mutation_kind.clone(),
        fact_payload: fact.fact_payload.clone(),
        source_span_refs: span_refs_from_json(&fact.source_span_refs)?,
        confidence: fact.confidence,
        proposed_by: fact.proposed_by.clone(),
        promoted_by: fact.promoted_by.clone(),
        promotion_requested_event_id: link(
            KERNEL_EVENT_LEDGER_TABLE,
            &fact.promotion_requested_event_id,
        ),
        promotion_accepted_event_id: link(
            KERNEL_EVENT_LEDGER_TABLE,
            &fact.promotion_accepted_event_id,
        ),
    };
    let rows: Vec<PromotedFactRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(PROMOTED_FACT_UPSERT_STATEMENT, content)
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::NotFound("promoted fact after insert"))?
        .into_row()
}

/// Insert-if-absent for `knowledge_crdt_promoted_facts`, keyed on the natural
/// `proposal_id`. `pub(crate)` so the atomic-promotion helper reuses the exact
/// same statement rather than re-deriving one that could drift.
pub(crate) const PROMOTED_FACT_UPSERT_STATEMENT: &str = "{ \
     LET $existing = (SELECT * FROM knowledge_crdt_promoted_facts \
       WHERE proposal_id = $proposal_id); \
     IF array::len($existing) > 0 { \
       RETURN $existing \
     } ELSE { \
       RETURN CREATE type::record('knowledge_crdt_promoted_facts', $fact_id) \
         CONTENT { fact_id: $fact_id, proposal_id: $proposal_id, \
           workspace_id: $workspace_id, mutation_kind: $mutation_kind, \
           fact_payload: $fact_payload, source_span_refs: $source_span_refs, \
           confidence: $confidence, proposed_by: $proposed_by, \
           promoted_by: $promoted_by, \
           promotion_requested_event_id: $promotion_requested_event_id, \
           promotion_accepted_event_id: $promotion_accepted_event_id } \
     }; \
   };";

pub async fn get_promoted_fact_by_proposal(
    storage: &SurrealStorage,
    proposal_id: &str,
) -> StorageResult<Option<PromotedFactRow>> {
    let bindings = PromotedFactProposalBindings {
        proposal: link(GRAPH_PROPOSALS_TABLE, proposal_id),
    };
    let record: Option<PromotedFactRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM knowledge_crdt_promoted_facts \
                         WHERE proposal_id = $proposal;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(PromotedFactRecord::into_row).transpose()
}

/// Authority-hardening #2 (recovery/replay branch): materialize the promoted
/// fact for a proposal whose EventLedger promotion pair already exists but
/// whose fact row never landed (the historical crash window). Inserts the
/// fact (carrying the already-committed event ids) and flips the proposal to
/// 'promoted', in ONE transaction, idempotent on `proposal_id`. Does NOT
/// append events — the pair is already durable, so passive replay converges
/// without re-appending (which the EventLedger would reject as an idempotency
/// conflict). The 0190 span-evidence trigger still guards the insert.
pub async fn materialize_promoted_fact_from_existing_events(
    storage: &SurrealStorage,
    fact: NewPromotedFact,
) -> StorageResult<PromotedFactRow> {
    // Fact insert, proposal flip and read-back stay in ONE transaction, exactly
    // as the relational version did: a block statement is the embedded store's
    // implicit transaction boundary, so a crash can never leave the fact
    // durable with the proposal still 'approved'. Idempotent on `proposal_id`.
    let content = PromotedFactCreate {
        fact_id: fact.fact_id.clone(),
        proposal_id: link(GRAPH_PROPOSALS_TABLE, &fact.proposal_id),
        workspace_id: fact.workspace_id.clone(),
        mutation_kind: fact.mutation_kind.clone(),
        fact_payload: fact.fact_payload.clone(),
        source_span_refs: span_refs_from_json(&fact.source_span_refs)?,
        confidence: fact.confidence,
        proposed_by: fact.proposed_by.clone(),
        promoted_by: fact.promoted_by.clone(),
        promotion_requested_event_id: link(
            KERNEL_EVENT_LEDGER_TABLE,
            &fact.promotion_requested_event_id,
        ),
        promotion_accepted_event_id: link(
            KERNEL_EVENT_LEDGER_TABLE,
            &fact.promotion_accepted_event_id,
        ),
    };
    let rows: Vec<PromotedFactRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "{ \
                           LET $existing = (SELECT * FROM knowledge_crdt_promoted_facts \
                             WHERE proposal_id = $proposal_id); \
                           IF array::len($existing) = 0 { \
                             CREATE type::record('knowledge_crdt_promoted_facts', $fact_id) \
                               CONTENT { fact_id: $fact_id, proposal_id: $proposal_id, \
                                 workspace_id: $workspace_id, mutation_kind: $mutation_kind, \
                                 fact_payload: $fact_payload, \
                                 source_span_refs: $source_span_refs, \
                                 confidence: $confidence, proposed_by: $proposed_by, \
                                 promoted_by: $promoted_by, \
                                 promotion_requested_event_id: \
                                   $promotion_requested_event_id, \
                                 promotion_accepted_event_id: \
                                   $promotion_accepted_event_id }; \
                           }; \
                           UPDATE knowledge_crdt_graph_proposals SET review_state = 'promoted' \
                             WHERE id = $proposal_id AND review_state = 'approved'; \
                           RETURN SELECT * FROM knowledge_crdt_promoted_facts \
                             WHERE proposal_id = $proposal_id; \
                         };",
                        content,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::NotFound("promoted fact after materialize"))?
        .into_row()
}

// ---------------------------------------------------------------------------
// MT-074 AI edit proposals.
// ---------------------------------------------------------------------------

/// One AI edit proposal (row of `knowledge_crdt_ai_edit_proposals`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AiEditProposalRow {
    pub proposal_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub base_update_seq: i64,
    pub base_state_vector: String,
    pub proposed_diff: Value,
    pub diff_sha256: String,
    pub source_span_citations: Value,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub lease_id: Option<String>,
    pub review_state: String,
    pub decided_by: Option<String>,
    pub decided_at_utc: Option<DateTime<Utc>>,
    pub decision_reason: Option<String>,
    pub recorded_event_id: String,
    pub decided_event_id: Option<String>,
    pub promotion_requested_event_id: Option<String>,
    pub promotion_accepted_event_id: Option<String>,
    /// Authority-hardening #5: the applied update bound to the approved diff
    /// (set only when the applied content hashed to `diff_sha256`).
    pub applied_update_id: Option<String>,
    pub applied_update_sha256: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

/// Stored `knowledge_crdt_ai_edit_proposals` projection.
#[derive(SurrealValue)]
struct AiEditProposalRecord {
    proposal_id: String,
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
    base_update_seq: i64,
    base_state_vector: String,
    proposed_diff: Value,
    diff_sha256: String,
    source_span_citations: Value,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    correlation_id: String,
    lease_id: Option<RecordId>,
    review_state: String,
    decided_by: Option<String>,
    decided_at_utc: Option<Datetime>,
    decision_reason: Option<String>,
    recorded_event_id: RecordId,
    decided_event_id: Option<RecordId>,
    promotion_requested_event_id: Option<RecordId>,
    promotion_accepted_event_id: Option<RecordId>,
    applied_update_id: Option<String>,
    applied_update_sha256: Option<String>,
    created_at_utc: Datetime,
}

impl AiEditProposalRecord {
    fn into_row(self) -> StorageResult<AiEditProposalRow> {
        Ok(AiEditProposalRow {
            proposal_id: self.proposal_id,
            workspace_id: self.workspace_id,
            document_id: self.document_id,
            crdt_document_id: self.crdt_document_id,
            base_update_seq: self.base_update_seq,
            base_state_vector: self.base_state_vector,
            proposed_diff: self.proposed_diff,
            diff_sha256: self.diff_sha256,
            source_span_citations: self.source_span_citations,
            actor_id: self.actor_id,
            actor_kind: self.actor_kind,
            session_id: self.session_id,
            correlation_id: self.correlation_id,
            lease_id: optional_record_key(
                self.lease_id,
                "AI edit proposal lease link is not a string key",
            )?,
            review_state: self.review_state,
            decided_by: self.decided_by,
            decided_at_utc: self.decided_at_utc.map(Datetime::into_inner),
            decision_reason: self.decision_reason,
            recorded_event_id: record_key(
                self.recorded_event_id,
                "AI edit proposal recorded-event link is not a string key",
            )?,
            decided_event_id: optional_record_key(
                self.decided_event_id,
                "AI edit proposal decided-event link is not a string key",
            )?,
            promotion_requested_event_id: optional_record_key(
                self.promotion_requested_event_id,
                "AI edit proposal promotion-requested link is not a string key",
            )?,
            promotion_accepted_event_id: optional_record_key(
                self.promotion_accepted_event_id,
                "AI edit proposal promotion-accepted link is not a string key",
            )?,
            applied_update_id: self.applied_update_id,
            applied_update_sha256: self.applied_update_sha256,
            created_at_utc: self.created_at_utc.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct AiEditProposalCreate {
    proposal_id: String,
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
    base_update_seq: i64,
    base_state_vector: String,
    proposed_diff: Value,
    diff_sha256: String,
    source_span_citations: Vec<String>,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    correlation_id: String,
    lease_id: Option<RecordId>,
    recorded_event_id: RecordId,
}

#[derive(SurrealValue)]
struct AiEditDocumentBindings {
    crdt_document_id: String,
    review_state: Option<String>,
}

#[derive(SurrealValue)]
struct AiEditPromoteBindings {
    proposal_id: String,
    promotion_requested_event_id: RecordId,
    promotion_accepted_event_id: RecordId,
}

#[derive(SurrealValue)]
struct AiEditBindBindings {
    proposal_id: String,
    applied_update_id: String,
    applied_content_sha256: String,
}

#[derive(SurrealValue)]
struct CrdtUpdateLookupBindings {
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
    update_id: String,
}

#[derive(SurrealValue)]
struct CrdtUpdateShaRow {
    update_sha256: String,
}

#[derive(Clone, Debug)]
pub struct NewAiEditProposal {
    pub proposal_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub base_update_seq: i64,
    pub base_state_vector: String,
    pub proposed_diff: Value,
    pub diff_sha256: String,
    pub source_span_citations: Vec<String>,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub lease_id: Option<String>,
    pub recorded_event_id: String,
}

pub async fn insert_ai_edit_proposal(
    storage: &SurrealStorage,
    proposal: NewAiEditProposal,
) -> StorageResult<AiEditProposalRow> {
    if proposal.source_span_citations.is_empty()
        || proposal
            .source_span_citations
            .iter()
            .any(|span| span.trim().is_empty())
    {
        return Err(StorageError::Validation(
            "AI edit proposal requires at least one non-empty source span citation",
        ));
    }
    let content = AiEditProposalCreate {
        proposal_id: proposal.proposal_id.clone(),
        workspace_id: proposal.workspace_id.clone(),
        document_id: proposal.document_id.clone(),
        crdt_document_id: proposal.crdt_document_id.clone(),
        base_update_seq: proposal.base_update_seq,
        base_state_vector: proposal.base_state_vector.clone(),
        proposed_diff: proposal.proposed_diff.clone(),
        diff_sha256: proposal.diff_sha256.clone(),
        source_span_citations: proposal.source_span_citations.clone(),
        actor_id: proposal.actor_id.clone(),
        actor_kind: proposal.actor_kind.clone(),
        session_id: proposal.session_id.clone(),
        correlation_id: proposal.correlation_id.clone(),
        lease_id: optional_link(LEASES_TABLE, proposal.lease_id.as_deref()),
        recorded_event_id: link(KERNEL_EVENT_LEDGER_TABLE, &proposal.recorded_event_id),
    };
    let rows: Vec<AiEditProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "CREATE type::record('knowledge_crdt_ai_edit_proposals', $proposal_id) \
                         CONTENT { proposal_id: $proposal_id, workspace_id: $workspace_id, \
                           document_id: $document_id, crdt_document_id: $crdt_document_id, \
                           base_update_seq: $base_update_seq, \
                           base_state_vector: $base_state_vector, \
                           proposed_diff: $proposed_diff, diff_sha256: $diff_sha256, \
                           source_span_citations: $source_span_citations, \
                           actor_id: $actor_id, actor_kind: $actor_kind, \
                           session_id: $session_id, correlation_id: $correlation_id, \
                           lease_id: $lease_id, recorded_event_id: $recorded_event_id };",
                        content,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "AI edit proposal insert produced no row".to_owned(),
        ))?
        .into_row()
}

pub async fn get_ai_edit_proposal(
    storage: &SurrealStorage,
    proposal_id: &str,
) -> StorageResult<Option<AiEditProposalRow>> {
    let bindings = ProposalIdBindings {
        proposal_id: proposal_id.to_owned(),
    };
    let record: Option<AiEditProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM knowledge_crdt_ai_edit_proposals \
                         WHERE proposal_id = $proposal_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(AiEditProposalRecord::into_row).transpose()
}

pub async fn list_ai_edit_proposals_for_document(
    storage: &SurrealStorage,
    crdt_document_id: &str,
    review_state: Option<&str>,
) -> StorageResult<Vec<AiEditProposalRow>> {
    // The two relational statements collapse into one: `NONE` is the embedded
    // store's "no filter" value, so the optional review-state predicate no
    // longer needs a second hand-written query that can drift from the first.
    let bindings = AiEditDocumentBindings {
        crdt_document_id: crdt_document_id.to_owned(),
        review_state: review_state.map(ToOwned::to_owned),
    };
    let records: Vec<AiEditProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM knowledge_crdt_ai_edit_proposals \
                         WHERE crdt_document_id = $crdt_document_id \
                         AND ($review_state = NONE OR review_state = $review_state) \
                         ORDER BY created_at_utc ASC, proposal_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(AiEditProposalRecord::into_row)
        .collect()
}

/// Atomic review decision: proposed -> approved|rejected (no lost updates).
pub async fn decide_ai_edit_proposal(
    storage: &SurrealStorage,
    proposal_id: &str,
    new_state: &str,
    decided_by: &str,
    decision_reason: &str,
    decided_event_id: &str,
) -> StorageResult<Option<AiEditProposalRow>> {
    if !matches!(new_state, "approved" | "rejected") {
        return Err(StorageError::Validation(
            "AI edit proposal decision must be approved or rejected",
        ));
    }
    let bindings = ProposalDecisionBindings {
        proposal_id: proposal_id.to_owned(),
        review_state: new_state.to_owned(),
        decided_by: decided_by.to_owned(),
        decision_reason: decision_reason.to_owned(),
        decided_event_id: link(KERNEL_EVENT_LEDGER_TABLE, decided_event_id),
    };
    let records: Vec<AiEditProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "UPDATE knowledge_crdt_ai_edit_proposals \
                         SET review_state = $review_state, decided_by = $decided_by, \
                             decided_at_utc = time::now(), \
                             decision_reason = $decision_reason, \
                             decided_event_id = $decided_event_id \
                         WHERE proposal_id = $proposal_id AND review_state = 'proposed' \
                         RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .next()
        .map(AiEditProposalRecord::into_row)
        .transpose()
}

/// approved -> promoted with the EventLedger promotion pair (atomic guard).
pub async fn mark_ai_edit_proposal_promoted(
    storage: &SurrealStorage,
    proposal_id: &str,
    promotion_requested_event_id: &str,
    promotion_accepted_event_id: &str,
) -> StorageResult<Option<AiEditProposalRow>> {
    let bindings = AiEditPromoteBindings {
        proposal_id: proposal_id.to_owned(),
        promotion_requested_event_id: link(KERNEL_EVENT_LEDGER_TABLE, promotion_requested_event_id),
        promotion_accepted_event_id: link(KERNEL_EVENT_LEDGER_TABLE, promotion_accepted_event_id),
    };
    let records: Vec<AiEditProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "UPDATE knowledge_crdt_ai_edit_proposals SET review_state = 'promoted', \
                         promotion_requested_event_id = $promotion_requested_event_id, \
                         promotion_accepted_event_id = $promotion_accepted_event_id \
                         WHERE proposal_id = $proposal_id AND review_state = 'approved' \
                         RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .next()
        .map(AiEditProposalRecord::into_row)
        .transpose()
}

/// Authority-hardening #5: bind the applied update to the approved diff. Sets
/// `applied_update_id` + `applied_update_sha256` ONLY when the proposal is
/// approved/promoted AND `applied_content_sha256` EQUALS the approved
/// `diff_sha256` (the WHERE clause enforces the hash match server-side, and
/// the 0192 CHECK is the schema backstop). Returns `None` when no row matched
/// — i.e. the hash did not match the approved diff, the proposal is not in an
/// applicable state, or it does not exist — so the caller can emit a durable
/// `ai_edit_applied_mismatch` denial. Idempotent: re-binding the same update
/// id + hash is a no-op that still returns the row.
pub async fn bind_applied_ai_edit_update(
    storage: &SurrealStorage,
    proposal_id: &str,
    applied_update_id: &str,
    applied_content_sha256: &str,
) -> StorageResult<Option<AiEditProposalRow>> {
    // Every guard stays server-side, including the hash equality: the binding
    // is refused by the WHERE clause when the applied content does not hash to
    // the approved `diff_sha256`, so a mismatch still returns `None` for the
    // caller to turn into a durable denial rather than being decided in Rust.
    let bindings = AiEditBindBindings {
        proposal_id: proposal_id.to_owned(),
        applied_update_id: applied_update_id.to_owned(),
        applied_content_sha256: applied_content_sha256.to_owned(),
    };
    let records: Vec<AiEditProposalRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "UPDATE knowledge_crdt_ai_edit_proposals \
                         SET applied_update_id = $applied_update_id, \
                             applied_update_sha256 = $applied_content_sha256 \
                         WHERE proposal_id = $proposal_id \
                           AND review_state IN ['approved', 'promoted'] \
                           AND diff_sha256 = $applied_content_sha256 \
                           AND (applied_update_id = NONE \
                                OR applied_update_id = $applied_update_id) \
                         RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .next()
        .map(AiEditProposalRecord::into_row)
        .transpose()
}

/// MT-074 V1 FAIL remediation: look up the real `kernel_crdt_updates` row that
/// an applied-binding claims to reference and return its persisted
/// `update_sha256`. The lookup keys on the proposal's authority identity
/// (`workspace_id`, `document_id`, `crdt_document_id`) plus the candidate
/// `applied_update_id` — exactly the `kernel_crdt_updates` PRIMARY KEY. Returns
/// `None` when NO such update row exists, so the caller refuses the binding and
/// emits a durable denial even when the diff hash matches. The hash match alone
/// is insufficient: an absent update id means there is no real document update
/// to anchor the approved edit to.
pub async fn find_applied_crdt_update_sha256(
    storage: &SurrealStorage,
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    applied_update_id: &str,
) -> StorageResult<Option<String>> {
    let bindings = CrdtUpdateLookupBindings {
        workspace_id: workspace_id.to_owned(),
        document_id: document_id.to_owned(),
        crdt_document_id: crdt_document_id.to_owned(),
        update_id: applied_update_id.to_owned(),
    };
    let row: Option<CrdtUpdateShaRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT update_sha256 FROM kernel_crdt_updates \
                         WHERE workspace_id = $workspace_id AND document_id = $document_id \
                         AND crdt_document_id = $crdt_document_id AND update_id = $update_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    Ok(row.map(|row| row.update_sha256))
}

// ---------------------------------------------------------------------------
// MT-079 swarm checkpoints + recovery receipts.
// ---------------------------------------------------------------------------

/// One swarm checkpoint (row of `knowledge_crdt_swarm_checkpoints`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SwarmCheckpointRow {
    pub checkpoint_id: String,
    pub session_id: String,
    pub actor_id: String,
    pub lane_id: String,
    pub lease_id: String,
    pub scope_ref: String,
    pub resume_pointer: Value,
    pub checkpoint_payload: Value,
    pub payload_sha256: String,
    pub recorded_event_id: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Stored `knowledge_crdt_swarm_checkpoints` projection.
#[derive(SurrealValue)]
struct SwarmCheckpointRecord {
    checkpoint_id: String,
    session_id: String,
    actor_id: String,
    lane_id: String,
    lease_id: RecordId,
    scope_ref: String,
    resume_pointer: Value,
    checkpoint_payload: Value,
    payload_sha256: String,
    recorded_event_id: RecordId,
    created_at_utc: Datetime,
}

impl SwarmCheckpointRecord {
    fn into_row(self) -> StorageResult<SwarmCheckpointRow> {
        Ok(SwarmCheckpointRow {
            checkpoint_id: self.checkpoint_id,
            session_id: self.session_id,
            actor_id: self.actor_id,
            lane_id: self.lane_id,
            lease_id: record_key(self.lease_id, "checkpoint lease link is not a string key")?,
            scope_ref: self.scope_ref,
            resume_pointer: self.resume_pointer,
            checkpoint_payload: self.checkpoint_payload,
            payload_sha256: self.payload_sha256,
            recorded_event_id: record_key(
                self.recorded_event_id,
                "checkpoint recorded-event link is not a string key",
            )?,
            created_at_utc: self.created_at_utc.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct SwarmCheckpointCreate {
    checkpoint_id: String,
    session_id: String,
    actor_id: String,
    lane_id: String,
    lease_id: RecordId,
    scope_ref: String,
    resume_pointer: Value,
    checkpoint_payload: Value,
    payload_sha256: String,
    recorded_event_id: RecordId,
}

#[derive(SurrealValue)]
struct CheckpointIdBindings {
    checkpoint_id: String,
}

#[derive(SurrealValue)]
struct LaneIdBindings {
    lane_id: String,
}

#[derive(SurrealValue)]
struct CheckpointLinkBindings {
    checkpoint: RecordId,
}

#[derive(Clone, Debug)]
pub struct NewSwarmCheckpoint {
    pub checkpoint_id: String,
    pub session_id: String,
    pub actor_id: String,
    pub lane_id: String,
    pub lease_id: String,
    pub scope_ref: String,
    pub resume_pointer: Value,
    pub checkpoint_payload: Value,
    pub payload_sha256: String,
    pub recorded_event_id: String,
}

pub async fn insert_swarm_checkpoint(
    storage: &SurrealStorage,
    checkpoint: NewSwarmCheckpoint,
) -> StorageResult<SwarmCheckpointRow> {
    let content = SwarmCheckpointCreate {
        checkpoint_id: checkpoint.checkpoint_id.clone(),
        session_id: checkpoint.session_id.clone(),
        actor_id: checkpoint.actor_id.clone(),
        lane_id: checkpoint.lane_id.clone(),
        lease_id: link(LEASES_TABLE, &checkpoint.lease_id),
        scope_ref: checkpoint.scope_ref.clone(),
        resume_pointer: checkpoint.resume_pointer.clone(),
        checkpoint_payload: checkpoint.checkpoint_payload.clone(),
        payload_sha256: checkpoint.payload_sha256.clone(),
        recorded_event_id: link(KERNEL_EVENT_LEDGER_TABLE, &checkpoint.recorded_event_id),
    };
    let rows: Vec<SwarmCheckpointRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "CREATE type::record('knowledge_crdt_swarm_checkpoints', $checkpoint_id) \
                         CONTENT { checkpoint_id: $checkpoint_id, session_id: $session_id, \
                           actor_id: $actor_id, lane_id: $lane_id, lease_id: $lease_id, \
                           scope_ref: $scope_ref, resume_pointer: $resume_pointer, \
                           checkpoint_payload: $checkpoint_payload, \
                           payload_sha256: $payload_sha256, \
                           recorded_event_id: $recorded_event_id };",
                        content,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "swarm checkpoint insert produced no row".to_owned(),
        ))?
        .into_row()
}

pub async fn get_swarm_checkpoint(
    storage: &SurrealStorage,
    checkpoint_id: &str,
) -> StorageResult<Option<SwarmCheckpointRow>> {
    let bindings = CheckpointIdBindings {
        checkpoint_id: checkpoint_id.to_owned(),
    };
    let record: Option<SwarmCheckpointRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM knowledge_crdt_swarm_checkpoints \
                         WHERE checkpoint_id = $checkpoint_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(SwarmCheckpointRecord::into_row).transpose()
}

/// Latest checkpoint for a lane (recovery entrypoint after session loss).
pub async fn latest_checkpoint_for_lane(
    storage: &SurrealStorage,
    lane_id: &str,
) -> StorageResult<Option<SwarmCheckpointRow>> {
    let bindings = LaneIdBindings {
        lane_id: lane_id.to_owned(),
    };
    let record: Option<SwarmCheckpointRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM knowledge_crdt_swarm_checkpoints WHERE lane_id = $lane_id \
                         ORDER BY created_at_utc DESC, checkpoint_id DESC LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(SwarmCheckpointRecord::into_row).transpose()
}

/// One recovery receipt (row of `knowledge_crdt_recovery_receipts`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecoveryReceiptRow {
    pub receipt_id: String,
    pub checkpoint_id: String,
    pub prior_session_id: String,
    pub new_session_id: String,
    pub new_actor_id: String,
    pub new_lease_id: String,
    pub lease_lineage: Value,
    pub resume_pointer: Value,
    pub recorded_event_id: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Stored `knowledge_crdt_recovery_receipts` projection.
#[derive(SurrealValue)]
struct RecoveryReceiptRecord {
    receipt_id: String,
    checkpoint_id: RecordId,
    prior_session_id: String,
    new_session_id: String,
    new_actor_id: String,
    new_lease_id: RecordId,
    lease_lineage: Value,
    resume_pointer: Value,
    recorded_event_id: RecordId,
    created_at_utc: Datetime,
}

impl RecoveryReceiptRecord {
    fn into_row(self) -> StorageResult<RecoveryReceiptRow> {
        Ok(RecoveryReceiptRow {
            receipt_id: self.receipt_id,
            checkpoint_id: record_key(
                self.checkpoint_id,
                "recovery receipt checkpoint link is not a string key",
            )?,
            prior_session_id: self.prior_session_id,
            new_session_id: self.new_session_id,
            new_actor_id: self.new_actor_id,
            new_lease_id: record_key(
                self.new_lease_id,
                "recovery receipt lease link is not a string key",
            )?,
            lease_lineage: self.lease_lineage,
            resume_pointer: self.resume_pointer,
            recorded_event_id: record_key(
                self.recorded_event_id,
                "recovery receipt recorded-event link is not a string key",
            )?,
            created_at_utc: self.created_at_utc.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct RecoveryReceiptCreate {
    receipt_id: String,
    checkpoint_id: RecordId,
    prior_session_id: String,
    new_session_id: String,
    new_actor_id: String,
    new_lease_id: RecordId,
    lease_lineage: Value,
    resume_pointer: Value,
    recorded_event_id: RecordId,
}

#[derive(Clone, Debug)]
pub struct NewRecoveryReceipt {
    pub receipt_id: String,
    pub checkpoint_id: String,
    pub prior_session_id: String,
    pub new_session_id: String,
    pub new_actor_id: String,
    pub new_lease_id: String,
    pub lease_lineage: Value,
    pub resume_pointer: Value,
    pub recorded_event_id: String,
}

pub async fn insert_recovery_receipt(
    storage: &SurrealStorage,
    receipt: NewRecoveryReceipt,
) -> StorageResult<RecoveryReceiptRow> {
    let content = RecoveryReceiptCreate {
        receipt_id: receipt.receipt_id.clone(),
        checkpoint_id: link(CHECKPOINTS_TABLE, &receipt.checkpoint_id),
        prior_session_id: receipt.prior_session_id.clone(),
        new_session_id: receipt.new_session_id.clone(),
        new_actor_id: receipt.new_actor_id.clone(),
        new_lease_id: link(LEASES_TABLE, &receipt.new_lease_id),
        lease_lineage: receipt.lease_lineage.clone(),
        resume_pointer: receipt.resume_pointer.clone(),
        recorded_event_id: link(KERNEL_EVENT_LEDGER_TABLE, &receipt.recorded_event_id),
    };
    let rows: Vec<RecoveryReceiptRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "CREATE type::record('knowledge_crdt_recovery_receipts', $receipt_id) \
                         CONTENT { receipt_id: $receipt_id, checkpoint_id: $checkpoint_id, \
                           prior_session_id: $prior_session_id, \
                           new_session_id: $new_session_id, new_actor_id: $new_actor_id, \
                           new_lease_id: $new_lease_id, lease_lineage: $lease_lineage, \
                           resume_pointer: $resume_pointer, \
                           recorded_event_id: $recorded_event_id };",
                        content,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "recovery receipt insert produced no row".to_owned(),
        ))?
        .into_row()
}

pub async fn list_recovery_receipts_for_checkpoint(
    storage: &SurrealStorage,
    checkpoint_id: &str,
) -> StorageResult<Vec<RecoveryReceiptRow>> {
    let bindings = CheckpointLinkBindings {
        checkpoint: link(CHECKPOINTS_TABLE, checkpoint_id),
    };
    let records: Vec<RecoveryReceiptRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM knowledge_crdt_recovery_receipts \
                         WHERE checkpoint_id = $checkpoint \
                         ORDER BY created_at_utc ASC, receipt_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(RecoveryReceiptRecord::into_row)
        .collect()
}

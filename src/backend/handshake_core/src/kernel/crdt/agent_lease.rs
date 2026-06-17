//! WP-KERNEL-009 MT-076 CRDTAndConcurrencyCore-076-AgentLeaseExpiration.
//!
//! Implements the MT-041 `swarm_lease_checkpoint_contract_seed` AgentLaneLease
//! semantics on PostgreSQL/EventLedger:
//!   * claim / renew / release / expire / takeover transitions;
//!   * server-side expiry (database clock, never the client clock);
//!   * typed denial receipt + EventLedger event for writes under an
//!     expired/foreign/released lease;
//!   * every transition appends an EventLedger event with an idempotency key.
//!
//! Lease ids are ULIDs (Crockford base32, 26 chars, 48-bit ms timestamp +
//! 80-bit OS-CSPRNG randomness) per the seed's `lease_id (ULID)` requirement;
//! the encoder lives here (pure Rust, no new dependency).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::knowledge_crdt::{
    self, AgentLaneLeaseRow, LeaseInsertOutcome, LeaseTakeoverFailure, NewAgentLaneLease,
    NewKnowledgeCrdtDenialReceipt, insert_denial_receipt, new_denial_receipt_id,
};
use crate::storage::{Database, StorageError};

use super::actor_site::KnowledgeActorIdV1;

pub const AGENT_LANE_LEASE_SCHEMA_ID: &str = "hsk.kernel.knowledge_agent_lane_lease@1";
pub const LEASE_WRITE_DENIAL_SCHEMA_ID: &str = "hsk.kernel.knowledge_lease_write_denial@1";

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Encode a 128-bit value as a 26-character Crockford base32 ULID string.
fn encode_ulid(value: u128) -> String {
    let mut out = String::with_capacity(26);
    for index in 0..26 {
        let shift = 125usize.saturating_sub(5 * index);
        let bits = ((value >> shift) & 0x1F) as usize;
        out.push(CROCKFORD[bits] as char);
    }
    out
}

/// Generate a new ULID: 48-bit unix-ms timestamp + 80-bit CSPRNG randomness.
pub fn new_ulid() -> String {
    let now_ms = Utc::now().timestamp_millis().max(0) as u128;
    let mut random = [0u8; 10];
    getrandom::getrandom(&mut random).expect("operating-system CSPRNG available");
    let mut random_bits: u128 = 0;
    for byte in random {
        random_bits = (random_bits << 8) | byte as u128;
    }
    encode_ulid(((now_ms & 0xFFFF_FFFF_FFFF) << 80) | random_bits)
}

/// Build a ULID at an exact timestamp (deterministic-ordering tests).
pub fn ulid_at(now_ms: u64, random_bits: u128) -> String {
    encode_ulid(((now_ms as u128 & 0xFFFF_FFFF_FFFF) << 80) | (random_bits & ((1 << 80) - 1)))
}

/// Typed lease scopes (MT-041 seed: workspace|document|source_root|index_run).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeLeaseScopeKind {
    Workspace,
    Document,
    SourceRoot,
    IndexRun,
}

impl KnowledgeLeaseScopeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Document => "document",
            Self::SourceRoot => "source_root",
            Self::IndexRun => "index_run",
        }
    }
}

/// Claim request for a lane lease.
#[derive(Debug, Clone)]
pub struct LeaseClaimRequestV1 {
    pub lane_id: String,
    pub actor: KnowledgeActorIdV1,
    pub session_id: String,
    pub correlation_id: String,
    pub scope_kind: KnowledgeLeaseScopeKind,
    pub scope_id: String,
    pub ttl_seconds: i64,
}

/// Typed claim outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseClaimOutcomeV1 {
    Claimed(Box<AgentLaneLeaseRow>),
    /// Another unreleased lease holds the scope. `holder_expired` says
    /// whether a takeover is possible right now (database clock).
    ScopeHeld {
        holder: Box<AgentLaneLeaseRow>,
        holder_expired: bool,
    },
}

#[derive(Debug)]
pub enum LeaseFlowError {
    Storage(StorageError),
    Event(String),
}

impl std::fmt::Display for LeaseFlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(error) => write!(f, "lease storage failure: {error}"),
            Self::Event(message) => write!(f, "lease event failure: {message}"),
        }
    }
}

impl std::error::Error for LeaseFlowError {}

impl From<StorageError> for LeaseFlowError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

async fn append_lease_event(
    db: &(dyn Database + '_),
    lease: &AgentLaneLeaseRow,
    event_type: KernelEventType,
    idempotency_key: String,
    payload: serde_json::Value,
) -> Result<String, LeaseFlowError> {
    let actor = KnowledgeActorIdV1::parse(&lease.actor_id)
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let event = NewKernelEvent::builder(
        format!("KTR-KNOWLEDGE-LEASE-{}", lease.lease_id),
        lease.session_id.clone(),
        event_type,
        actor.to_kernel_actor(),
    )
    .aggregate("knowledge_agent_lease", lease.lease_id.clone())
    .idempotency_key(idempotency_key)
    .correlation_id(lease.correlation_id.clone())
    .source_component("knowledge_crdt_agent_lease")
    .payload(payload)
    .build()
    .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    let stored = db
        .append_kernel_event(event)
        .await
        .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
    Ok(stored.event_id)
}

fn lease_payload(lease: &AgentLaneLeaseRow, transition: &str) -> serde_json::Value {
    json!({
        "schema_id": AGENT_LANE_LEASE_SCHEMA_ID,
        "transition": transition,
        "lease_id": lease.lease_id,
        "lane_id": lease.lane_id,
        "actor_id": lease.actor_id,
        "actor_kind": lease.actor_kind,
        "session_id": lease.session_id,
        "correlation_id": lease.correlation_id,
        "scope_ref": lease.scope_ref(),
        "claimed_at_utc": lease.claimed_at_utc.to_rfc3339(),
        "expires_at_utc": lease.expires_at_utc.to_rfc3339(),
        "renewal_count": lease.renewal_count,
        "released_at_utc": lease.released_at_utc.map(|at| at.to_rfc3339()),
        "takeover_of": lease.takeover_of,
    })
}

/// Claim a lane lease (KNOWLEDGE_CRDT_LEASE_CLAIMED on success).
pub async fn claim_lease(
    db: &(dyn Database + '_),
    pool: &PgPool,
    request: LeaseClaimRequestV1,
) -> Result<LeaseClaimOutcomeV1, LeaseFlowError> {
    let lease_id = new_ulid();
    let outcome = knowledge_crdt::insert_lease(
        pool,
        NewAgentLaneLease {
            lease_id: lease_id.clone(),
            lane_id: request.lane_id.clone(),
            actor_id: request.actor.canonical(),
            actor_kind: request.actor.kind().as_str().to_string(),
            session_id: request.session_id.clone(),
            correlation_id: request.correlation_id.clone(),
            scope_kind: request.scope_kind.as_str().to_string(),
            scope_id: request.scope_id.clone(),
            ttl_seconds: request.ttl_seconds,
            takeover_of: None,
        },
    )
    .await?;
    match outcome {
        LeaseInsertOutcome::Inserted(lease) => {
            append_lease_event(
                db,
                &lease,
                KernelEventType::KnowledgeCrdtLeaseClaimed,
                format!("knowledge-lease:{}:claimed", lease.lease_id),
                lease_payload(&lease, "claim"),
            )
            .await?;
            Ok(LeaseClaimOutcomeV1::Claimed(lease))
        }
        LeaseInsertOutcome::ScopeHeld { holder } => {
            let holder_expired = holder.is_expired;
            Ok(LeaseClaimOutcomeV1::ScopeHeld {
                holder,
                holder_expired,
            })
        }
    }
}

/// Renew an own, unexpired lease (KNOWLEDGE_CRDT_LEASE_RENEWED).
/// Returns None when the server-side guard refuses (expired, foreign,
/// released, or unknown lease) — callers escalate to the write guard for a
/// durable denial.
pub async fn renew_lease(
    db: &(dyn Database + '_),
    pool: &PgPool,
    lease_id: &str,
    actor: &KnowledgeActorIdV1,
    ttl_seconds: i64,
) -> Result<Option<AgentLaneLeaseRow>, LeaseFlowError> {
    let renewed =
        knowledge_crdt::renew_lease(pool, lease_id, &actor.canonical(), ttl_seconds).await?;
    if let Some(lease) = &renewed {
        append_lease_event(
            db,
            lease,
            KernelEventType::KnowledgeCrdtLeaseRenewed,
            format!(
                "knowledge-lease:{}:renewed:{}",
                lease.lease_id, lease.renewal_count
            ),
            lease_payload(lease, "renew"),
        )
        .await?;
    }
    Ok(renewed)
}

/// Release an own lease (KNOWLEDGE_CRDT_LEASE_RELEASED).
pub async fn release_lease(
    db: &(dyn Database + '_),
    pool: &PgPool,
    lease_id: &str,
    actor: &KnowledgeActorIdV1,
) -> Result<Option<AgentLaneLeaseRow>, LeaseFlowError> {
    let released = knowledge_crdt::release_lease(pool, lease_id, &actor.canonical()).await?;
    if let Some(lease) = &released {
        append_lease_event(
            db,
            lease,
            KernelEventType::KnowledgeCrdtLeaseReleased,
            format!("knowledge-lease:{}:released", lease.lease_id),
            lease_payload(lease, "release"),
        )
        .await?;
    }
    Ok(released)
}

/// Server-side expiry sweep: stamps overdue leases and appends one
/// KNOWLEDGE_CRDT_LEASE_EXPIRED event per lease (idempotent — the sweep
/// stamps each lease exactly once).
pub async fn expire_due_leases(
    db: &(dyn Database + '_),
    pool: &PgPool,
) -> Result<Vec<AgentLaneLeaseRow>, LeaseFlowError> {
    let expired = knowledge_crdt::sweep_expired_leases(pool).await?;
    for lease in &expired {
        append_lease_event(
            db,
            lease,
            KernelEventType::KnowledgeCrdtLeaseExpired,
            format!("knowledge-lease:{}:expired", lease.lease_id),
            lease_payload(lease, "expire"),
        )
        .await?;
    }
    Ok(expired)
}

/// Typed takeover outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseTakeoverOutcomeV1 {
    TakenOver(Box<AgentLaneLeaseRow>),
    Refused(LeaseTakeoverFailure),
}

/// Take over an expired lease's scope (KNOWLEDGE_CRDT_LEASE_TAKEN_OVER).
/// The new lease records `takeover_of = prior lease id` lineage.
pub async fn takeover_lease(
    db: &(dyn Database + '_),
    pool: &PgPool,
    prior_lease_id: &str,
    request: LeaseClaimRequestV1,
) -> Result<LeaseTakeoverOutcomeV1, LeaseFlowError> {
    let new_lease_id = new_ulid();
    let result = knowledge_crdt::takeover_lease(
        pool,
        prior_lease_id,
        NewAgentLaneLease {
            lease_id: new_lease_id,
            lane_id: request.lane_id.clone(),
            actor_id: request.actor.canonical(),
            actor_kind: request.actor.kind().as_str().to_string(),
            session_id: request.session_id.clone(),
            correlation_id: request.correlation_id.clone(),
            scope_kind: request.scope_kind.as_str().to_string(),
            scope_id: request.scope_id.clone(),
            ttl_seconds: request.ttl_seconds,
            takeover_of: Some(prior_lease_id.to_string()),
        },
    )
    .await?;
    match result {
        Ok(lease) => {
            append_lease_event(
                db,
                &lease,
                KernelEventType::KnowledgeCrdtLeaseTakenOver,
                format!(
                    "knowledge-lease:{}:takeover-of:{prior_lease_id}",
                    lease.lease_id
                ),
                lease_payload(&lease, "takeover"),
            )
            .await?;
            Ok(LeaseTakeoverOutcomeV1::TakenOver(Box::new(lease)))
        }
        Err(failure) => Ok(LeaseTakeoverOutcomeV1::Refused(failure)),
    }
}

/// Typed write-guard denial reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum LeaseWriteDenialReasonV1 {
    LeaseNotFound {
        lease_id: String,
    },
    LeaseReleased {
        lease_id: String,
    },
    LeaseExpired {
        lease_id: String,
        expires_at_utc: DateTime<Utc>,
    },
    ForeignLease {
        lease_id: String,
        holder_actor_id: String,
        writer_actor_id: String,
    },
    ScopeMismatch {
        lease_id: String,
        lease_scope_ref: String,
        write_scope_ref: String,
    },
}

/// Durable lease write denial: receipt row id + EventLedger event id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseWriteDenialV1 {
    pub schema_id: String,
    pub reason: LeaseWriteDenialReasonV1,
    pub denial_receipt_id: String,
    pub event_ledger_event_id: String,
}

/// Outcome of the pre-write lease guard.
#[derive(Debug, Clone, PartialEq)]
pub enum LeaseWriteGuardOutcomeV1 {
    Allowed(Box<AgentLaneLeaseRow>),
    Denied(Box<LeaseWriteDenialV1>),
}

/// Server-side pre-write guard: a write to `scope` claimed under `lease_id`
/// by `writer` is allowed only when the lease exists, is unreleased, is not
/// expired (database clock), belongs to the writer, and covers the scope.
/// Every denial is durable: KNOWLEDGE_CRDT_LEASE_WRITE_DENIED event plus a
/// `knowledge_crdt_denial_receipts` row.
pub async fn guard_lease_for_write(
    db: &(dyn Database + '_),
    pool: &PgPool,
    lease_id: &str,
    writer: &KnowledgeActorIdV1,
    writer_session_id: &str,
    writer_correlation_id: &str,
    workspace_id: &str,
    scope_kind: KnowledgeLeaseScopeKind,
    scope_id: &str,
) -> Result<LeaseWriteGuardOutcomeV1, LeaseFlowError> {
    let lease = knowledge_crdt::get_lease(pool, lease_id).await?;
    let write_scope_ref = format!("{}:{}", scope_kind.as_str(), scope_id);

    let reason = match &lease {
        None => Some(LeaseWriteDenialReasonV1::LeaseNotFound {
            lease_id: lease_id.to_string(),
        }),
        Some(lease) if lease.released_at_utc.is_some() => {
            Some(LeaseWriteDenialReasonV1::LeaseReleased {
                lease_id: lease.lease_id.clone(),
            })
        }
        Some(lease) if lease.is_expired => Some(LeaseWriteDenialReasonV1::LeaseExpired {
            lease_id: lease.lease_id.clone(),
            expires_at_utc: lease.expires_at_utc,
        }),
        Some(lease) if lease.actor_id != writer.canonical() => {
            Some(LeaseWriteDenialReasonV1::ForeignLease {
                lease_id: lease.lease_id.clone(),
                holder_actor_id: lease.actor_id.clone(),
                writer_actor_id: writer.canonical(),
            })
        }
        Some(lease) if lease.scope_ref() != write_scope_ref => {
            Some(LeaseWriteDenialReasonV1::ScopeMismatch {
                lease_id: lease.lease_id.clone(),
                lease_scope_ref: lease.scope_ref(),
                write_scope_ref: write_scope_ref.clone(),
            })
        }
        Some(_) => None,
    };

    match reason {
        None => Ok(LeaseWriteGuardOutcomeV1::Allowed(Box::new(
            lease.expect("lease present when no denial reason"),
        ))),
        Some(reason) => {
            let receipt_id = new_denial_receipt_id();
            let event = NewKernelEvent::builder(
                format!("KTR-KNOWLEDGE-LEASE-{lease_id}"),
                writer_session_id.to_string(),
                KernelEventType::KnowledgeCrdtLeaseWriteDenied,
                writer.to_kernel_actor(),
            )
            .aggregate("knowledge_agent_lease", lease_id.to_string())
            .idempotency_key(format!("knowledge-lease-denial:{receipt_id}"))
            .correlation_id(writer_correlation_id.to_string())
            .source_component("knowledge_crdt_agent_lease")
            .payload(json!({
                "schema_id": LEASE_WRITE_DENIAL_SCHEMA_ID,
                "reason": reason,
                "lease_id": lease_id,
                "writer_actor_id": writer.canonical(),
                "write_scope_ref": write_scope_ref,
            }))
            .build()
            .map_err(|error| LeaseFlowError::Event(error.to_string()))?;
            let stored_event = db
                .append_kernel_event(event)
                .await
                .map_err(|error| LeaseFlowError::Event(error.to_string()))?;

            let receipt = insert_denial_receipt(
                pool,
                NewKnowledgeCrdtDenialReceipt {
                    receipt_id: receipt_id.clone(),
                    receipt_kind: "lease_write_denied".to_string(),
                    workspace_id: workspace_id.to_string(),
                    document_id: None,
                    crdt_document_id: match scope_kind {
                        KnowledgeLeaseScopeKind::Document => Some(scope_id.to_string()),
                        _ => None,
                    },
                    scope_ref: format!("lease_scope:{write_scope_ref}"),
                    actor_id: writer.canonical(),
                    actor_kind: writer.kind().as_str().to_string(),
                    session_id: writer_session_id.to_string(),
                    correlation_id: writer_correlation_id.to_string(),
                    denial_payload: json!({
                        "schema_id": LEASE_WRITE_DENIAL_SCHEMA_ID,
                        "reason": reason,
                        "lease_id": lease_id,
                    }),
                    event_ledger_event_id: stored_event.event_id.clone(),
                    idempotency_key: format!("knowledge-lease-denial:{receipt_id}"),
                },
            )
            .await?;

            Ok(LeaseWriteGuardOutcomeV1::Denied(Box::new(
                LeaseWriteDenialV1 {
                    schema_id: LEASE_WRITE_DENIAL_SCHEMA_ID.to_string(),
                    reason,
                    denial_receipt_id: receipt.receipt_id,
                    event_ledger_event_id: stored_event.event_id,
                },
            )))
        }
    }
}

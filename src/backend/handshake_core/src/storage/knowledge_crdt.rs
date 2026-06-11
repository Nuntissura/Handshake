//! WP-KERNEL-009 CRDTAndConcurrencyCore storage (MT-065..MT-080).
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11. This
//! module is the PostgreSQL surface for the WP-009 CRDT support tables
//! (`knowledge_crdt_*`, migrations 0150-0159): denial receipts, graph
//! mutation proposals, promoted facts, AI edit proposals, agent lane leases,
//! swarm checkpoints and recovery receipts.
//!
//! Pattern follows `storage/knowledge.rs` (MT-049 precedent): free async
//! functions over `&sqlx::PgPool` rather than widening the legacy `Database`
//! trait. The shared pool is available everywhere it is needed (AppState,
//! `PostgresDatabase::pool()`, and the test fixture
//! `postgres_backend_with_pool_from_env`). There is NO in-memory, SQLite,
//! or fixture fallback: without PostgreSQL every function fails closed with
//! a typed `StorageError`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{StorageError, StorageResult};

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
pub const KNOWLEDGE_CRDT_DENIAL_KINDS: [&str; 10] = [
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
pub async fn insert_denial_receipt(
    pool: &PgPool,
    receipt: NewKnowledgeCrdtDenialReceipt,
) -> StorageResult<KnowledgeCrdtDenialReceiptRow> {
    if !KNOWLEDGE_CRDT_DENIAL_KINDS.contains(&receipt.receipt_kind.as_str()) {
        return Err(StorageError::Validation(
            "unknown knowledge CRDT denial receipt kind",
        ));
    }
    let inserted = sqlx::query(
        r#"
        INSERT INTO knowledge_crdt_denial_receipts (
            receipt_id, receipt_kind, workspace_id, document_id,
            crdt_document_id, scope_ref, actor_id, actor_kind,
            session_id, correlation_id, denial_payload,
            event_ledger_event_id, idempotency_key
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (idempotency_key) DO NOTHING
        "#,
    )
    .bind(&receipt.receipt_id)
    .bind(&receipt.receipt_kind)
    .bind(&receipt.workspace_id)
    .bind(&receipt.document_id)
    .bind(&receipt.crdt_document_id)
    .bind(&receipt.scope_ref)
    .bind(&receipt.actor_id)
    .bind(&receipt.actor_kind)
    .bind(&receipt.session_id)
    .bind(&receipt.correlation_id)
    .bind(&receipt.denial_payload)
    .bind(&receipt.event_ledger_event_id)
    .bind(&receipt.idempotency_key)
    .execute(pool)
    .await?;
    let _ = inserted;
    get_denial_receipt_by_idempotency_key(pool, &receipt.idempotency_key)
        .await?
        .ok_or(StorageError::NotFound(
            "knowledge CRDT denial receipt after insert",
        ))
}

pub async fn get_denial_receipt_by_idempotency_key(
    pool: &PgPool,
    idempotency_key: &str,
) -> StorageResult<Option<KnowledgeCrdtDenialReceiptRow>> {
    let row = sqlx::query(&select_denial_receipts_sql("WHERE idempotency_key = $1"))
        .bind(idempotency_key)
        .fetch_optional(pool)
        .await?;
    row.map(map_denial_receipt).transpose()
}

/// All denial receipts for a CRDT document, oldest first (MT-075 input).
pub async fn list_denial_receipts_for_document(
    pool: &PgPool,
    crdt_document_id: &str,
) -> StorageResult<Vec<KnowledgeCrdtDenialReceiptRow>> {
    let rows = sqlx::query(&select_denial_receipts_sql(
        "WHERE crdt_document_id = $1 ORDER BY created_at ASC, receipt_id ASC",
    ))
    .bind(crdt_document_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(map_denial_receipt).collect()
}

/// All denial receipts for an arbitrary scope ref (lease scopes, proposals).
pub async fn list_denial_receipts_for_scope(
    pool: &PgPool,
    scope_ref: &str,
) -> StorageResult<Vec<KnowledgeCrdtDenialReceiptRow>> {
    let rows = sqlx::query(&select_denial_receipts_sql(
        "WHERE scope_ref = $1 ORDER BY created_at ASC, receipt_id ASC",
    ))
    .bind(scope_ref)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(map_denial_receipt).collect()
}

fn select_denial_receipts_sql(suffix: &str) -> String {
    format!(
        r#"
        SELECT receipt_id, receipt_kind, workspace_id, document_id,
               crdt_document_id, scope_ref, actor_id, actor_kind,
               session_id, correlation_id, denial_payload,
               event_ledger_event_id, idempotency_key, created_at
        FROM knowledge_crdt_denial_receipts
        {suffix}
        "#
    )
}

fn map_denial_receipt(row: sqlx::postgres::PgRow) -> StorageResult<KnowledgeCrdtDenialReceiptRow> {
    Ok(KnowledgeCrdtDenialReceiptRow {
        receipt_id: row.try_get("receipt_id")?,
        receipt_kind: row.try_get("receipt_kind")?,
        workspace_id: row.try_get("workspace_id")?,
        document_id: row.try_get("document_id")?,
        crdt_document_id: row.try_get("crdt_document_id")?,
        scope_ref: row.try_get("scope_ref")?,
        actor_id: row.try_get("actor_id")?,
        actor_kind: row.try_get("actor_kind")?,
        session_id: row.try_get("session_id")?,
        correlation_id: row.try_get("correlation_id")?,
        denial_payload: row.try_get("denial_payload")?,
        event_ledger_event_id: row.try_get("event_ledger_event_id")?,
        idempotency_key: row.try_get("idempotency_key")?,
        created_at: row.try_get("created_at")?,
    })
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

const LEASE_COLUMNS: &str = r#"
    lease_id, lane_id, actor_id, actor_kind, session_id, correlation_id,
    scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count,
    released_at_utc, expired_at_utc, takeover_of,
    (expires_at_utc < NOW()) AS is_expired
"#;

fn map_lease(row: sqlx::postgres::PgRow) -> StorageResult<AgentLaneLeaseRow> {
    Ok(AgentLaneLeaseRow {
        lease_id: row.try_get("lease_id")?,
        lane_id: row.try_get("lane_id")?,
        actor_id: row.try_get("actor_id")?,
        actor_kind: row.try_get("actor_kind")?,
        session_id: row.try_get("session_id")?,
        correlation_id: row.try_get("correlation_id")?,
        scope_kind: row.try_get("scope_kind")?,
        scope_id: row.try_get("scope_id")?,
        claimed_at_utc: row.try_get("claimed_at_utc")?,
        expires_at_utc: row.try_get("expires_at_utc")?,
        renewal_count: row.try_get("renewal_count")?,
        released_at_utc: row.try_get("released_at_utc")?,
        expired_at_utc: row.try_get("expired_at_utc")?,
        takeover_of: row.try_get("takeover_of")?,
        is_expired: row.try_get("is_expired")?,
    })
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
    pool: &PgPool,
    lease: NewAgentLaneLease,
) -> StorageResult<LeaseInsertOutcome> {
    if lease.ttl_seconds <= 0 {
        return Err(StorageError::Validation("lease ttl must be positive"));
    }
    let result = sqlx::query(&format!(
        r#"
        INSERT INTO knowledge_crdt_agent_lane_leases (
            lease_id, lane_id, actor_id, actor_kind, session_id,
            correlation_id, scope_kind, scope_id,
            claimed_at_utc, expires_at_utc, takeover_of
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8,
                NOW(), NOW() + make_interval(secs => $9::double precision), $10)
        RETURNING {LEASE_COLUMNS}
        "#
    ))
    .bind(&lease.lease_id)
    .bind(&lease.lane_id)
    .bind(&lease.actor_id)
    .bind(&lease.actor_kind)
    .bind(&lease.session_id)
    .bind(&lease.correlation_id)
    .bind(&lease.scope_kind)
    .bind(&lease.scope_id)
    .bind(lease.ttl_seconds)
    .bind(&lease.takeover_of)
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => Ok(LeaseInsertOutcome::Inserted(Box::new(map_lease(row)?))),
        Err(error) => {
            let message = error.to_string();
            if message.contains("uq_knowledge_crdt_lease_active_scope") {
                let holder =
                    find_unreleased_lease_for_scope(pool, &lease.scope_kind, &lease.scope_id)
                        .await?
                        .ok_or(StorageError::Conflict(
                            "lease scope contended but holder vanished; retry claim",
                        ))?;
                Ok(LeaseInsertOutcome::ScopeHeld {
                    holder: Box::new(holder),
                })
            } else {
                Err(StorageError::Database(message))
            }
        }
    }
}

pub async fn get_lease(pool: &PgPool, lease_id: &str) -> StorageResult<Option<AgentLaneLeaseRow>> {
    let row = sqlx::query(&format!(
        "SELECT {LEASE_COLUMNS} FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $1"
    ))
    .bind(lease_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_lease).transpose()
}

/// The unreleased lease for a scope, if any (may be expired; `is_expired`
/// tells, per the database clock).
pub async fn find_unreleased_lease_for_scope(
    pool: &PgPool,
    scope_kind: &str,
    scope_id: &str,
) -> StorageResult<Option<AgentLaneLeaseRow>> {
    let row = sqlx::query(&format!(
        r#"
        SELECT {LEASE_COLUMNS} FROM knowledge_crdt_agent_lane_leases
        WHERE scope_kind = $1 AND scope_id = $2 AND released_at_utc IS NULL
        "#
    ))
    .bind(scope_kind)
    .bind(scope_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_lease).transpose()
}

/// Renew: extends expiry by `ttl_seconds` from NOW() without changing lease
/// identity. Server-side guards: own lease, unreleased, NOT expired.
pub async fn renew_lease(
    pool: &PgPool,
    lease_id: &str,
    actor_id: &str,
    ttl_seconds: i64,
) -> StorageResult<Option<AgentLaneLeaseRow>> {
    if ttl_seconds <= 0 {
        return Err(StorageError::Validation("lease ttl must be positive"));
    }
    let row = sqlx::query(&format!(
        r#"
        UPDATE knowledge_crdt_agent_lane_leases
        SET expires_at_utc = NOW() + make_interval(secs => $3::double precision),
            renewal_count = renewal_count + 1
        WHERE lease_id = $1 AND actor_id = $2
          AND released_at_utc IS NULL
          AND expires_at_utc > NOW()
        RETURNING {LEASE_COLUMNS}
        "#
    ))
    .bind(lease_id)
    .bind(actor_id)
    .bind(ttl_seconds)
    .fetch_optional(pool)
    .await?;
    row.map(map_lease).transpose()
}

/// Release an own lease (allowed after expiry as cleanup; expiry only blocks
/// writes and renewals).
pub async fn release_lease(
    pool: &PgPool,
    lease_id: &str,
    actor_id: &str,
) -> StorageResult<Option<AgentLaneLeaseRow>> {
    let row = sqlx::query(&format!(
        r#"
        UPDATE knowledge_crdt_agent_lane_leases
        SET released_at_utc = NOW()
        WHERE lease_id = $1 AND actor_id = $2 AND released_at_utc IS NULL
        RETURNING {LEASE_COLUMNS}
        "#
    ))
    .bind(lease_id)
    .bind(actor_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_lease).transpose()
}

/// Server-side expiry sweep: stamp every overdue unreleased lease exactly
/// once (expired_at_utc) and return the stamped rows so the kernel layer
/// can append the KNOWLEDGE_CRDT_LEASE_EXPIRED events.
pub async fn sweep_expired_leases(pool: &PgPool) -> StorageResult<Vec<AgentLaneLeaseRow>> {
    let rows = sqlx::query(&format!(
        r#"
        UPDATE knowledge_crdt_agent_lane_leases
        SET expired_at_utc = NOW()
        WHERE released_at_utc IS NULL
          AND expired_at_utc IS NULL
          AND expires_at_utc < NOW()
        RETURNING {LEASE_COLUMNS}
        "#
    ))
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(map_lease).collect()
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
    pool: &PgPool,
    prior_lease_id: &str,
    new_lease: NewAgentLaneLease,
) -> StorageResult<Result<AgentLaneLeaseRow, LeaseTakeoverFailure>> {
    if new_lease.ttl_seconds <= 0 {
        return Err(StorageError::Validation("lease ttl must be positive"));
    }
    let mut tx = pool.begin().await?;

    let prior = sqlx::query(&format!(
        r#"
        SELECT {LEASE_COLUMNS} FROM knowledge_crdt_agent_lane_leases
        WHERE lease_id = $1
        FOR UPDATE
        "#
    ))
    .bind(prior_lease_id)
    .fetch_optional(&mut *tx)
    .await?;
    let prior = match prior {
        Some(row) => map_lease(row)?,
        None => {
            tx.rollback().await?;
            return Ok(Err(LeaseTakeoverFailure::PriorLeaseNotFound));
        }
    };
    if prior.released_at_utc.is_some() {
        tx.rollback().await?;
        return Ok(Err(LeaseTakeoverFailure::PriorLeaseAlreadyReleased));
    }
    if !prior.is_expired {
        tx.rollback().await?;
        return Ok(Err(LeaseTakeoverFailure::PriorLeaseNotExpired {
            expires_at_utc: prior.expires_at_utc,
        }));
    }

    sqlx::query(
        r#"
        UPDATE knowledge_crdt_agent_lane_leases
        SET released_at_utc = NOW(),
            expired_at_utc = COALESCE(expired_at_utc, NOW())
        WHERE lease_id = $1
        "#,
    )
    .bind(prior_lease_id)
    .execute(&mut *tx)
    .await?;

    let inserted = sqlx::query(&format!(
        r#"
        INSERT INTO knowledge_crdt_agent_lane_leases (
            lease_id, lane_id, actor_id, actor_kind, session_id,
            correlation_id, scope_kind, scope_id,
            claimed_at_utc, expires_at_utc, takeover_of
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8,
                NOW(), NOW() + make_interval(secs => $9::double precision), $10)
        RETURNING {LEASE_COLUMNS}
        "#
    ))
    .bind(&new_lease.lease_id)
    .bind(&new_lease.lane_id)
    .bind(&new_lease.actor_id)
    .bind(&new_lease.actor_kind)
    .bind(&new_lease.session_id)
    .bind(&new_lease.correlation_id)
    .bind(&new_lease.scope_kind)
    .bind(&new_lease.scope_id)
    .bind(new_lease.ttl_seconds)
    .bind(prior_lease_id)
    .fetch_one(&mut *tx)
    .await?;
    let lease = map_lease(inserted)?;

    tx.commit().await?;
    Ok(Ok(lease))
}

/// Walk the takeover lineage from `lease_id` back to the root claim
/// (newest first). Chains are short (one row per takeover).
pub async fn lease_lineage(pool: &PgPool, lease_id: &str) -> StorageResult<Vec<AgentLaneLeaseRow>> {
    let mut lineage = Vec::new();
    let mut cursor = Some(lease_id.to_string());
    while let Some(current) = cursor {
        let Some(lease) = get_lease(pool, &current).await? else {
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

const GRAPH_PROPOSAL_COLUMNS: &str = r#"
    proposal_id, workspace_id, mutation_kind, mutation_payload,
    source_span_refs, confidence, actor_id, actor_kind, session_id,
    correlation_id, lease_id, review_state, decided_by, decided_at_utc,
    decision_reason, recorded_event_id, decided_event_id, created_at_utc
"#;

fn map_graph_proposal(row: sqlx::postgres::PgRow) -> StorageResult<GraphMutationProposalRow> {
    Ok(GraphMutationProposalRow {
        proposal_id: row.try_get("proposal_id")?,
        workspace_id: row.try_get("workspace_id")?,
        mutation_kind: row.try_get("mutation_kind")?,
        mutation_payload: row.try_get("mutation_payload")?,
        source_span_refs: row.try_get("source_span_refs")?,
        confidence: row.try_get("confidence")?,
        actor_id: row.try_get("actor_id")?,
        actor_kind: row.try_get("actor_kind")?,
        session_id: row.try_get("session_id")?,
        correlation_id: row.try_get("correlation_id")?,
        lease_id: row.try_get("lease_id")?,
        review_state: row.try_get("review_state")?,
        decided_by: row.try_get("decided_by")?,
        decided_at_utc: row.try_get("decided_at_utc")?,
        decision_reason: row.try_get("decision_reason")?,
        recorded_event_id: row.try_get("recorded_event_id")?,
        decided_event_id: row.try_get("decided_event_id")?,
        created_at_utc: row.try_get("created_at_utc")?,
    })
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
    pool: &PgPool,
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
    let row = sqlx::query(&format!(
        r#"
        INSERT INTO knowledge_crdt_graph_proposals (
            proposal_id, workspace_id, mutation_kind, mutation_payload,
            source_span_refs, confidence, actor_id, actor_kind,
            session_id, correlation_id, lease_id, recorded_event_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING {GRAPH_PROPOSAL_COLUMNS}
        "#
    ))
    .bind(&proposal.proposal_id)
    .bind(&proposal.workspace_id)
    .bind(&proposal.mutation_kind)
    .bind(&proposal.mutation_payload)
    .bind(serde_json::json!(proposal.source_span_refs))
    .bind(proposal.confidence)
    .bind(&proposal.actor_id)
    .bind(&proposal.actor_kind)
    .bind(&proposal.session_id)
    .bind(&proposal.correlation_id)
    .bind(&proposal.lease_id)
    .bind(&proposal.recorded_event_id)
    .fetch_one(pool)
    .await?;
    map_graph_proposal(row)
}

pub async fn get_graph_proposal(
    pool: &PgPool,
    proposal_id: &str,
) -> StorageResult<Option<GraphMutationProposalRow>> {
    let row = sqlx::query(&format!(
        "SELECT {GRAPH_PROPOSAL_COLUMNS} FROM knowledge_crdt_graph_proposals WHERE proposal_id = $1"
    ))
    .bind(proposal_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_graph_proposal).transpose()
}

pub async fn list_graph_proposals_by_state(
    pool: &PgPool,
    workspace_id: &str,
    review_state: &str,
) -> StorageResult<Vec<GraphMutationProposalRow>> {
    let rows = sqlx::query(&format!(
        r#"
        SELECT {GRAPH_PROPOSAL_COLUMNS} FROM knowledge_crdt_graph_proposals
        WHERE workspace_id = $1 AND review_state = $2
        ORDER BY created_at_utc ASC, proposal_id ASC
        "#
    ))
    .bind(workspace_id)
    .bind(review_state)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(map_graph_proposal).collect()
}

/// Atomic review decision: proposed -> approved|rejected. Returns None when
/// the proposal is not in 'proposed' (no lost-update double decisions).
pub async fn decide_graph_proposal(
    pool: &PgPool,
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
    let row = sqlx::query(&format!(
        r#"
        UPDATE knowledge_crdt_graph_proposals
        SET review_state = $2, decided_by = $3, decided_at_utc = NOW(),
            decision_reason = $4, decided_event_id = $5
        WHERE proposal_id = $1 AND review_state = 'proposed'
        RETURNING {GRAPH_PROPOSAL_COLUMNS}
        "#
    ))
    .bind(proposal_id)
    .bind(new_state)
    .bind(decided_by)
    .bind(decision_reason)
    .bind(decided_event_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_graph_proposal).transpose()
}

/// approved -> promoted (MT-069 bridge finalization).
pub async fn mark_graph_proposal_promoted(
    pool: &PgPool,
    proposal_id: &str,
) -> StorageResult<Option<GraphMutationProposalRow>> {
    let row = sqlx::query(&format!(
        r#"
        UPDATE knowledge_crdt_graph_proposals
        SET review_state = 'promoted'
        WHERE proposal_id = $1 AND review_state = 'approved'
        RETURNING {GRAPH_PROPOSAL_COLUMNS}
        "#
    ))
    .bind(proposal_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_graph_proposal).transpose()
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
    pool: &PgPool,
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
                // Resolve span -> source -> workspace + stale, in one query.
                let row = sqlx::query(
                    r#"
                    SELECT src.workspace_id AS workspace_id, src.stale AS stale
                    FROM knowledge_spans sp
                    JOIN knowledge_sources src ON src.source_id = sp.source_id
                    WHERE sp.span_id = $1
                    "#,
                )
                .bind(&span_id)
                .fetch_optional(pool)
                .await?;
                match row {
                    None => {
                        rejections.push(PromotionSpanRejection::SpanNotFound { span_ref: span_id })
                    }
                    Some(row) => {
                        let span_workspace_id: String = row.try_get("workspace_id")?;
                        let stale: bool = row.try_get("stale")?;
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

/// Column list for `knowledge_crdt_promoted_facts` selects. `pub(crate)` so
/// the Postgres atomic-promotion helper (postgres.rs) can re-select the row
/// inside its transaction.
pub(crate) const PROMOTED_FACT_COLUMNS: &str = r#"
    fact_id, proposal_id, workspace_id, mutation_kind, fact_payload,
    source_span_refs, confidence, proposed_by, promoted_by,
    promotion_requested_event_id, promotion_accepted_event_id, promoted_at_utc
"#;

/// `pub(crate)` row mapper for the Postgres atomic-promotion helper.
pub(crate) fn map_promoted_fact_row(row: sqlx::postgres::PgRow) -> StorageResult<PromotedFactRow> {
    map_promoted_fact(row)
}

fn map_promoted_fact(row: sqlx::postgres::PgRow) -> StorageResult<PromotedFactRow> {
    Ok(PromotedFactRow {
        fact_id: row.try_get("fact_id")?,
        proposal_id: row.try_get("proposal_id")?,
        workspace_id: row.try_get("workspace_id")?,
        mutation_kind: row.try_get("mutation_kind")?,
        fact_payload: row.try_get("fact_payload")?,
        source_span_refs: row.try_get("source_span_refs")?,
        confidence: row.try_get("confidence")?,
        proposed_by: row.try_get("proposed_by")?,
        promoted_by: row.try_get("promoted_by")?,
        promotion_requested_event_id: row.try_get("promotion_requested_event_id")?,
        promotion_accepted_event_id: row.try_get("promotion_accepted_event_id")?,
        promoted_at_utc: row.try_get("promoted_at_utc")?,
    })
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
    pool: &PgPool,
    fact: NewPromotedFact,
) -> StorageResult<PromotedFactRow> {
    sqlx::query(
        r#"
        INSERT INTO knowledge_crdt_promoted_facts (
            fact_id, proposal_id, workspace_id, mutation_kind, fact_payload,
            source_span_refs, confidence, proposed_by, promoted_by,
            promotion_requested_event_id, promotion_accepted_event_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (proposal_id) DO NOTHING
        "#,
    )
    .bind(&fact.fact_id)
    .bind(&fact.proposal_id)
    .bind(&fact.workspace_id)
    .bind(&fact.mutation_kind)
    .bind(&fact.fact_payload)
    .bind(&fact.source_span_refs)
    .bind(fact.confidence)
    .bind(&fact.proposed_by)
    .bind(&fact.promoted_by)
    .bind(&fact.promotion_requested_event_id)
    .bind(&fact.promotion_accepted_event_id)
    .execute(pool)
    .await?;
    get_promoted_fact_by_proposal(pool, &fact.proposal_id)
        .await?
        .ok_or(StorageError::NotFound("promoted fact after insert"))
}

pub async fn get_promoted_fact_by_proposal(
    pool: &PgPool,
    proposal_id: &str,
) -> StorageResult<Option<PromotedFactRow>> {
    let row = sqlx::query(&format!(
        "SELECT {PROMOTED_FACT_COLUMNS} FROM knowledge_crdt_promoted_facts WHERE proposal_id = $1"
    ))
    .bind(proposal_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_promoted_fact).transpose()
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
    pool: &PgPool,
    fact: NewPromotedFact,
) -> StorageResult<PromotedFactRow> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO knowledge_crdt_promoted_facts (
            fact_id, proposal_id, workspace_id, mutation_kind, fact_payload,
            source_span_refs, confidence, proposed_by, promoted_by,
            promotion_requested_event_id, promotion_accepted_event_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (proposal_id) DO NOTHING
        "#,
    )
    .bind(&fact.fact_id)
    .bind(&fact.proposal_id)
    .bind(&fact.workspace_id)
    .bind(&fact.mutation_kind)
    .bind(&fact.fact_payload)
    .bind(&fact.source_span_refs)
    .bind(fact.confidence)
    .bind(&fact.proposed_by)
    .bind(&fact.promoted_by)
    .bind(&fact.promotion_requested_event_id)
    .bind(&fact.promotion_accepted_event_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        UPDATE knowledge_crdt_graph_proposals
        SET review_state = 'promoted'
        WHERE proposal_id = $1 AND review_state = 'approved'
        "#,
    )
    .bind(&fact.proposal_id)
    .execute(&mut *tx)
    .await?;
    let row = sqlx::query(&format!(
        "SELECT {PROMOTED_FACT_COLUMNS} FROM knowledge_crdt_promoted_facts WHERE proposal_id = $1"
    ))
    .bind(&fact.proposal_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    row.map(map_promoted_fact)
        .transpose()?
        .ok_or(StorageError::NotFound("promoted fact after materialize"))
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

const AI_EDIT_PROPOSAL_COLUMNS: &str = r#"
    proposal_id, workspace_id, document_id, crdt_document_id,
    base_update_seq, base_state_vector, proposed_diff, diff_sha256,
    source_span_citations, actor_id, actor_kind, session_id, correlation_id,
    lease_id, review_state, decided_by, decided_at_utc, decision_reason,
    recorded_event_id, decided_event_id, promotion_requested_event_id,
    promotion_accepted_event_id, applied_update_id, applied_update_sha256,
    created_at_utc
"#;

fn map_ai_edit_proposal(row: sqlx::postgres::PgRow) -> StorageResult<AiEditProposalRow> {
    Ok(AiEditProposalRow {
        proposal_id: row.try_get("proposal_id")?,
        workspace_id: row.try_get("workspace_id")?,
        document_id: row.try_get("document_id")?,
        crdt_document_id: row.try_get("crdt_document_id")?,
        base_update_seq: row.try_get("base_update_seq")?,
        base_state_vector: row.try_get("base_state_vector")?,
        proposed_diff: row.try_get("proposed_diff")?,
        diff_sha256: row.try_get("diff_sha256")?,
        source_span_citations: row.try_get("source_span_citations")?,
        actor_id: row.try_get("actor_id")?,
        actor_kind: row.try_get("actor_kind")?,
        session_id: row.try_get("session_id")?,
        correlation_id: row.try_get("correlation_id")?,
        lease_id: row.try_get("lease_id")?,
        review_state: row.try_get("review_state")?,
        decided_by: row.try_get("decided_by")?,
        decided_at_utc: row.try_get("decided_at_utc")?,
        decision_reason: row.try_get("decision_reason")?,
        recorded_event_id: row.try_get("recorded_event_id")?,
        decided_event_id: row.try_get("decided_event_id")?,
        promotion_requested_event_id: row.try_get("promotion_requested_event_id")?,
        promotion_accepted_event_id: row.try_get("promotion_accepted_event_id")?,
        applied_update_id: row.try_get("applied_update_id")?,
        applied_update_sha256: row.try_get("applied_update_sha256")?,
        created_at_utc: row.try_get("created_at_utc")?,
    })
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
    pool: &PgPool,
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
    let row = sqlx::query(&format!(
        r#"
        INSERT INTO knowledge_crdt_ai_edit_proposals (
            proposal_id, workspace_id, document_id, crdt_document_id,
            base_update_seq, base_state_vector, proposed_diff, diff_sha256,
            source_span_citations, actor_id, actor_kind, session_id,
            correlation_id, lease_id, recorded_event_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        RETURNING {AI_EDIT_PROPOSAL_COLUMNS}
        "#
    ))
    .bind(&proposal.proposal_id)
    .bind(&proposal.workspace_id)
    .bind(&proposal.document_id)
    .bind(&proposal.crdt_document_id)
    .bind(proposal.base_update_seq)
    .bind(&proposal.base_state_vector)
    .bind(&proposal.proposed_diff)
    .bind(&proposal.diff_sha256)
    .bind(serde_json::json!(proposal.source_span_citations))
    .bind(&proposal.actor_id)
    .bind(&proposal.actor_kind)
    .bind(&proposal.session_id)
    .bind(&proposal.correlation_id)
    .bind(&proposal.lease_id)
    .bind(&proposal.recorded_event_id)
    .fetch_one(pool)
    .await?;
    map_ai_edit_proposal(row)
}

pub async fn get_ai_edit_proposal(
    pool: &PgPool,
    proposal_id: &str,
) -> StorageResult<Option<AiEditProposalRow>> {
    let row = sqlx::query(&format!(
        "SELECT {AI_EDIT_PROPOSAL_COLUMNS} FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $1"
    ))
    .bind(proposal_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_ai_edit_proposal).transpose()
}

pub async fn list_ai_edit_proposals_for_document(
    pool: &PgPool,
    crdt_document_id: &str,
    review_state: Option<&str>,
) -> StorageResult<Vec<AiEditProposalRow>> {
    let rows = match review_state {
        Some(state) => {
            sqlx::query(&format!(
                r#"
                SELECT {AI_EDIT_PROPOSAL_COLUMNS} FROM knowledge_crdt_ai_edit_proposals
                WHERE crdt_document_id = $1 AND review_state = $2
                ORDER BY created_at_utc ASC, proposal_id ASC
                "#
            ))
            .bind(crdt_document_id)
            .bind(state)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query(&format!(
                r#"
                SELECT {AI_EDIT_PROPOSAL_COLUMNS} FROM knowledge_crdt_ai_edit_proposals
                WHERE crdt_document_id = $1
                ORDER BY created_at_utc ASC, proposal_id ASC
                "#
            ))
            .bind(crdt_document_id)
            .fetch_all(pool)
            .await?
        }
    };
    rows.into_iter().map(map_ai_edit_proposal).collect()
}

/// Atomic review decision: proposed -> approved|rejected (no lost updates).
pub async fn decide_ai_edit_proposal(
    pool: &PgPool,
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
    let row = sqlx::query(&format!(
        r#"
        UPDATE knowledge_crdt_ai_edit_proposals
        SET review_state = $2, decided_by = $3, decided_at_utc = NOW(),
            decision_reason = $4, decided_event_id = $5
        WHERE proposal_id = $1 AND review_state = 'proposed'
        RETURNING {AI_EDIT_PROPOSAL_COLUMNS}
        "#
    ))
    .bind(proposal_id)
    .bind(new_state)
    .bind(decided_by)
    .bind(decision_reason)
    .bind(decided_event_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_ai_edit_proposal).transpose()
}

/// approved -> promoted with the EventLedger promotion pair (atomic guard).
pub async fn mark_ai_edit_proposal_promoted(
    pool: &PgPool,
    proposal_id: &str,
    promotion_requested_event_id: &str,
    promotion_accepted_event_id: &str,
) -> StorageResult<Option<AiEditProposalRow>> {
    let row = sqlx::query(&format!(
        r#"
        UPDATE knowledge_crdt_ai_edit_proposals
        SET review_state = 'promoted',
            promotion_requested_event_id = $2,
            promotion_accepted_event_id = $3
        WHERE proposal_id = $1 AND review_state = 'approved'
        RETURNING {AI_EDIT_PROPOSAL_COLUMNS}
        "#
    ))
    .bind(proposal_id)
    .bind(promotion_requested_event_id)
    .bind(promotion_accepted_event_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_ai_edit_proposal).transpose()
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
    pool: &PgPool,
    proposal_id: &str,
    applied_update_id: &str,
    applied_content_sha256: &str,
) -> StorageResult<Option<AiEditProposalRow>> {
    let row = sqlx::query(&format!(
        r#"
        UPDATE knowledge_crdt_ai_edit_proposals
        SET applied_update_id = $2,
            applied_update_sha256 = $3
        WHERE proposal_id = $1
          AND review_state IN ('approved', 'promoted')
          AND diff_sha256 = $3
          AND (applied_update_id IS NULL OR applied_update_id = $2)
        RETURNING {AI_EDIT_PROPOSAL_COLUMNS}
        "#
    ))
    .bind(proposal_id)
    .bind(applied_update_id)
    .bind(applied_content_sha256)
    .fetch_optional(pool)
    .await?;
    row.map(map_ai_edit_proposal).transpose()
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

const CHECKPOINT_COLUMNS: &str = r#"
    checkpoint_id, session_id, actor_id, lane_id, lease_id, scope_ref,
    resume_pointer, checkpoint_payload, payload_sha256, recorded_event_id,
    created_at_utc
"#;

fn map_checkpoint(row: sqlx::postgres::PgRow) -> StorageResult<SwarmCheckpointRow> {
    Ok(SwarmCheckpointRow {
        checkpoint_id: row.try_get("checkpoint_id")?,
        session_id: row.try_get("session_id")?,
        actor_id: row.try_get("actor_id")?,
        lane_id: row.try_get("lane_id")?,
        lease_id: row.try_get("lease_id")?,
        scope_ref: row.try_get("scope_ref")?,
        resume_pointer: row.try_get("resume_pointer")?,
        checkpoint_payload: row.try_get("checkpoint_payload")?,
        payload_sha256: row.try_get("payload_sha256")?,
        recorded_event_id: row.try_get("recorded_event_id")?,
        created_at_utc: row.try_get("created_at_utc")?,
    })
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
    pool: &PgPool,
    checkpoint: NewSwarmCheckpoint,
) -> StorageResult<SwarmCheckpointRow> {
    let row = sqlx::query(&format!(
        r#"
        INSERT INTO knowledge_crdt_swarm_checkpoints (
            checkpoint_id, session_id, actor_id, lane_id, lease_id,
            scope_ref, resume_pointer, checkpoint_payload, payload_sha256,
            recorded_event_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING {CHECKPOINT_COLUMNS}
        "#
    ))
    .bind(&checkpoint.checkpoint_id)
    .bind(&checkpoint.session_id)
    .bind(&checkpoint.actor_id)
    .bind(&checkpoint.lane_id)
    .bind(&checkpoint.lease_id)
    .bind(&checkpoint.scope_ref)
    .bind(&checkpoint.resume_pointer)
    .bind(&checkpoint.checkpoint_payload)
    .bind(&checkpoint.payload_sha256)
    .bind(&checkpoint.recorded_event_id)
    .fetch_one(pool)
    .await?;
    map_checkpoint(row)
}

pub async fn get_swarm_checkpoint(
    pool: &PgPool,
    checkpoint_id: &str,
) -> StorageResult<Option<SwarmCheckpointRow>> {
    let row = sqlx::query(&format!(
        "SELECT {CHECKPOINT_COLUMNS} FROM knowledge_crdt_swarm_checkpoints WHERE checkpoint_id = $1"
    ))
    .bind(checkpoint_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_checkpoint).transpose()
}

/// Latest checkpoint for a lane (recovery entrypoint after session loss).
pub async fn latest_checkpoint_for_lane(
    pool: &PgPool,
    lane_id: &str,
) -> StorageResult<Option<SwarmCheckpointRow>> {
    let row = sqlx::query(&format!(
        r#"
        SELECT {CHECKPOINT_COLUMNS} FROM knowledge_crdt_swarm_checkpoints
        WHERE lane_id = $1
        ORDER BY created_at_utc DESC, checkpoint_id DESC
        LIMIT 1
        "#
    ))
    .bind(lane_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_checkpoint).transpose()
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

const RECOVERY_RECEIPT_COLUMNS: &str = r#"
    receipt_id, checkpoint_id, prior_session_id, new_session_id,
    new_actor_id, new_lease_id, lease_lineage, resume_pointer,
    recorded_event_id, created_at_utc
"#;

fn map_recovery_receipt(row: sqlx::postgres::PgRow) -> StorageResult<RecoveryReceiptRow> {
    Ok(RecoveryReceiptRow {
        receipt_id: row.try_get("receipt_id")?,
        checkpoint_id: row.try_get("checkpoint_id")?,
        prior_session_id: row.try_get("prior_session_id")?,
        new_session_id: row.try_get("new_session_id")?,
        new_actor_id: row.try_get("new_actor_id")?,
        new_lease_id: row.try_get("new_lease_id")?,
        lease_lineage: row.try_get("lease_lineage")?,
        resume_pointer: row.try_get("resume_pointer")?,
        recorded_event_id: row.try_get("recorded_event_id")?,
        created_at_utc: row.try_get("created_at_utc")?,
    })
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
    pool: &PgPool,
    receipt: NewRecoveryReceipt,
) -> StorageResult<RecoveryReceiptRow> {
    let row = sqlx::query(&format!(
        r#"
        INSERT INTO knowledge_crdt_recovery_receipts (
            receipt_id, checkpoint_id, prior_session_id, new_session_id,
            new_actor_id, new_lease_id, lease_lineage, resume_pointer,
            recorded_event_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING {RECOVERY_RECEIPT_COLUMNS}
        "#
    ))
    .bind(&receipt.receipt_id)
    .bind(&receipt.checkpoint_id)
    .bind(&receipt.prior_session_id)
    .bind(&receipt.new_session_id)
    .bind(&receipt.new_actor_id)
    .bind(&receipt.new_lease_id)
    .bind(&receipt.lease_lineage)
    .bind(&receipt.resume_pointer)
    .bind(&receipt.recorded_event_id)
    .fetch_one(pool)
    .await?;
    map_recovery_receipt(row)
}

pub async fn list_recovery_receipts_for_checkpoint(
    pool: &PgPool,
    checkpoint_id: &str,
) -> StorageResult<Vec<RecoveryReceiptRow>> {
    let rows = sqlx::query(&format!(
        r#"
        SELECT {RECOVERY_RECEIPT_COLUMNS} FROM knowledge_crdt_recovery_receipts
        WHERE checkpoint_id = $1
        ORDER BY created_at_utc ASC, receipt_id ASC
        "#
    ))
    .bind(checkpoint_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(map_recovery_receipt).collect()
}

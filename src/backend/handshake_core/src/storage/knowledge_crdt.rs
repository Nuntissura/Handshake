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
pub const KNOWLEDGE_CRDT_DENIAL_KINDS: [&str; 9] = [
    "stale_draft_save",
    "concurrent_draft_fork",
    "ahead_of_head_save",
    "update_content_mismatch",
    "sequence_slot_race",
    "lease_write_denied",
    "index_run_slot_rejected",
    "graph_promotion_denied",
    "ai_edit_promotion_denied",
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


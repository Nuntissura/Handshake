//! WP-KERNEL-012 MT-109 FEMS memory-pack + review-gated proposal storage.
//!
//! Durable PostgreSQL authority for the Front End Memory System (FEMS) surfaces the
//! native editors read/write:
//!
//! * `fems_memory_packs`   — a seeded/generated [`crate::ace::MemoryPack`] the
//!   `GET /workspaces/{ws}/memory/pack` route returns (AC-109-2). The pack JSON is
//!   the REAL `ace::MemoryPack` shape (`items[].memory_id` / `memory_class` /
//!   `source_refs`) so the native client has a pinned contract.
//! * `fems_memory_proposals` — review-gated memory-write PROPOSALS submitted from the
//!   editor (`POST /workspaces/{ws}/memory/proposals`, AC-109-3). Every row lands as
//!   `status='pending_review'`; there is NO code path here that promotes a proposal
//!   into a committed memory item — the commit is downstream + review-gated.
//! * `fems_memory_items`   — COMMITTED memory items. This table exists so the AC-109-3
//!   negative proof can assert that submitting a proposal does NOT mutate committed
//!   memory. The proposals intake NEVER writes here; only a downstream review/commit
//!   path (out of MT-109 scope) would.
//!
//! PostgreSQL/EventLedger authority only — NO SQLite. JSONB columns are written as
//! canonical text with an explicit `::jsonb` cast and read back via `::text`, mirroring
//! the kernel event-ledger append/read pattern so no sqlx jsonb-codec feature is assumed.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};

use crate::ace::MemoryPack;
use crate::storage::{StorageError, StorageResult};

/// Create the FEMS memory tables if they do not already exist. Idempotent
/// (`CREATE TABLE IF NOT EXISTS`), safe to call at the start of every request and in
/// each isolated test schema.
pub async fn ensure_fems_memory_schema(pool: &PgPool) -> StorageResult<()> {
    let statements = [
        r#"
        CREATE TABLE IF NOT EXISTS fems_memory_packs (
            pack_id       TEXT PRIMARY KEY,
            workspace_id  TEXT NOT NULL,
            scope_key     TEXT NOT NULL DEFAULT '',
            pack          JSONB NOT NULL,
            generated_at  TIMESTAMPTZ NOT NULL,
            created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_fems_memory_packs_scope ON fems_memory_packs (workspace_id, scope_key, created_at DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS fems_memory_proposals (
            proposal_id      TEXT PRIMARY KEY,
            workspace_id     TEXT NOT NULL,
            document_id      TEXT NOT NULL,
            selection_start  BIGINT NOT NULL,
            selection_end    BIGINT NOT NULL,
            content_hash     TEXT NOT NULL,
            memory_class     TEXT NOT NULL,
            status           TEXT NOT NULL DEFAULT 'pending_review',
            review_gated     BOOLEAN NOT NULL DEFAULT TRUE,
            proposal         JSONB NOT NULL,
            created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_fems_memory_proposals_ws_status ON fems_memory_proposals (workspace_id, status, created_at DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS fems_memory_items (
            memory_id     TEXT PRIMARY KEY,
            workspace_id  TEXT NOT NULL,
            item          JSONB NOT NULL,
            created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_fems_memory_items_ws ON fems_memory_items (workspace_id)",
    ];

    for statement in statements {
        sqlx::query(statement).execute(pool).await?;
    }

    Ok(())
}

fn to_jsonb_text<T: Serialize>(value: &T) -> StorageResult<String> {
    serde_json::to_string(value).map_err(|err| StorageError::Serialization(err.to_string()))
}

// ---------------------------------------------------------------------------
// Memory packs (AC-109-2).
// ---------------------------------------------------------------------------

/// Insert or replace a stored memory pack for `(workspace_id, scope_key)`, keyed by the
/// pack's own `pack_id`. The pack JSON is the REAL `ace::MemoryPack` shape.
pub async fn upsert_memory_pack(
    pool: &PgPool,
    workspace_id: &str,
    scope_key: &str,
    pack: &MemoryPack,
) -> StorageResult<()> {
    ensure_fems_memory_schema(pool).await?;
    let pack_json = to_jsonb_text(pack)?;
    let generated_at = chrono::DateTime::parse_from_rfc3339(&pack.generated_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    sqlx::query(
        r#"
        INSERT INTO fems_memory_packs (pack_id, workspace_id, scope_key, pack, generated_at)
        VALUES ($1, $2, $3, $4::jsonb, $5)
        ON CONFLICT (pack_id) DO UPDATE SET
            workspace_id = EXCLUDED.workspace_id,
            scope_key = EXCLUDED.scope_key,
            pack = EXCLUDED.pack,
            generated_at = EXCLUDED.generated_at
        "#,
    )
    .bind(&pack.pack_id)
    .bind(workspace_id)
    .bind(scope_key)
    .bind(pack_json)
    .bind(generated_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch the most recently created memory pack for `workspace_id`, optionally narrowed to
/// a `scope_key`. Returns the REAL `ace::MemoryPack` decoded from the stored JSON, or
/// `None` when the workspace has no stored pack yet.
pub async fn get_latest_memory_pack(
    pool: &PgPool,
    workspace_id: &str,
    scope_key: Option<&str>,
) -> StorageResult<Option<MemoryPack>> {
    ensure_fems_memory_schema(pool).await?;
    let row = if let Some(scope) = scope_key.filter(|s| !s.trim().is_empty()) {
        sqlx::query(
            r#"
            SELECT pack::text AS pack
            FROM fems_memory_packs
            WHERE workspace_id = $1 AND scope_key = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .bind(scope)
        .fetch_optional(pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT pack::text AS pack
            FROM fems_memory_packs
            WHERE workspace_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
    };

    match row {
        Some(row) => {
            let pack_text: String = row.try_get("pack")?;
            let pack: MemoryPack = serde_json::from_str(&pack_text)
                .map_err(|err| StorageError::Serialization(err.to_string()))?;
            Ok(Some(pack))
        }
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Review-gated proposals (AC-109-3).
// ---------------------------------------------------------------------------

/// A review-gated proposal row as stored/read from `fems_memory_proposals`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredMemoryProposal {
    pub proposal_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub selection_start: i64,
    pub selection_end: i64,
    pub content_hash: String,
    pub memory_class: String,
    pub status: String,
    pub review_gated: bool,
    pub proposal: Value,
}

/// Insert a review-gated proposal. Always stored as `status='pending_review'`; this
/// function NEVER writes to `fems_memory_items` (the never-editor-direct invariant).
pub async fn insert_memory_proposal(
    pool: &PgPool,
    proposal: &StoredMemoryProposal,
) -> StorageResult<()> {
    ensure_fems_memory_schema(pool).await?;
    let proposal_json = to_jsonb_text(&proposal.proposal)?;
    sqlx::query(
        r#"
        INSERT INTO fems_memory_proposals (
            proposal_id, workspace_id, document_id, selection_start, selection_end,
            content_hash, memory_class, status, review_gated, proposal
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb)
        "#,
    )
    .bind(&proposal.proposal_id)
    .bind(&proposal.workspace_id)
    .bind(&proposal.document_id)
    .bind(proposal.selection_start)
    .bind(proposal.selection_end)
    .bind(&proposal.content_hash)
    .bind(&proposal.memory_class)
    .bind(&proposal.status)
    .bind(proposal.review_gated)
    .bind(proposal_json)
    .execute(pool)
    .await?;
    Ok(())
}

/// Read a stored proposal by id (used by the AC-109-3 proofs).
pub async fn get_memory_proposal(
    pool: &PgPool,
    proposal_id: &str,
) -> StorageResult<Option<StoredMemoryProposal>> {
    ensure_fems_memory_schema(pool).await?;
    let row = sqlx::query(
        r#"
        SELECT proposal_id, workspace_id, document_id, selection_start, selection_end,
               content_hash, memory_class, status, review_gated, proposal::text AS proposal
        FROM fems_memory_proposals
        WHERE proposal_id = $1
        "#,
    )
    .bind(proposal_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let proposal_text: String = row.try_get("proposal")?;
            let proposal_value: Value = serde_json::from_str(&proposal_text)
                .map_err(|err| StorageError::Serialization(err.to_string()))?;
            Ok(Some(StoredMemoryProposal {
                proposal_id: row.try_get("proposal_id")?,
                workspace_id: row.try_get("workspace_id")?,
                document_id: row.try_get("document_id")?,
                selection_start: row.try_get("selection_start")?,
                selection_end: row.try_get("selection_end")?,
                content_hash: row.try_get("content_hash")?,
                memory_class: row.try_get("memory_class")?,
                status: row.try_get("status")?,
                review_gated: row.try_get("review_gated")?,
                proposal: proposal_value,
            }))
        }
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Committed memory items (proposals must NEVER write here — AC-109-3 negative proof).
// ---------------------------------------------------------------------------

/// Insert or replace a COMMITTED memory item. This is only reachable from a downstream
/// review/commit path (out of MT-109 scope) or a test seeding a committed item to prove
/// a proposal cannot mutate it.
pub async fn upsert_memory_item(
    pool: &PgPool,
    workspace_id: &str,
    memory_id: &str,
    item: &Value,
) -> StorageResult<()> {
    ensure_fems_memory_schema(pool).await?;
    let item_json = to_jsonb_text(item)?;
    sqlx::query(
        r#"
        INSERT INTO fems_memory_items (memory_id, workspace_id, item)
        VALUES ($1, $2, $3::jsonb)
        ON CONFLICT (memory_id) DO UPDATE SET
            workspace_id = EXCLUDED.workspace_id,
            item = EXCLUDED.item,
            updated_at = now()
        "#,
    )
    .bind(memory_id)
    .bind(workspace_id)
    .bind(item_json)
    .execute(pool)
    .await?;
    Ok(())
}

/// Read a committed memory item's JSON by id (AC-109-3 negative proof: unchanged after a
/// proposal is submitted).
pub async fn get_memory_item(
    pool: &PgPool,
    memory_id: &str,
) -> StorageResult<Option<Value>> {
    ensure_fems_memory_schema(pool).await?;
    let row = sqlx::query(
        r#"
        SELECT item::text AS item
        FROM fems_memory_items
        WHERE memory_id = $1
        "#,
    )
    .bind(memory_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let item_text: String = row.try_get("item")?;
            let item: Value = serde_json::from_str(&item_text)
                .map_err(|err| StorageError::Serialization(err.to_string()))?;
            Ok(Some(item))
        }
        None => Ok(None),
    }
}

/// Count committed memory items for a workspace (AC-109-3 negative proof: submitting a
/// proposal does not increase the committed-item count).
pub async fn count_memory_items(pool: &PgPool, workspace_id: &str) -> StorageResult<i64> {
    ensure_fems_memory_schema(pool).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM fems_memory_items WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

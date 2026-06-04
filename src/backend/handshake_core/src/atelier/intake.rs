//! Intake / inbox sorting (MT-016): persistent intake batches and per-item
//! accept / reject / defer lanes for the operator's "IntakeSorterView" flow.
//!
//! CKC source: `app/backend/library.js` (`createIntakeBatch`,
//! `listIntakeBatches`, `getIntakeBatch`, `updateIntakeBatchItem`,
//! `classifyIntakeBatch`, `_normalizeIntakeStatus`) and `app/backend/db.js`
//! (`IntakeBatch` / `IntakeBatchItem` tables). Schema/behavior INTENT only;
//! the SQLite originals are not copied. Storage authority is PostgreSQL.
//!
//! Translated contract (the load-bearing invariants from CKC):
//!   * Persistent batches: a scan produces a durable `atelier_intake_batch`
//!     plus one `atelier_intake_item` per source file; nothing is ephemeral.
//!   * Accept / reject / defer lanes: CKC's `accepted` / `rejected` / `pending`
//!     review lanes become item lane states. "defer" == CKC "pending".
//!   * Idempotency: re-scanning the same source is safe. A batch carries a
//!     unique `idempotency_key`; re-creating with the same key returns the
//!     existing batch. Items are unique per `(batch, source_path)`; re-adding
//!     the same source returns the existing item instead of duplicating it.
//!   * Source preservation / no silent deletes: rejecting an item only moves
//!     its lane; the row and its `source_path` are always retained so the
//!     original is never lost. There is no delete path in this module.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierError, AtelierResult, AtelierStore};

/// Intake event families (extends the MT-005 coverage set). The parent wires
/// these into [`super::event_family::ALL`].
pub mod intake_event_family {
    /// A persistent intake batch was opened.
    pub const INTAKE_BATCH_CREATED: &str = "atelier.intake.batch_created";
    /// A source file was registered as an item in a batch.
    pub const INTAKE_ITEM_ADDED: &str = "atelier.intake.item_added";
    /// An item moved into a lane (accepted / rejected / deferred / new).
    pub const INTAKE_ITEM_CLASSIFIED: &str = "atelier.intake.item_classified";
    /// A batch was closed after its items were triaged.
    pub const INTAKE_BATCH_CLOSED: &str = "atelier.intake.batch_closed";

    /// Intake event families, exported for parity/coverage proofs.
    pub const ALL: &[&str] = &[
        INTAKE_BATCH_CREATED,
        INTAKE_ITEM_ADDED,
        INTAKE_ITEM_CLASSIFIED,
        INTAKE_BATCH_CLOSED,
    ];
}

/// Triage lane for an intake item. Mirrors CKC's review-status lanes; `Deferred`
/// is CKC's `pending`. `New` is the untriaged inbox lane.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntakeLane {
    New,
    Accepted,
    Rejected,
    Deferred,
}

impl IntakeLane {
    /// Canonical lowercase database token.
    pub fn as_str(self) -> &'static str {
        match self {
            IntakeLane::New => "new",
            IntakeLane::Accepted => "accepted",
            IntakeLane::Rejected => "rejected",
            IntakeLane::Deferred => "deferred",
        }
    }

    /// Parse a lane token, accepting the CKC aliases (`pass`/`accept` ->
    /// accepted, `reject` -> rejected, `pending` -> deferred).
    pub fn parse(raw: &str) -> AtelierResult<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "new" => Ok(IntakeLane::New),
            "accepted" | "accept" | "pass" => Ok(IntakeLane::Accepted),
            "rejected" | "reject" => Ok(IntakeLane::Rejected),
            "deferred" | "defer" | "pending" => Ok(IntakeLane::Deferred),
            other => Err(AtelierError::Validation(format!(
                "intake lane must be new/accepted/rejected/deferred, got {other:?}"
            ))),
        }
    }
}

/// Lifecycle status of a persistent intake batch.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Open and accepting triage decisions.
    Open,
    /// All triage complete with no leftover `New` items.
    Closed,
}

impl BatchStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BatchStatus::Open => "open",
            BatchStatus::Closed => "closed",
        }
    }

    fn parse(raw: &str) -> BatchStatus {
        match raw.trim().to_ascii_lowercase().as_str() {
            "closed" => BatchStatus::Closed,
            _ => BatchStatus::Open,
        }
    }
}

/// A persistent intake batch produced by a scan of a source.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeBatch {
    pub batch_id: Uuid,
    /// Stable operator-facing label for the source scan (e.g. a folder path or
    /// sourcing-run id). Unique; doubles as the idempotency key.
    pub idempotency_key: String,
    /// Human-facing description of where the batch came from.
    pub source_label: String,
    /// Optional owning character (FK to `atelier_character.internal_id`).
    pub character_internal_id: Option<Uuid>,
    pub status: BatchStatus,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// One source file registered inside a batch. Always retains `source_path` so
/// the original is never lost (no silent deletes).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeItem {
    pub item_id: Uuid,
    pub batch_id: Uuid,
    /// Preserved path/URI of the original source; never mutated by triage.
    pub source_path: String,
    pub file_name: String,
    pub byte_len: i64,
    /// Optional content hash for cross-item dedup hints.
    pub content_hash: Option<String>,
    pub lane: IntakeLane,
    /// Free-form reason captured when an item is rejected/deferred.
    pub lane_reason: Option<String>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input to open (or idempotently re-open) a batch.
#[derive(Clone, Debug)]
pub struct NewIntakeBatch {
    pub idempotency_key: String,
    pub source_label: String,
    pub character_internal_id: Option<Uuid>,
}

/// Input to register a source file as an intake item.
#[derive(Clone, Debug)]
pub struct NewIntakeItem {
    pub source_path: String,
    pub file_name: String,
    pub byte_len: i64,
    pub content_hash: Option<String>,
}

/// Per-lane counts for a batch, used by the sorter view header.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IntakeLaneCounts {
    pub new: i64,
    pub accepted: i64,
    pub rejected: i64,
    pub deferred: i64,
}

fn batch_from_row(row: &sqlx::postgres::PgRow) -> IntakeBatch {
    let status: String = row.get("status");
    IntakeBatch {
        batch_id: row.get("batch_id"),
        idempotency_key: row.get("idempotency_key"),
        source_label: row.get("source_label"),
        character_internal_id: row.get("character_internal_id"),
        status: BatchStatus::parse(&status),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn item_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<IntakeItem> {
    let lane: String = row.get("lane");
    Ok(IntakeItem {
        item_id: row.get("item_id"),
        batch_id: row.get("batch_id"),
        source_path: row.get("source_path"),
        file_name: row.get("file_name"),
        byte_len: row.get("byte_len"),
        content_hash: row.get("content_hash"),
        lane: IntakeLane::parse(&lane)?,
        lane_reason: row.get("lane_reason"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

impl AtelierStore {
    /// Open a persistent intake batch, or return the existing one for the same
    /// `idempotency_key`. Re-scanning the same source is therefore safe and
    /// never creates a duplicate batch (CKC `createIntakeBatch` intent).
    pub async fn open_intake_batch(&self, new: &NewIntakeBatch) -> AtelierResult<IntakeBatch> {
        if new.idempotency_key.trim().is_empty() {
            return Err(AtelierError::Validation(
                "idempotency_key must not be empty".into(),
            ));
        }

        // Idempotent fast path: an existing batch with this key wins.
        if let Some(existing) = self.get_intake_batch_by_key(&new.idempotency_key).await? {
            return Ok(existing);
        }

        let row = sqlx::query(
            r#"INSERT INTO atelier_intake_batch
                 (idempotency_key, source_label, character_internal_id, status)
               VALUES ($1, $2, $3, 'open')
               ON CONFLICT (idempotency_key) DO UPDATE
                 SET idempotency_key = EXCLUDED.idempotency_key
               RETURNING batch_id, idempotency_key, source_label,
                         character_internal_id, status, created_at_utc, updated_at_utc"#,
        )
        .bind(&new.idempotency_key)
        .bind(&new.source_label)
        .bind(new.character_internal_id)
        .fetch_one(self.pool())
        .await?;
        let batch = batch_from_row(&row);

        self.record_event(
            intake_event_family::INTAKE_BATCH_CREATED,
            "atelier_intake_batch",
            &batch.idempotency_key,
            serde_json::json!({
                "batch_id": batch.batch_id,
                "source_label": batch.source_label,
                "character_internal_id": batch.character_internal_id,
            }),
        )
        .await?;
        Ok(batch)
    }

    /// Fetch a batch by its stable idempotency key.
    pub async fn get_intake_batch_by_key(
        &self,
        idempotency_key: &str,
    ) -> AtelierResult<Option<IntakeBatch>> {
        let row = sqlx::query(
            r#"SELECT batch_id, idempotency_key, source_label, character_internal_id,
                      status, created_at_utc, updated_at_utc
               FROM atelier_intake_batch WHERE idempotency_key = $1"#,
        )
        .bind(idempotency_key)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(batch_from_row))
    }

    /// List batches, newest first, optionally filtered by status.
    pub async fn list_intake_batches(
        &self,
        status: Option<BatchStatus>,
        limit: i64,
    ) -> AtelierResult<Vec<IntakeBatch>> {
        let capped = limit.clamp(1, 1000);
        let rows = sqlx::query(
            r#"SELECT batch_id, idempotency_key, source_label, character_internal_id,
                      status, created_at_utc, updated_at_utc
               FROM atelier_intake_batch
               WHERE ($1::TEXT IS NULL OR status = $1)
               ORDER BY updated_at_utc DESC
               LIMIT $2"#,
        )
        .bind(status.map(|s| s.as_str()))
        .bind(capped)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(batch_from_row).collect())
    }

    /// Register a source file in a batch, idempotently. Re-adding the same
    /// `(batch, source_path)` returns the existing item rather than creating a
    /// duplicate, and never mutates its lane (source preservation). Items always
    /// enter the `New` lane.
    pub async fn add_intake_item(
        &self,
        batch_id: Uuid,
        new: &NewIntakeItem,
    ) -> AtelierResult<IntakeItem> {
        if new.source_path.trim().is_empty() {
            return Err(AtelierError::Validation("source_path must not be empty".into()));
        }
        // Idempotent fast path: existing item for this source in this batch.
        if let Some(existing) = self.get_intake_item(batch_id, &new.source_path).await? {
            return Ok(existing);
        }

        let mut tx = self.pool().begin().await?;

        // Guard the FK explicitly so a bad batch_id is a clean validation error
        // rather than a raw constraint violation.
        let batch_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT batch_id FROM atelier_intake_batch WHERE batch_id = $1")
                .bind(batch_id)
                .fetch_optional(&mut *tx)
                .await?;
        if batch_exists.is_none() {
            return Err(AtelierError::NotFound(format!("intake batch {batch_id}")));
        }

        let row = sqlx::query(
            r#"INSERT INTO atelier_intake_item
                 (batch_id, source_path, file_name, byte_len, content_hash, lane)
               VALUES ($1, $2, $3, $4, $5, 'new')
               ON CONFLICT (batch_id, source_path) DO UPDATE
                 SET source_path = EXCLUDED.source_path
               RETURNING item_id, batch_id, source_path, file_name, byte_len,
                         content_hash, lane, lane_reason, created_at_utc, updated_at_utc"#,
        )
        .bind(batch_id)
        .bind(&new.source_path)
        .bind(&new.file_name)
        .bind(new.byte_len)
        .bind(&new.content_hash)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE atelier_intake_batch SET updated_at_utc = NOW() WHERE batch_id = $1")
            .bind(batch_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        let item = item_from_row(&row)?;
        self.record_event(
            intake_event_family::INTAKE_ITEM_ADDED,
            "atelier_intake_item",
            &item.item_id.to_string(),
            serde_json::json!({
                "batch_id": item.batch_id,
                "source_path": item.source_path,
                "file_name": item.file_name,
                "byte_len": item.byte_len,
            }),
        )
        .await?;
        Ok(item)
    }

    /// Fetch a single item by its preserved source path within a batch.
    pub async fn get_intake_item(
        &self,
        batch_id: Uuid,
        source_path: &str,
    ) -> AtelierResult<Option<IntakeItem>> {
        let row = sqlx::query(
            r#"SELECT item_id, batch_id, source_path, file_name, byte_len,
                      content_hash, lane, lane_reason, created_at_utc, updated_at_utc
               FROM atelier_intake_item
               WHERE batch_id = $1 AND source_path = $2"#,
        )
        .bind(batch_id)
        .bind(source_path)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(item_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// List the items in a batch (creation order), optionally filtered to a lane.
    pub async fn list_intake_items(
        &self,
        batch_id: Uuid,
        lane: Option<IntakeLane>,
    ) -> AtelierResult<Vec<IntakeItem>> {
        let rows = sqlx::query(
            r#"SELECT item_id, batch_id, source_path, file_name, byte_len,
                      content_hash, lane, lane_reason, created_at_utc, updated_at_utc
               FROM atelier_intake_item
               WHERE batch_id = $1 AND ($2::TEXT IS NULL OR lane = $2)
               ORDER BY created_at_utc ASC"#,
        )
        .bind(batch_id)
        .bind(lane.map(|l| l.as_str()))
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(item_from_row).collect()
    }

    /// Move an item into a triage lane (accept / reject / defer / new). This is
    /// the only state change triage performs: the source row is preserved and
    /// never deleted, only its lane and reason change.
    pub async fn classify_intake_item(
        &self,
        item_id: Uuid,
        lane: IntakeLane,
        reason: Option<&str>,
    ) -> AtelierResult<IntakeItem> {
        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(
            r#"UPDATE atelier_intake_item
               SET lane = $2, lane_reason = $3, updated_at_utc = NOW()
               WHERE item_id = $1
               RETURNING item_id, batch_id, source_path, file_name, byte_len,
                         content_hash, lane, lane_reason, created_at_utc, updated_at_utc"#,
        )
        .bind(item_id)
        .bind(lane.as_str())
        .bind(reason)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("intake item {item_id}")))?;

        let item = item_from_row(&row)?;

        sqlx::query("UPDATE atelier_intake_batch SET updated_at_utc = NOW() WHERE batch_id = $1")
            .bind(item.batch_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        self.record_event(
            intake_event_family::INTAKE_ITEM_CLASSIFIED,
            "atelier_intake_item",
            &item.item_id.to_string(),
            serde_json::json!({
                "batch_id": item.batch_id,
                "lane": item.lane,
                "reason": item.lane_reason,
                "source_path": item.source_path,
            }),
        )
        .await?;
        Ok(item)
    }

    /// Per-lane counts for the sorter header.
    pub async fn intake_lane_counts(&self, batch_id: Uuid) -> AtelierResult<IntakeLaneCounts> {
        let rows = sqlx::query(
            r#"SELECT lane, COUNT(*) AS n
               FROM atelier_intake_item
               WHERE batch_id = $1
               GROUP BY lane"#,
        )
        .bind(batch_id)
        .fetch_all(self.pool())
        .await?;

        let mut counts = IntakeLaneCounts::default();
        for row in &rows {
            let lane: String = row.get("lane");
            let n: i64 = row.get("n");
            match IntakeLane::parse(&lane)? {
                IntakeLane::New => counts.new = n,
                IntakeLane::Accepted => counts.accepted = n,
                IntakeLane::Rejected => counts.rejected = n,
                IntakeLane::Deferred => counts.deferred = n,
            }
        }
        Ok(counts)
    }

    /// Close a batch once triage is done. Refuses to close while any item is
    /// still in the `New` lane, so nothing is silently dropped. Returns the
    /// updated batch.
    pub async fn close_intake_batch(&self, batch_id: Uuid) -> AtelierResult<IntakeBatch> {
        let counts = self.intake_lane_counts(batch_id).await?;
        if counts.new > 0 {
            return Err(AtelierError::Validation(format!(
                "cannot close intake batch {batch_id}: {} item(s) still in the new lane",
                counts.new
            )));
        }

        let row = sqlx::query(
            r#"UPDATE atelier_intake_batch
               SET status = 'closed', updated_at_utc = NOW()
               WHERE batch_id = $1
               RETURNING batch_id, idempotency_key, source_label, character_internal_id,
                         status, created_at_utc, updated_at_utc"#,
        )
        .bind(batch_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("intake batch {batch_id}")))?;
        let batch = batch_from_row(&row);

        self.record_event(
            intake_event_family::INTAKE_BATCH_CLOSED,
            "atelier_intake_batch",
            &batch.idempotency_key,
            serde_json::json!({
                "batch_id": batch.batch_id,
                "accepted": counts.accepted,
                "rejected": counts.rejected,
                "deferred": counts.deferred,
            }),
        )
        .await?;
        Ok(batch)
    }
}

//! PostgreSQL store for the ingestion-owned tables (migrations 0160-0169).
//!
//! Pattern: like `AtelierStore`, this store owns its SQL over the shared
//! `AppState::postgres_pool`. Rows in the pre-existing knowledge tables
//! (`knowledge_source_roots`, `knowledge_sources`, 0131/0132) are NOT written
//! here — the engine goes through `storage::knowledge::KnowledgeStore` for
//! those, so there is exactly one SQL authority per table.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};

use super::allowlist::{PolicyVerdictKind, RootRegistrationPolicy};
use super::receipts::{ExtractionReceipt, NewExtractionReceipt};
use super::repair::{NewRepairEntry, RepairAttemptOutcome, RepairEntry, RepairState};
use super::spans::{ExtractedSpan, SpanAnchor};
use super::{new_ingestion_id, IngestionError, IngestionResult};
use crate::ai_ready_data::chunking::sha256_hex;

/// Store over the ingestion-owned `knowledge_ingestion_*` tables.
#[derive(Clone)]
pub struct KnowledgeIngestionStore {
    pool: PgPool,
}

// ---------------------------------------------------------------------------
// MT-081 root registration policies + decisions.
// ---------------------------------------------------------------------------

/// Durable row of `knowledge_ingestion_root_policies`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRootPolicy {
    pub policy_id: String,
    pub workspace_id: String,
    pub policy_version: i32,
    pub policy: RootRegistrationPolicy,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Durable row of `knowledge_ingestion_policy_decisions`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDecision {
    pub decision_id: String,
    pub workspace_id: String,
    pub policy_id: Option<String>,
    pub candidate_path: String,
    pub root_kind: String,
    pub verdict: PolicyVerdictKind,
    pub matched_pattern: Option<String>,
    pub operator_approved: bool,
    pub actor_kind: String,
    pub actor_id: String,
    pub receipt_event_id: Option<String>,
    pub decided_at: DateTime<Utc>,
}

/// Insert payload for [`PolicyDecision`].
#[derive(Clone, Debug)]
pub struct NewPolicyDecision {
    pub workspace_id: String,
    pub policy_id: Option<String>,
    pub candidate_path: String,
    pub root_kind: String,
    pub verdict: PolicyVerdictKind,
    pub matched_pattern: Option<String>,
    pub operator_approved: bool,
    pub actor_kind: String,
    pub actor_id: String,
    pub receipt_event_id: Option<String>,
}

fn patterns_from_value(value: &Value, field: &str) -> IngestionResult<Vec<String>> {
    let Some(items) = value.as_array() else {
        return Err(IngestionError::Validation(format!(
            "policy {field} must be a JSON array"
        )));
    };
    items
        .iter()
        .map(|item| {
            item.as_str().map(|s| s.to_string()).ok_or_else(|| {
                IngestionError::Validation(format!("policy {field} entries must be strings"))
            })
        })
        .collect()
}

fn stored_policy_from_pg(row: &sqlx::postgres::PgRow) -> IngestionResult<StoredRootPolicy> {
    let allow: Value = row.get("allow_patterns");
    let deny: Value = row.get("deny_patterns");
    Ok(StoredRootPolicy {
        policy_id: row.get("policy_id"),
        workspace_id: row.get("workspace_id"),
        policy_version: row.get("policy_version"),
        policy: RootRegistrationPolicy {
            allow_patterns: patterns_from_value(&allow, "allow_patterns")?,
            deny_patterns: patterns_from_value(&deny, "deny_patterns")?,
            require_operator_approval: row.get("require_operator_approval"),
        },
        active: row.get("active"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn decision_from_pg(row: &sqlx::postgres::PgRow) -> IngestionResult<PolicyDecision> {
    Ok(PolicyDecision {
        decision_id: row.get("decision_id"),
        workspace_id: row.get("workspace_id"),
        policy_id: row.get("policy_id"),
        candidate_path: row.get("candidate_path"),
        root_kind: row.get("root_kind"),
        verdict: row.get::<String, _>("verdict").parse()?,
        matched_pattern: row.get("matched_pattern"),
        operator_approved: row.get("operator_approved"),
        actor_kind: row.get("actor_kind"),
        actor_id: row.get("actor_id"),
        receipt_event_id: row.get("receipt_event_id"),
        decided_at: row.get("decided_at"),
    })
}

const POLICY_COLUMNS: &str = "policy_id, workspace_id, policy_version, allow_patterns, \
     deny_patterns, require_operator_approval, active, created_at, updated_at";

const DECISION_COLUMNS: &str = "decision_id, workspace_id, policy_id, candidate_path, root_kind, \
     verdict, matched_pattern, operator_approved, actor_kind, actor_id, receipt_event_id, \
     decided_at";

impl KnowledgeIngestionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Activate a new policy version for the workspace. The previous active
    /// policy (if any) is kept as an inactive row so historical decisions
    /// retain their FK context; its version seeds the new version number.
    pub async fn activate_root_policy(
        &self,
        workspace_id: &str,
        policy: &RootRegistrationPolicy,
    ) -> IngestionResult<StoredRootPolicy> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::storage::StorageError::from)?;

        let previous_version: Option<i32> = sqlx::query_scalar(
            "UPDATE knowledge_ingestion_root_policies
             SET active = FALSE, updated_at = NOW()
             WHERE workspace_id = $1 AND active
             RETURNING policy_version",
        )
        .bind(workspace_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(crate::storage::StorageError::from)?;

        let policy_id = new_ingestion_id("KIP");
        let sql = format!(
            "INSERT INTO knowledge_ingestion_root_policies
                 (policy_id, workspace_id, policy_version, allow_patterns,
                  deny_patterns, require_operator_approval)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING {POLICY_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(&policy_id)
            .bind(workspace_id)
            .bind(previous_version.unwrap_or(0) + 1)
            .bind(serde_json::json!(policy.allow_patterns))
            .bind(serde_json::json!(policy.deny_patterns))
            .bind(policy.require_operator_approval)
            .fetch_one(&mut *tx)
            .await
            .map_err(crate::storage::StorageError::from)?;
        let stored = stored_policy_from_pg(&row)?;

        tx.commit()
            .await
            .map_err(crate::storage::StorageError::from)?;
        Ok(stored)
    }

    /// The active policy for a workspace, if one was configured.
    pub async fn get_active_root_policy(
        &self,
        workspace_id: &str,
    ) -> IngestionResult<Option<StoredRootPolicy>> {
        let sql = format!(
            "SELECT {POLICY_COLUMNS} FROM knowledge_ingestion_root_policies
             WHERE workspace_id = $1 AND active"
        );
        let row = sqlx::query(&sql)
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?;
        row.as_ref().map(stored_policy_from_pg).transpose()
    }

    /// Persist one policy evaluation outcome (allowed or denied).
    pub async fn record_policy_decision(
        &self,
        decision: NewPolicyDecision,
    ) -> IngestionResult<PolicyDecision> {
        let decision_id = new_ingestion_id("KIPD");
        let sql = format!(
            "INSERT INTO knowledge_ingestion_policy_decisions
                 (decision_id, workspace_id, policy_id, candidate_path, root_kind,
                  verdict, matched_pattern, operator_approved, actor_kind, actor_id,
                  receipt_event_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             RETURNING {DECISION_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(&decision_id)
            .bind(&decision.workspace_id)
            .bind(&decision.policy_id)
            .bind(&decision.candidate_path)
            .bind(&decision.root_kind)
            .bind(decision.verdict.as_str())
            .bind(&decision.matched_pattern)
            .bind(decision.operator_approved)
            .bind(&decision.actor_kind)
            .bind(&decision.actor_id)
            .bind(&decision.receipt_event_id)
            .fetch_one(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?;
        decision_from_pg(&row)
    }

    /// Decisions for a workspace, newest first.
    pub async fn list_policy_decisions(
        &self,
        workspace_id: &str,
        limit: i64,
    ) -> IngestionResult<Vec<PolicyDecision>> {
        let sql = format!(
            "SELECT {DECISION_COLUMNS} FROM knowledge_ingestion_policy_decisions
             WHERE workspace_id = $1 ORDER BY decided_at DESC, decision_id DESC LIMIT $2"
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?;
        rows.iter().map(decision_from_pg).collect()
    }

    // -- MT-085 extraction receipts ------------------------------------------

    /// Persist one extraction-attempt receipt.
    pub async fn record_extraction_receipt(
        &self,
        receipt: NewExtractionReceipt,
        receipt_event_id: Option<&str>,
    ) -> IngestionResult<ExtractionReceipt> {
        receipt.validate()?;
        let receipt_id = new_ingestion_id("KIRC");
        let sql = format!(
            "INSERT INTO knowledge_ingestion_receipts
                 (receipt_id, workspace_id, source_id, ingestion_run_token,
                  extractor_id, extractor_version, status, error_class,
                  error_detail, spans_produced, spans_failed, redaction_count,
                  content_hash, duration_ms, receipt_event_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             RETURNING {RECEIPT_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(&receipt_id)
            .bind(&receipt.workspace_id)
            .bind(&receipt.source_id)
            .bind(&receipt.ingestion_run_token)
            .bind(&receipt.extractor_id)
            .bind(&receipt.extractor_version)
            .bind(receipt.status.as_str())
            .bind(receipt.error_class.map(|c| c.as_str()))
            .bind(&receipt.error_detail)
            .bind(receipt.spans_produced)
            .bind(receipt.spans_failed)
            .bind(receipt.redaction_count)
            .bind(&receipt.content_hash)
            .bind(receipt.duration_ms)
            .bind(receipt_event_id)
            .fetch_one(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?;
        receipt_from_pg(&row)
    }

    /// Receipts for one source, newest first.
    pub async fn list_extraction_receipts(
        &self,
        source_id: &str,
        limit: i64,
    ) -> IngestionResult<Vec<ExtractionReceipt>> {
        let sql = format!(
            "SELECT {RECEIPT_COLUMNS} FROM knowledge_ingestion_receipts
             WHERE source_id = $1 ORDER BY created_at DESC, receipt_id DESC LIMIT $2"
        );
        let rows = sqlx::query(&sql)
            .bind(source_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?;
        rows.iter().map(receipt_from_pg).collect()
    }

    pub async fn get_extraction_receipt(
        &self,
        receipt_id: &str,
    ) -> IngestionResult<Option<ExtractionReceipt>> {
        let sql = format!(
            "SELECT {RECEIPT_COLUMNS} FROM knowledge_ingestion_receipts WHERE receipt_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(receipt_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?;
        row.as_ref().map(receipt_from_pg).transpose()
    }

    // -- MT-087..MT-091 ingestion spans --------------------------------------

    /// Replace the stored spans of a source with the spans of a new
    /// extraction attempt (receipt). Old spans are deleted (the receipt
    /// trail of prior attempts remains); the new spans insert atomically.
    pub async fn replace_source_spans(
        &self,
        workspace_id: &str,
        source_id: &str,
        receipt_id: &str,
        spans: &[ExtractedSpan],
    ) -> IngestionResult<Vec<StoredSpan>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::storage::StorageError::from)?;
        sqlx::query("DELETE FROM knowledge_ingestion_spans WHERE source_id = $1")
            .bind(source_id)
            .execute(&mut *tx)
            .await
            .map_err(crate::storage::StorageError::from)?;

        let sql = format!(
            "INSERT INTO knowledge_ingestion_spans
                 (span_id, workspace_id, source_id, receipt_id, span_index,
                  anchor_kind, anchor, byte_start, byte_end, content,
                  content_hash, redaction_state, link_candidates)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             RETURNING {SPAN_COLUMNS}"
        );
        let mut stored = Vec::with_capacity(spans.len());
        for (index, span) in spans.iter().enumerate() {
            let span_id = new_ingestion_id("KISP");
            let content_hash = sha256_hex(span.content.as_bytes());
            let row = sqlx::query(&sql)
                .bind(&span_id)
                .bind(workspace_id)
                .bind(source_id)
                .bind(receipt_id)
                .bind(index as i32)
                .bind(span.anchor.kind_str())
                .bind(span.anchor.to_json())
                .bind(span.byte_start)
                .bind(span.byte_end)
                .bind(&span.content)
                .bind(&content_hash)
                .bind(span.redaction.as_str())
                .bind(serde_json::json!(span.link_candidates))
                .fetch_one(&mut *tx)
                .await
                .map_err(crate::storage::StorageError::from)?;
            stored.push(span_from_pg(&row)?);
        }
        tx.commit()
            .await
            .map_err(crate::storage::StorageError::from)?;
        Ok(stored)
    }

    /// Stored spans of a source in span order.
    pub async fn list_source_spans(&self, source_id: &str) -> IngestionResult<Vec<StoredSpan>> {
        let sql = format!(
            "SELECT {SPAN_COLUMNS} FROM knowledge_ingestion_spans
             WHERE source_id = $1 ORDER BY span_index"
        );
        let rows = sqlx::query(&sql)
            .bind(source_id)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?;
        rows.iter().map(span_from_pg).collect()
    }

    // -- MT-094 repair queue --------------------------------------------------

    /// Enqueue (or refresh) the OPEN repair entry for a source. If an open
    /// entry exists it is updated in place (reason, detail, receipt) instead
    /// of multiplying rows; terminal entries stay untouched and a new entry
    /// is created.
    pub async fn enqueue_repair(&self, entry: NewRepairEntry) -> IngestionResult<RepairEntry> {
        // Refresh an existing open entry first.
        let updated = {
            let sql = format!(
                "UPDATE knowledge_ingestion_repair_queue
                 SET reason_class = $2, reason_detail = $3, receipt_id = $4,
                     enqueue_event_id = COALESCE($5, enqueue_event_id),
                     updated_at = NOW()
                 WHERE source_id = $1 AND state IN ('queued', 'retrying')
                 RETURNING {REPAIR_COLUMNS}"
            );
            sqlx::query(&sql)
                .bind(&entry.source_id)
                .bind(entry.reason_class.as_str())
                .bind(&entry.reason_detail)
                .bind(&entry.receipt_id)
                .bind(&entry.enqueue_event_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(crate::storage::StorageError::from)?
        };
        if let Some(row) = updated {
            return repair_from_pg(&row);
        }

        let repair_id = new_ingestion_id("KIRQ");
        let sql = format!(
            "INSERT INTO knowledge_ingestion_repair_queue
                 (repair_id, workspace_id, source_id, receipt_id, reason_class,
                  reason_detail, max_attempts, enqueue_event_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING {REPAIR_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(&repair_id)
            .bind(&entry.workspace_id)
            .bind(&entry.source_id)
            .bind(&entry.receipt_id)
            .bind(entry.reason_class.as_str())
            .bind(&entry.reason_detail)
            .bind(entry.max_attempts)
            .bind(&entry.enqueue_event_id)
            .fetch_one(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?;
        repair_from_pg(&row)
    }

    pub async fn get_repair_entry(&self, repair_id: &str) -> IngestionResult<Option<RepairEntry>> {
        let sql = format!(
            "SELECT {REPAIR_COLUMNS} FROM knowledge_ingestion_repair_queue WHERE repair_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(repair_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?;
        row.as_ref().map(repair_from_pg).transpose()
    }

    /// Repair entries for a workspace, optionally filtered by state.
    pub async fn list_repair_entries(
        &self,
        workspace_id: &str,
        state: Option<RepairState>,
        limit: i64,
    ) -> IngestionResult<Vec<RepairEntry>> {
        let rows = match state {
            Some(state) => {
                let sql = format!(
                    "SELECT {REPAIR_COLUMNS} FROM knowledge_ingestion_repair_queue
                     WHERE workspace_id = $1 AND state = $2
                     ORDER BY created_at DESC LIMIT $3"
                );
                sqlx::query(&sql)
                    .bind(workspace_id)
                    .bind(state.as_str())
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await
            }
            None => {
                let sql = format!(
                    "SELECT {REPAIR_COLUMNS} FROM knowledge_ingestion_repair_queue
                     WHERE workspace_id = $1 ORDER BY created_at DESC LIMIT $2"
                );
                sqlx::query(&sql)
                    .bind(workspace_id)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await
            }
        }
        .map_err(crate::storage::StorageError::from)?;
        rows.iter().map(repair_from_pg).collect()
    }

    /// Claim an open entry for a retry: queued|retrying -> retrying with the
    /// attempt counted. Typed `Conflict` when the entry is terminal,
    /// `Conflict` when the budget is exhausted (entry dead-letters), and
    /// `NotFound` for unknown ids.
    pub async fn begin_repair_attempt(&self, repair_id: &str) -> IngestionResult<RepairEntry> {
        let sql = format!(
            "UPDATE knowledge_ingestion_repair_queue
             SET state = 'retrying', attempts = attempts + 1,
                 last_attempt_at = NOW(), updated_at = NOW()
             WHERE repair_id = $1 AND state IN ('queued', 'retrying')
               AND attempts < max_attempts
             RETURNING {REPAIR_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(repair_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?;
        match row {
            Some(row) => repair_from_pg(&row),
            None => match self.get_repair_entry(repair_id).await? {
                Some(entry) if entry.state.is_terminal() => Err(IngestionError::Storage(
                    crate::storage::StorageError::Conflict(
                        "repair entry is terminal; retries are over",
                    ),
                )),
                Some(_) => {
                    // Budget exhausted: dead-letter the entry now.
                    let _ = self.dead_letter_repair(repair_id).await?;
                    Err(IngestionError::Storage(
                        crate::storage::StorageError::Conflict(
                            "repair attempts exhausted; entry dead-lettered",
                        ),
                    ))
                }
                None => Err(IngestionError::Storage(
                    crate::storage::StorageError::NotFound("repair entry"),
                )),
            },
        }
    }

    /// Settle a retry attempt: resolve, requeue, or dead-letter on budget.
    pub async fn settle_repair_attempt(
        &self,
        repair_id: &str,
        outcome: RepairAttemptOutcome,
    ) -> IngestionResult<RepairEntry> {
        match outcome {
            RepairAttemptOutcome::Resolved {
                resolved_receipt_id,
            } => {
                let sql = format!(
                    "UPDATE knowledge_ingestion_repair_queue
                     SET state = 'resolved', resolved_receipt_id = $2, updated_at = NOW()
                     WHERE repair_id = $1 AND state = 'retrying'
                     RETURNING {REPAIR_COLUMNS}"
                );
                let row = sqlx::query(&sql)
                    .bind(repair_id)
                    .bind(&resolved_receipt_id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(crate::storage::StorageError::from)?
                    .ok_or(IngestionError::Storage(
                        crate::storage::StorageError::Conflict(
                            "repair entry is not in a retrying state",
                        ),
                    ))?;
                repair_from_pg(&row)
            }
            RepairAttemptOutcome::FailedAgain {
                receipt_id,
                reason_detail,
            } => {
                // Requeue while budget remains, dead-letter otherwise.
                let sql = format!(
                    "UPDATE knowledge_ingestion_repair_queue
                     SET state = CASE WHEN attempts >= max_attempts
                                      THEN 'dead_letter' ELSE 'queued' END,
                         reason_detail = $2,
                         receipt_id = COALESCE($3, receipt_id),
                         updated_at = NOW()
                     WHERE repair_id = $1 AND state = 'retrying'
                     RETURNING {REPAIR_COLUMNS}"
                );
                let row = sqlx::query(&sql)
                    .bind(repair_id)
                    .bind(&reason_detail)
                    .bind(&receipt_id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(crate::storage::StorageError::from)?
                    .ok_or(IngestionError::Storage(
                        crate::storage::StorageError::Conflict(
                            "repair entry is not in a retrying state",
                        ),
                    ))?;
                repair_from_pg(&row)
            }
        }
    }

    /// Operator decision: dead-letter an open entry directly.
    pub async fn dead_letter_repair(&self, repair_id: &str) -> IngestionResult<RepairEntry> {
        let sql = format!(
            "UPDATE knowledge_ingestion_repair_queue
             SET state = 'dead_letter', updated_at = NOW()
             WHERE repair_id = $1 AND state IN ('queued', 'retrying')
             RETURNING {REPAIR_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(repair_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(crate::storage::StorageError::from)?
            .ok_or(IngestionError::Storage(
                crate::storage::StorageError::Conflict(
                    "repair entry is not open; cannot dead-letter",
                ),
            ))?;
        repair_from_pg(&row)
    }
}

// ---------------------------------------------------------------------------
// Row mappers for receipts, spans, repair entries.
// ---------------------------------------------------------------------------

/// Durable row of `knowledge_ingestion_spans`.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredSpan {
    pub span_id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub receipt_id: String,
    pub span_index: i32,
    pub anchor: SpanAnchor,
    pub byte_start: Option<i64>,
    pub byte_end: Option<i64>,
    pub content: String,
    pub content_hash: String,
    pub redaction_state: super::spans::SpanRedaction,
    pub link_candidates: Value,
    pub created_at: DateTime<Utc>,
}

const RECEIPT_COLUMNS: &str = "receipt_id, workspace_id, source_id, ingestion_run_token, \
     extractor_id, extractor_version, status, error_class, error_detail, spans_produced, \
     spans_failed, redaction_count, content_hash, duration_ms, receipt_event_id, created_at";

const SPAN_COLUMNS: &str = "span_id, workspace_id, source_id, receipt_id, span_index, \
     anchor_kind, anchor, byte_start, byte_end, content, content_hash, redaction_state, \
     link_candidates, created_at";

const REPAIR_COLUMNS: &str = "repair_id, workspace_id, source_id, receipt_id, reason_class, \
     reason_detail, state, attempts, max_attempts, last_attempt_at, resolved_receipt_id, \
     enqueue_event_id, created_at, updated_at";

fn receipt_from_pg(row: &sqlx::postgres::PgRow) -> IngestionResult<ExtractionReceipt> {
    Ok(ExtractionReceipt {
        receipt_id: row.get("receipt_id"),
        workspace_id: row.get("workspace_id"),
        source_id: row.get("source_id"),
        ingestion_run_token: row.get("ingestion_run_token"),
        extractor_id: row.get("extractor_id"),
        extractor_version: row.get("extractor_version"),
        status: row.get::<String, _>("status").parse()?,
        error_class: row
            .get::<Option<String>, _>("error_class")
            .map(|c| c.parse())
            .transpose()?,
        error_detail: row.get("error_detail"),
        spans_produced: row.get("spans_produced"),
        spans_failed: row.get("spans_failed"),
        redaction_count: row.get("redaction_count"),
        content_hash: row.get("content_hash"),
        duration_ms: row.get("duration_ms"),
        receipt_event_id: row.get("receipt_event_id"),
        created_at: row.get("created_at"),
    })
}

fn span_from_pg(row: &sqlx::postgres::PgRow) -> IngestionResult<StoredSpan> {
    let anchor_value: Value = row.get("anchor");
    Ok(StoredSpan {
        span_id: row.get("span_id"),
        workspace_id: row.get("workspace_id"),
        source_id: row.get("source_id"),
        receipt_id: row.get("receipt_id"),
        span_index: row.get("span_index"),
        anchor: SpanAnchor::from_json(&anchor_value)?,
        byte_start: row.get("byte_start"),
        byte_end: row.get("byte_end"),
        content: row.get("content"),
        content_hash: row.get("content_hash"),
        redaction_state: row.get::<String, _>("redaction_state").parse()?,
        link_candidates: row.get("link_candidates"),
        created_at: row.get("created_at"),
    })
}

fn repair_from_pg(row: &sqlx::postgres::PgRow) -> IngestionResult<RepairEntry> {
    Ok(RepairEntry {
        repair_id: row.get("repair_id"),
        workspace_id: row.get("workspace_id"),
        source_id: row.get("source_id"),
        receipt_id: row.get("receipt_id"),
        reason_class: row.get::<String, _>("reason_class").parse()?,
        reason_detail: row.get("reason_detail"),
        state: row.get::<String, _>("state").parse()?,
        attempts: row.get("attempts"),
        max_attempts: row.get("max_attempts"),
        last_attempt_at: row.get("last_attempt_at"),
        resolved_receipt_id: row.get("resolved_receipt_id"),
        enqueue_event_id: row.get("enqueue_event_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

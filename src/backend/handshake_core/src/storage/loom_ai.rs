//! WP-KERNEL-009 MT-260 UnifiedWorkSurface-260-AILoomJobs (GAP-LM-011) storage.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — AI
//! auto-tagging/auto-captioning/auto-linking MUST leave actor, denial, or
//! promotion receipts. This module is the PostgreSQL surface for the
//! `loom_ai_suggestions` table (migration 0333): every model suggestion is a
//! PENDING proposal row that becomes authority only after operator/validator
//! confirm-to-promote.
//!
//! Pattern follows `storage/knowledge_crdt.rs`: free async functions over
//! `&sqlx::PgPool`. There is NO in-memory/SQLite fallback; without PostgreSQL
//! every function fails closed with a typed `StorageError`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{StorageError, StorageResult};

/// The three AI Loom job kinds.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoomAiJobKind {
    AutoTag,
    AutoCaption,
    LinkSuggest,
}

impl LoomAiJobKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AutoTag => "auto_tag",
            Self::AutoCaption => "auto_caption",
            Self::LinkSuggest => "link_suggest",
        }
    }

    pub fn parse(value: &str) -> StorageResult<Self> {
        match value {
            "auto_tag" => Ok(Self::AutoTag),
            "auto_caption" => Ok(Self::AutoCaption),
            "link_suggest" => Ok(Self::LinkSuggest),
            _ => Err(StorageError::Validation("invalid loom ai job kind")),
        }
    }
}

/// A persisted AI Loom suggestion row.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LoomAiSuggestionRow {
    pub suggestion_id: String,
    pub job_id: String,
    pub workspace_id: String,
    pub kind: String,
    pub block_id: String,
    pub target_block_id: Option<String>,
    pub suggested_value: Value,
    pub model_attribution: Value,
    pub prompt_sha256: String,
    pub output_sha256: String,
    pub review_state: String,
    pub decided_by: Option<String>,
    pub decided_at_utc: Option<DateTime<Utc>>,
    pub decision_reason: Option<String>,
    pub recorded_event_id: String,
    pub decided_event_id: Option<String>,
    pub promotion_requested_event_id: Option<String>,
    pub promotion_accepted_event_id: Option<String>,
    pub promoted_artifact_ref: Option<String>,
    pub value_hash: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Input for [`insert_loom_ai_suggestion`].
#[derive(Clone, Debug)]
pub struct NewLoomAiSuggestion {
    pub suggestion_id: String,
    pub job_id: String,
    pub workspace_id: String,
    pub kind: LoomAiJobKind,
    pub block_id: String,
    pub target_block_id: Option<String>,
    pub suggested_value: Value,
    pub model_attribution: Value,
    pub prompt_sha256: String,
    pub output_sha256: String,
    pub value_hash: String,
    pub recorded_event_id: String,
}

/// New suggestion id (`LAIS-<32 hex>`, time-ordered v7).
pub fn new_suggestion_id() -> String {
    format!("LAIS-{}", Uuid::now_v7().simple())
}

/// New job id (`LAIJ-<32 hex>`, time-ordered v7).
pub fn new_job_id() -> String {
    format!("LAIJ-{}", Uuid::now_v7().simple())
}

const SUGGESTION_COLUMNS: &str = "suggestion_id, job_id, workspace_id, kind, block_id, \
    target_block_id, suggested_value, model_attribution, prompt_sha256, output_sha256, \
    review_state, decided_by, decided_at_utc, decision_reason, recorded_event_id, \
    decided_event_id, promotion_requested_event_id, promotion_accepted_event_id, \
    promoted_artifact_ref, value_hash, created_at_utc";

fn row_to_suggestion(row: &sqlx::postgres::PgRow) -> LoomAiSuggestionRow {
    LoomAiSuggestionRow {
        suggestion_id: row.get("suggestion_id"),
        job_id: row.get("job_id"),
        workspace_id: row.get("workspace_id"),
        kind: row.get("kind"),
        block_id: row.get("block_id"),
        target_block_id: row.get("target_block_id"),
        suggested_value: row.get("suggested_value"),
        model_attribution: row.get("model_attribution"),
        prompt_sha256: row.get("prompt_sha256"),
        output_sha256: row.get("output_sha256"),
        review_state: row.get("review_state"),
        decided_by: row.get("decided_by"),
        decided_at_utc: row.get("decided_at_utc"),
        decision_reason: row.get("decision_reason"),
        recorded_event_id: row.get("recorded_event_id"),
        decided_event_id: row.get("decided_event_id"),
        promotion_requested_event_id: row.get("promotion_requested_event_id"),
        promotion_accepted_event_id: row.get("promotion_accepted_event_id"),
        promoted_artifact_ref: row.get("promoted_artifact_ref"),
        value_hash: row.get("value_hash"),
        created_at_utc: row.get("created_at_utc"),
    }
}

/// Insert a PENDING suggestion. Idempotent on
/// (job_id, block_id, kind, value_hash, target): a re-run that produces the
/// same suggestion returns the EXISTING row rather than a duplicate.
pub async fn insert_loom_ai_suggestion(
    pool: &PgPool,
    new: NewLoomAiSuggestion,
) -> StorageResult<LoomAiSuggestionRow> {
    let inserted = sqlx::query(&format!(
        r#"
        INSERT INTO loom_ai_suggestions (
            suggestion_id, job_id, workspace_id, kind, block_id, target_block_id,
            suggested_value, model_attribution, prompt_sha256, output_sha256,
            recorded_event_id, value_hash
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (job_id, block_id, kind, value_hash, COALESCE(target_block_id, ''))
            DO NOTHING
        RETURNING {SUGGESTION_COLUMNS}
        "#
    ))
    .bind(&new.suggestion_id)
    .bind(&new.job_id)
    .bind(&new.workspace_id)
    .bind(new.kind.as_str())
    .bind(&new.block_id)
    .bind(&new.target_block_id)
    .bind(&new.suggested_value)
    .bind(&new.model_attribution)
    .bind(&new.prompt_sha256)
    .bind(&new.output_sha256)
    .bind(&new.recorded_event_id)
    .bind(&new.value_hash)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = inserted {
        return Ok(row_to_suggestion(&row));
    }

    // Conflict: the suggestion already exists — return the persisted row.
    let existing = sqlx::query(&format!(
        r#"
        SELECT {SUGGESTION_COLUMNS} FROM loom_ai_suggestions
        WHERE job_id = $1 AND block_id = $2 AND kind = $3 AND value_hash = $4
            AND COALESCE(target_block_id, '') = COALESCE($5, '')
        "#
    ))
    .bind(&new.job_id)
    .bind(&new.block_id)
    .bind(new.kind.as_str())
    .bind(&new.value_hash)
    .bind(&new.target_block_id)
    .fetch_one(pool)
    .await?;
    Ok(row_to_suggestion(&existing))
}

/// Read one suggestion by id.
pub async fn get_loom_ai_suggestion(
    pool: &PgPool,
    suggestion_id: &str,
) -> StorageResult<Option<LoomAiSuggestionRow>> {
    let row = sqlx::query(&format!(
        "SELECT {SUGGESTION_COLUMNS} FROM loom_ai_suggestions WHERE suggestion_id = $1"
    ))
    .bind(suggestion_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(row_to_suggestion))
}

/// List suggestions for a job, optionally filtered by review_state, newest
/// first. When `job_id` is None, list all suggestions in the workspace.
pub async fn list_loom_ai_suggestions(
    pool: &PgPool,
    workspace_id: &str,
    job_id: Option<&str>,
    review_state: Option<&str>,
) -> StorageResult<Vec<LoomAiSuggestionRow>> {
    let rows = sqlx::query(&format!(
        r#"
        SELECT {SUGGESTION_COLUMNS} FROM loom_ai_suggestions
        WHERE workspace_id = $1
            AND ($2::text IS NULL OR job_id = $2)
            AND ($3::text IS NULL OR review_state = $3)
        ORDER BY kind, created_at_utc DESC, suggestion_id
        "#
    ))
    .bind(workspace_id)
    .bind(job_id)
    .bind(review_state)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_suggestion).collect())
}

/// Stamp the decision on a PENDING row (-> accepted | rejected). Returns the
/// updated row, or `None` if the row was not pending (lost a race / wrong
/// state). The caller has already validated reviewer authority and written the
/// AI_EDIT_PROPOSAL_DECIDED event.
pub async fn decide_loom_ai_suggestion(
    pool: &PgPool,
    suggestion_id: &str,
    new_state: &str,
    decided_by: &str,
    decision_reason: &str,
    decided_event_id: &str,
) -> StorageResult<Option<LoomAiSuggestionRow>> {
    let row = sqlx::query(&format!(
        r#"
        UPDATE loom_ai_suggestions
        SET review_state = $2, decided_by = $3, decided_at_utc = NOW(),
            decision_reason = $4, decided_event_id = $5
        WHERE suggestion_id = $1 AND review_state = 'pending'
        RETURNING {SUGGESTION_COLUMNS}
        "#
    ))
    .bind(suggestion_id)
    .bind(new_state)
    .bind(decided_by)
    .bind(decision_reason)
    .bind(decided_event_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(row_to_suggestion))
}

/// Mark an ACCEPTED row promoted (stamp the promotion pair + artifact ref).
/// Returns `None` if the row was not in 'accepted' state.
pub async fn mark_loom_ai_suggestion_promoted(
    pool: &PgPool,
    suggestion_id: &str,
    promotion_requested_event_id: &str,
    promotion_accepted_event_id: &str,
    promoted_artifact_ref: &str,
) -> StorageResult<Option<LoomAiSuggestionRow>> {
    let row = sqlx::query(&format!(
        r#"
        UPDATE loom_ai_suggestions
        SET review_state = 'promoted',
            promotion_requested_event_id = $2,
            promotion_accepted_event_id = $3,
            promoted_artifact_ref = $4
        WHERE suggestion_id = $1 AND review_state = 'accepted'
        RETURNING {SUGGESTION_COLUMNS}
        "#
    ))
    .bind(suggestion_id)
    .bind(promotion_requested_event_id)
    .bind(promotion_accepted_event_id)
    .bind(promoted_artifact_ref)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(row_to_suggestion))
}

/// Persist an `auto_caption` / `auto_tags` derived field on a LoomBlock,
/// stamping `generated_by` provenance. This is the caption/tag promotion target
/// (LoomBlockDerived.auto_caption / auto_tags). Merges into the existing
/// `derived_json` rather than overwriting other derived fields. Returns the
/// block_id when a row was updated.
pub async fn apply_loom_block_auto_derived(
    pool: &PgPool,
    workspace_id: &str,
    block_id: &str,
    auto_caption: Option<&str>,
    auto_tags: Option<&[String]>,
    generated_by: Value,
) -> StorageResult<Option<String>> {
    // jsonb_set/||-merge so unrelated derived keys (metrics, preview) survive.
    let mut patch = serde_json::Map::new();
    if let Some(caption) = auto_caption {
        patch.insert("auto_caption".to_string(), Value::String(caption.to_string()));
    }
    if let Some(tags) = auto_tags {
        patch.insert(
            "auto_tags".to_string(),
            Value::Array(tags.iter().cloned().map(Value::String).collect()),
        );
    }
    patch.insert("generated_by".to_string(), generated_by);
    let patch_value = Value::Object(patch);

    let row = sqlx::query(
        r#"
        UPDATE loom_blocks
        SET derived_json = (derived_json::jsonb || $3::jsonb)::text,
            updated_at = NOW()
        WHERE workspace_id = $1 AND block_id = $2
        RETURNING block_id
        "#,
    )
    .bind(workspace_id)
    .bind(block_id)
    .bind(&patch_value)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.get::<String, _>("block_id")))
}

//! WP-KERNEL-009 MT-260 UnifiedWorkSurface-260-AILoomJobs (GAP-LM-011) storage.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — AI
//! auto-tagging/auto-captioning/auto-linking MUST leave actor, denial, or
//! promotion receipts. This module is the storage surface for the
//! `loom_ai_suggestions` table: every model suggestion is a PENDING proposal
//! row that becomes authority only after operator/validator
//! confirm-to-promote.
//!
//! Pattern follows `storage/block_view_outbox_surreal.rs`: free async
//! functions over the embedded SurrealDB store's sealed data facade. There is
//! no in-memory or fixture fallback; without the durable store every function
//! fails closed with a typed `StorageError`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use super::surreal::SurrealStorage;
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

/// The stored row shape. `workspace_id` and the receipt refs are record links
/// in the embedded store; the public row type keeps them as plain ids.
#[derive(SurrealValue)]
struct SuggestionRecord {
    suggestion_id: String,
    job_id: String,
    workspace_id: RecordId,
    kind: String,
    block_id: String,
    target_block_id: Option<String>,
    suggested_value: Value,
    model_attribution: Value,
    prompt_sha256: String,
    output_sha256: String,
    review_state: String,
    decided_by: Option<String>,
    decided_at_utc: Option<Datetime>,
    decision_reason: Option<String>,
    recorded_event_id: RecordId,
    decided_event_id: Option<RecordId>,
    promotion_requested_event_id: Option<RecordId>,
    promotion_accepted_event_id: Option<RecordId>,
    promoted_artifact_ref: Option<String>,
    value_hash: String,
    created_at_utc: Datetime,
}

fn string_key(record_id: RecordId) -> StorageResult<String> {
    match record_id.key {
        RecordIdKey::String(id) => Ok(id),
        _ => Err(StorageError::Serialization(
            "loom ai suggestion record link is not a string key".to_owned(),
        )),
    }
}

fn opt_string_key(record_id: Option<RecordId>) -> StorageResult<Option<String>> {
    record_id.map(string_key).transpose()
}

fn record_to_suggestion(record: SuggestionRecord) -> StorageResult<LoomAiSuggestionRow> {
    Ok(LoomAiSuggestionRow {
        suggestion_id: record.suggestion_id,
        job_id: record.job_id,
        workspace_id: string_key(record.workspace_id)?,
        kind: record.kind,
        block_id: record.block_id,
        target_block_id: record.target_block_id,
        suggested_value: record.suggested_value,
        model_attribution: record.model_attribution,
        prompt_sha256: record.prompt_sha256,
        output_sha256: record.output_sha256,
        review_state: record.review_state,
        decided_by: record.decided_by,
        decided_at_utc: record.decided_at_utc.map(Datetime::into_inner),
        decision_reason: record.decision_reason,
        recorded_event_id: string_key(record.recorded_event_id)?,
        decided_event_id: opt_string_key(record.decided_event_id)?,
        promotion_requested_event_id: opt_string_key(record.promotion_requested_event_id)?,
        promotion_accepted_event_id: opt_string_key(record.promotion_accepted_event_id)?,
        promoted_artifact_ref: record.promoted_artifact_ref,
        value_hash: record.value_hash,
        created_at_utc: record.created_at_utc.into_inner(),
    })
}

fn map_err(error: super::surreal::SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

#[derive(SurrealValue)]
struct InsertBindings {
    suggestion_id: String,
    job_id: String,
    workspace: RecordId,
    kind: String,
    block_id: String,
    target_block_id: Option<String>,
    suggested_value: Value,
    model_attribution: Value,
    prompt_sha256: String,
    output_sha256: String,
    recorded_event: RecordId,
    value_hash: String,
}

/// Insert a PENDING suggestion. Idempotent on
/// (job_id, block_id, kind, value_hash, target): a re-run that produces the
/// same suggestion returns the EXISTING row rather than a duplicate. The
/// existence probe and the create run inside one statement, so two racing
/// producers cannot both observe the identity as free.
pub async fn insert_loom_ai_suggestion(
    storage: &SurrealStorage,
    new: NewLoomAiSuggestion,
) -> StorageResult<LoomAiSuggestionRow> {
    let bindings = InsertBindings {
        suggestion_id: new.suggestion_id.clone(),
        job_id: new.job_id.clone(),
        workspace: RecordId::new("workspaces", new.workspace_id.clone()),
        kind: new.kind.as_str().to_owned(),
        block_id: new.block_id.clone(),
        target_block_id: new.target_block_id.clone(),
        suggested_value: new.suggested_value.clone(),
        model_attribution: new.model_attribution.clone(),
        prompt_sha256: new.prompt_sha256.clone(),
        output_sha256: new.output_sha256.clone(),
        recorded_event: RecordId::new("kernel_event_ledger", new.recorded_event_id.clone()),
        value_hash: new.value_hash.clone(),
    };
    let rows: Vec<SuggestionRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "IF (SELECT VALUE id FROM loom_ai_suggestions WHERE job_id = $job_id \
                         AND block_id = $block_id AND kind = $kind AND value_hash = $value_hash \
                         AND target_block_id = $target_block_id LIMIT 1)[0] = NONE \
                         { RETURN CREATE type::record('loom_ai_suggestions', $suggestion_id) \
                           CONTENT { suggestion_id: $suggestion_id, job_id: $job_id, \
                           workspace_id: $workspace, kind: $kind, block_id: $block_id, \
                           target_block_id: $target_block_id, suggested_value: $suggested_value, \
                           model_attribution: $model_attribution, prompt_sha256: $prompt_sha256, \
                           output_sha256: $output_sha256, recorded_event_id: $recorded_event, \
                           value_hash: $value_hash } RETURN AFTER; } \
                         ELSE { RETURN SELECT * FROM loom_ai_suggestions WHERE job_id = $job_id \
                           AND block_id = $block_id AND kind = $kind AND value_hash = $value_hash \
                           AND target_block_id = $target_block_id LIMIT 1; };",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "loom ai suggestion insert returned no record".to_owned(),
        ))
        .and_then(record_to_suggestion)
}

/// Read one suggestion by id.
pub async fn get_loom_ai_suggestion(
    storage: &SurrealStorage,
    suggestion_id: &str,
) -> StorageResult<Option<LoomAiSuggestionRow>> {
    let suggestion_id = suggestion_id.to_owned();
    let row: Option<SuggestionRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .select_one("loom_ai_suggestions", &suggestion_id)
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    row.map(record_to_suggestion).transpose()
}

#[derive(SurrealValue)]
struct ListBindings {
    workspace: RecordId,
    job_id: Option<String>,
    review_state: Option<String>,
}

/// List suggestions for a job, optionally filtered by review_state, newest
/// first. When `job_id` is None, list all suggestions in the workspace.
pub async fn list_loom_ai_suggestions(
    storage: &SurrealStorage,
    workspace_id: &str,
    job_id: Option<&str>,
    review_state: Option<&str>,
) -> StorageResult<Vec<LoomAiSuggestionRow>> {
    let bindings = ListBindings {
        workspace: RecordId::new("workspaces", workspace_id.to_owned()),
        job_id: job_id.map(str::to_owned),
        review_state: review_state.map(str::to_owned),
    };
    let rows: Vec<SuggestionRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM loom_ai_suggestions WHERE workspace_id = $workspace \
                         AND ($job_id = NONE OR job_id = $job_id) \
                         AND ($review_state = NONE OR review_state = $review_state) \
                         ORDER BY kind ASC, created_at_utc DESC, suggestion_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter().map(record_to_suggestion).collect()
}

#[derive(SurrealValue)]
struct DecideBindings {
    suggestion_id: String,
    new_state: String,
    decided_by: String,
    decision_reason: String,
    decided_event: RecordId,
}

/// Stamp the decision on a PENDING row (-> accepted | rejected). Returns the
/// updated row, or `None` if the row was not pending (lost a race / wrong
/// state). The caller has already validated reviewer authority and written the
/// AI_EDIT_PROPOSAL_DECIDED event.
pub async fn decide_loom_ai_suggestion(
    storage: &SurrealStorage,
    suggestion_id: &str,
    new_state: &str,
    decided_by: &str,
    decision_reason: &str,
    decided_event_id: &str,
) -> StorageResult<Option<LoomAiSuggestionRow>> {
    let bindings = DecideBindings {
        suggestion_id: suggestion_id.to_owned(),
        new_state: new_state.to_owned(),
        decided_by: decided_by.to_owned(),
        decision_reason: decision_reason.to_owned(),
        decided_event: RecordId::new("kernel_event_ledger", decided_event_id.to_owned()),
    };
    let row: Option<SuggestionRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "UPDATE loom_ai_suggestions SET review_state = $new_state, \
                         decided_by = $decided_by, decided_at_utc = time::now(), \
                         decision_reason = $decision_reason, decided_event_id = $decided_event \
                         WHERE suggestion_id = $suggestion_id AND review_state = 'pending' \
                         RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    row.map(record_to_suggestion).transpose()
}

#[derive(SurrealValue)]
struct PromoteBindings {
    suggestion_id: String,
    promotion_requested_event: RecordId,
    promotion_accepted_event: RecordId,
    promoted_artifact_ref: String,
}

/// Mark an ACCEPTED row promoted (stamp the promotion pair + artifact ref).
/// Returns `None` if the row was not in 'accepted' state.
pub async fn mark_loom_ai_suggestion_promoted(
    storage: &SurrealStorage,
    suggestion_id: &str,
    promotion_requested_event_id: &str,
    promotion_accepted_event_id: &str,
    promoted_artifact_ref: &str,
) -> StorageResult<Option<LoomAiSuggestionRow>> {
    let bindings = PromoteBindings {
        suggestion_id: suggestion_id.to_owned(),
        promotion_requested_event: RecordId::new(
            "kernel_event_ledger",
            promotion_requested_event_id.to_owned(),
        ),
        promotion_accepted_event: RecordId::new(
            "kernel_event_ledger",
            promotion_accepted_event_id.to_owned(),
        ),
        promoted_artifact_ref: promoted_artifact_ref.to_owned(),
    };
    let row: Option<SuggestionRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "UPDATE loom_ai_suggestions SET review_state = 'promoted', \
                         promotion_requested_event_id = $promotion_requested_event, \
                         promotion_accepted_event_id = $promotion_accepted_event, \
                         promoted_artifact_ref = $promoted_artifact_ref \
                         WHERE suggestion_id = $suggestion_id AND review_state = 'accepted' \
                         RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    row.map(record_to_suggestion).transpose()
}

#[derive(SurrealValue)]
struct DerivedBindings {
    workspace: RecordId,
    block_id: String,
    auto_caption: Option<String>,
    auto_tags: Option<Vec<String>>,
    generated_by: Value,
}

/// Persist an `auto_caption` / `auto_tags` derived field on a LoomBlock,
/// stamping `generated_by` provenance. This is the caption/tag promotion
/// target (LoomBlockDerived.auto_caption / auto_tags). Patches only the AI
/// keys inside `derived_json` so unrelated derived fields (metrics, preview)
/// survive. Returns the block_id when a row was updated.
pub async fn apply_loom_block_auto_derived(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
    auto_caption: Option<&str>,
    auto_tags: Option<&[String]>,
    generated_by: Value,
) -> StorageResult<Option<String>> {
    let bindings = DerivedBindings {
        workspace: RecordId::new("workspaces", workspace_id.to_owned()),
        block_id: block_id.to_owned(),
        auto_caption: auto_caption.map(str::to_owned),
        auto_tags: auto_tags.map(<[String]>::to_vec),
        generated_by,
    };
    let updated: Vec<String> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "UPDATE loom_blocks SET \
                         derived_json.auto_caption = IF $auto_caption != NONE { $auto_caption } ELSE { derived_json.auto_caption }, \
                         derived_json.auto_tags = IF $auto_tags != NONE { $auto_tags } ELSE { derived_json.auto_tags }, \
                         derived_json.generated_by = $generated_by, \
                         updated_at = time::now() \
                         WHERE workspace_id = $workspace AND block_id = $block_id \
                         RETURN VALUE block_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    Ok(updated.into_iter().next())
}

//! WP-CKC-posekit-overhaul MT-020: deterministic prompt-feedback kernel.
//!
//! Atelier/CKC prompt-feedback primitive (handoff:
//! `HANDSHAKE_PROMPT_FEEDBACK_KERNEL_HANDOFF_2026-06-28.md`). It imports CUI/
//! ComfyUI prompt receipts as [`PromptCase`] rows, records reviewer verdicts,
//! deterministically rewrites future prompt rows through versioned rule packs
//! (the pure [`engine`]), and exports corrected machine-readable JSONL as a
//! hashed ArtifactStore artifact.
//!
//! Storage authority is PostgreSQL/EventLedger + ArtifactStore only (mirrors the
//! rest of the atelier domain). The JSONL export is a materialized artifact with
//! a content hash and provenance -- never a sidecar system of record. Models are
//! advisors here: every rewrite carries a deterministic rule trace, and a
//! prompt-stress verdict can NEVER become an identity-success verdict.
//!
//! SCOPE / DEFERRED (MT-020 first slice): this module is the deterministic
//! prompt-feedback kernel only. Deferred to the MT-044 follow-up tracker: the
//! Leeseo LoRA-training infra is a separate future scope (`leeseo_wishlist.md`:
//! GPU preflight/isolation, kohya experiment scaffolding, pipeline orchestrator,
//! determinism/provenance snapshots, experiment registry, deterministic dataset
//! builder, member-separation assist, faster-training knobs, epoch sweep/keeper
//! selection, training monitor, eval-harness upgrades); the native panel live
//! backend round-trip and CSV import are named MT-044 items. None are built here.

pub mod adapter;
pub mod engine;
pub mod model;

use std::path::Path;

use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use sqlx::Row;
use thiserror::Error;
use uuid::Uuid;

use crate::storage::artifacts::{
    artifact_root_rel, write_file_artifact, ArtifactClassification, ArtifactLayer, ArtifactManifest,
    ArtifactPayloadKind,
};

use super::{reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore};

use self::adapter::{export_jsonl, ExportRow};
use self::engine::{evaluate, Feedback, RewriteOutcome};
use self::model::{
    NewPromptCase, NewReviewVerdict, PromptCase, PromptCaseAxes, PromptExport, ReviewVerdict,
    ReviewerKind, RewritePlan, RewriteRuleSpec, RulePack, VerdictKind,
};

/// EventLedger families emitted by the prompt-feedback kernel (folded into
/// [`super::event_family::ALL`]).
pub mod prompt_feedback_event_family {
    /// A prompt case was imported from a source system (adapter output).
    pub const CASE_IMPORTED: &str = "atelier.prompt_feedback.case_imported";
    /// A reviewer verdict was recorded against a prompt case.
    pub const VERDICT_RECORDED: &str = "atelier.prompt_feedback.verdict_recorded";
    /// A deterministic rewrite plan + trace was persisted for a case.
    pub const REWRITE_PLANNED: &str = "atelier.prompt_feedback.rewrite_planned";
    /// A versioned rule pack was registered.
    pub const RULEPACK_REGISTERED: &str = "atelier.prompt_feedback.rulepack_registered";
    /// A JSONL export was materialized as a hashed ArtifactStore artifact.
    pub const EXPORT_MATERIALIZED: &str = "atelier.prompt_feedback.export_materialized";

    pub const ALL: &[&str] = &[
        CASE_IMPORTED,
        VERDICT_RECORDED,
        REWRITE_PLANNED,
        RULEPACK_REGISTERED,
        EXPORT_MATERIALIZED,
    ];
}

/// Errors from the pure prompt-feedback domain (adapter/engine/model). Converts
/// into [`AtelierError`] so the store methods return a single error type.
#[derive(Debug, Error)]
pub enum PromptFeedbackError {
    #[error("prompt feedback validation error: {0}")]
    Validation(String),
}

impl From<PromptFeedbackError> for AtelierError {
    fn from(err: PromptFeedbackError) -> Self {
        match err {
            PromptFeedbackError::Validation(detail) => AtelierError::Validation(detail),
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// True for a transient PostgreSQL conflict that is safe to retry: a deadlock
/// (40P01) or a serialization failure (40001). Under concurrent multi-agent /
/// cross-WP writes to the shared `kernel_event_ledger`, the DB aborts one side of
/// a deadlock cycle; the aborted transaction rolled back and can be retried. The
/// error may arrive as `Database(sqlx)` (case/rewrite inserts) or as the
/// stringified `EventLedger(..)` from `record_event_in_tx`, so match on Display.
fn is_retryable_conflict(err: &AtelierError) -> bool {
    let msg = err.to_string().to_ascii_lowercase();
    msg.contains("deadlock")
        || msg.contains("could not serialize")
        || msg.contains("40p01")
        || msg.contains("40001")
}

/// Run a transactional op with bounded retry + small backoff on a retryable
/// PostgreSQL conflict (deadlock / serialization failure). The op must be
/// self-contained and idempotent on retry (all our write methods are: upserts
/// keyed on stable columns, or fresh inserts whose only committed row is the one
/// that ultimately succeeds after the deadlocked attempt rolled back).
async fn with_retry<T, F, Fut>(op: F) -> AtelierResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = AtelierResult<T>>,
{
    let mut attempt: u32 = 0;
    loop {
        match op().await {
            Err(err) if attempt < 5 && is_retryable_conflict(&err) => {
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_millis(20 * u64::from(attempt)))
                    .await;
            }
            result => return result,
        }
    }
}

/// Deterministic hash of the review-feedback state that drives a rewrite. Vecs
/// are sorted first so ordering never changes the hash (F3).
fn hash_feedback(feedback: &Feedback) -> String {
    let mut failure_classes = feedback.failure_classes.clone();
    failure_classes.sort();
    let mut failure_tags = feedback.failure_tags.clone();
    failure_tags.sort();
    let canon = serde_json::json!({
        "failure_classes": failure_classes,
        "failure_tags": failure_tags,
        "contact_proof_recurring": feedback.contact_proof_recurring,
    });
    format!(
        "sha256:{}",
        sha256_hex(serde_json::to_string(&canon).unwrap_or_default().as_bytes())
    )
}

fn json_column<T: DeserializeOwned>(row: &sqlx::postgres::PgRow, column: &str) -> AtelierResult<T> {
    let value: serde_json::Value = row.get(column);
    serde_json::from_value(value).map_err(|err| {
        AtelierError::Validation(format!("failed to decode json column {column}: {err}"))
    })
}

fn to_json<T: serde::Serialize>(value: &T) -> AtelierResult<serde_json::Value> {
    serde_json::to_value(value)
        .map_err(|err| AtelierError::Validation(format!("failed to encode json: {err}")))
}

/// Descriptor rows for the built-in seed rule pack (a registry view of the 5
/// engine seed rules).
pub fn seed_rule_pack_specs() -> Vec<RewriteRuleSpec> {
    vec![
        RewriteRuleSpec {
            rule_id: engine::RULE_PROTECTED_EVAL.to_string(),
            reason_code: "protected_eval_prompt_mutation".to_string(),
            action_kind: "hard_reject".to_string(),
            summary: "Strip a prompt-stress positive tail from a protected standard eval row so a prompt-stress verdict never becomes identity success.".to_string(),
        },
        RewriteRuleSpec {
            rule_id: engine::RULE_LOOSE_CLOTHING.to_string(),
            reason_code: "target_blocked_by_outfit".to_string(),
            action_kind: "prompt_rewrite".to_string(),
            summary: "Rewrite a loose outfit that hides a huge-tits body target into a functional access state.".to_string(),
        },
        RewriteRuleSpec {
            rule_id: engine::RULE_WET_SCENE.to_string(),
            reason_code: "incoherent_wet_scene".to_string(),
            action_kind: "prompt_rewrite".to_string(),
            summary: "Give a wet setting visual logic (transparent/clinging/removed clothing) instead of dry full coverage.".to_string(),
        },
        RewriteRuleSpec {
            rule_id: engine::RULE_CONTACT_PROOF.to_string(),
            reason_code: "action_claim_without_contact_proof".to_string(),
            action_kind: "prompt_rewrite".to_string(),
            summary: "Add concrete contact mechanics when a contact level is claimed without proof; route to control/inpaint when recurring.".to_string(),
        },
        RewriteRuleSpec {
            rule_id: engine::RULE_ARTIFACT_REPAIR.to_string(),
            reason_code: "artifact_requires_workflow_repair".to_string(),
            action_kind: "workflow_routing_hint".to_string(),
            summary: "Route a technical artifact (bad hands/smear/broken anatomy) to workflow repair, never a permanent content ban.".to_string(),
        },
    ]
}

const PROMPT_FEEDBACK_CASE_COLUMNS: &str = "case_id, project_id, source_system, adapter_id, \
     source_iteration_id, source_case_id, source_recipe_id, segment, cell, framing, \
     clothing_state, render_stack, identity_judgement_allowed, prompt_quality_review_allowed, \
     positive_prompt, negative_prompt, micro_gate, expected_failure, image_artifact_ref, \
     sheet_artifact_ref, axes, hardcore_fields, imported_by, created_at_utc";

fn prompt_case_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<PromptCase> {
    Ok(PromptCase {
        case_id: row.get("case_id"),
        project_id: row.get("project_id"),
        source_system: row.get("source_system"),
        adapter_id: row.get("adapter_id"),
        source_iteration_id: row.get("source_iteration_id"),
        source_case_id: row.get("source_case_id"),
        source_recipe_id: row.get("source_recipe_id"),
        segment: row.get("segment"),
        cell: row.get("cell"),
        framing: row.get("framing"),
        clothing_state: row.get("clothing_state"),
        render_stack: row.get("render_stack"),
        identity_judgement_allowed: row.get("identity_judgement_allowed"),
        prompt_quality_review_allowed: row.get("prompt_quality_review_allowed"),
        positive_prompt: row.get("positive_prompt"),
        negative_prompt: row.get("negative_prompt"),
        micro_gate: row.get("micro_gate"),
        expected_failure: row.get("expected_failure"),
        image_artifact_ref: row.get("image_artifact_ref"),
        sheet_artifact_ref: row.get("sheet_artifact_ref"),
        axes: json_column::<PromptCaseAxes>(row, "axes")?,
        hardcore_fields: row.get("hardcore_fields"),
        imported_by: row.get("imported_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

const PROMPT_FEEDBACK_VERDICT_COLUMNS: &str = "verdict_id, case_id, reviewer_kind, reviewer_id, \
     verdict_kind, failure_class, failure_tags, is_identity_judgement, note, created_at_utc";

fn verdict_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ReviewVerdict> {
    let reviewer_kind_token: String = row.get("reviewer_kind");
    let verdict_kind_token: String = row.get("verdict_kind");
    Ok(ReviewVerdict {
        verdict_id: row.get("verdict_id"),
        case_id: row.get("case_id"),
        reviewer_kind: ReviewerKind::from_token(&reviewer_kind_token)?,
        reviewer_id: row.get("reviewer_id"),
        verdict_kind: VerdictKind::from_token(&verdict_kind_token)?,
        failure_class: row.get("failure_class"),
        failure_tags: json_column::<Vec<String>>(row, "failure_tags")?,
        is_identity_judgement: row.get("is_identity_judgement"),
        note: row.get("note"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn rule_pack_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<RulePack> {
    Ok(RulePack {
        rule_pack_id: row.get("rule_pack_id"),
        version: row.get("version"),
        title: row.get("title"),
        description: row.get("description"),
        rules: json_column::<Vec<RewriteRuleSpec>>(row, "rules")?,
        content_hash: row.get("content_hash"),
        registered_by: row.get("registered_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

const PROMPT_FEEDBACK_REWRITE_COLUMNS: &str = "rewrite_id, case_id, source_case_id, rule_pack_id, \
     rule_pack_version, input_hash, output_hash, changed_fields, rewritten_positive_prompt, \
     rewritten_negative_prompt, outcome, planned_by, created_at_utc";

fn rewrite_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<RewritePlan> {
    Ok(RewritePlan {
        rewrite_id: row.get("rewrite_id"),
        case_id: row.get("case_id"),
        source_case_id: row.get("source_case_id"),
        rule_pack_id: row.get("rule_pack_id"),
        rule_pack_version: row.get("rule_pack_version"),
        input_hash: row.get("input_hash"),
        output_hash: row.get("output_hash"),
        changed_fields: json_column::<Vec<String>>(row, "changed_fields")?,
        rewritten_positive_prompt: row.get("rewritten_positive_prompt"),
        rewritten_negative_prompt: row.get("rewritten_negative_prompt"),
        outcome: json_column::<RewriteOutcome>(row, "outcome")?,
        planned_by: row.get("planned_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

const PROMPT_FEEDBACK_EXPORT_COLUMNS: &str = "export_id, rule_pack_id, rule_pack_version, \
     artifact_ref, manifest_ref, content_hash, byte_len, row_count, source_case_ids, rewrite_ids, \
     exported_by, created_at_utc";

fn export_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<PromptExport> {
    Ok(PromptExport {
        export_id: row.get("export_id"),
        rule_pack_id: row.get("rule_pack_id"),
        rule_pack_version: row.get("rule_pack_version"),
        artifact_ref: row.get("artifact_ref"),
        manifest_ref: row.get("manifest_ref"),
        content_hash: row.get("content_hash"),
        byte_len: row.get("byte_len"),
        row_count: row.get("row_count"),
        source_case_ids: json_column::<Vec<String>>(row, "source_case_ids")?,
        rewrite_ids: json_column::<Vec<Uuid>>(row, "rewrite_ids")?,
        exported_by: row.get("exported_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn validate_new_prompt_case(new: &NewPromptCase) -> AtelierResult<()> {
    for (field, value) in [
        ("project_id", &new.project_id),
        ("source_system", &new.source_system),
        ("adapter_id", &new.adapter_id),
        ("source_case_id", &new.source_case_id),
        ("segment", &new.segment),
        ("cell", &new.cell),
        ("framing", &new.framing),
        ("clothing_state", &new.clothing_state),
        ("render_stack", &new.render_stack),
        ("imported_by", &new.imported_by),
    ] {
        if value.trim().is_empty() || value.trim() != value {
            return Err(AtelierError::Validation(format!(
                "prompt case {field} must not be empty or padded"
            )));
        }
    }
    if let Some(image_ref) = new.image_artifact_ref.as_deref() {
        reject_legacy_runtime_ref("image_artifact_ref", image_ref)?;
    }
    if let Some(sheet_ref) = new.sheet_artifact_ref.as_deref() {
        reject_legacy_runtime_ref("sheet_artifact_ref", sheet_ref)?;
    }
    if !new.hardcore_fields.is_object() {
        return Err(AtelierError::Validation(
            "prompt case hardcore_fields must be a JSON object".to_string(),
        ));
    }
    Ok(())
}

/// Filter for [`AtelierStore::list_prompt_cases`].
#[derive(Clone, Debug, Default)]
pub struct PromptCaseFilter {
    pub project_id: Option<String>,
    pub segment: Option<String>,
    pub cell: Option<String>,
    pub render_stack: Option<String>,
    pub limit: Option<i64>,
}

impl AtelierStore {
    /// Import a batch of prompt cases (adapter output). Idempotent on
    /// `(adapter_id, source_case_id)`: re-importing the same source case updates
    /// it in place. One `CASE_IMPORTED` event is emitted per case.
    pub async fn import_prompt_cases(
        &self,
        cases: &[NewPromptCase],
    ) -> AtelierResult<Vec<PromptCase>> {
        for case in cases {
            validate_new_prompt_case(case)?;
        }
        with_retry(|| self.import_prompt_cases_txn(cases)).await
    }

    async fn import_prompt_cases_txn(
        &self,
        cases: &[NewPromptCase],
    ) -> AtelierResult<Vec<PromptCase>> {
        let mut tx = self.pool().begin().await?;
        let mut imported = Vec::with_capacity(cases.len());
        for new in cases {
            let axes = to_json(&new.axes)?;
            let row = sqlx::query(&format!(
                r#"INSERT INTO atelier_prompt_feedback_case
                     (project_id, source_system, adapter_id, source_iteration_id, source_case_id,
                      source_recipe_id, segment, cell, framing, clothing_state, render_stack,
                      identity_judgement_allowed, prompt_quality_review_allowed, positive_prompt,
                      negative_prompt, micro_gate, expected_failure, image_artifact_ref,
                      sheet_artifact_ref, axes, hardcore_fields, imported_by)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                           $17, $18, $19, $20, $21, $22)
                   ON CONFLICT (adapter_id, source_case_id) DO UPDATE SET
                     project_id = EXCLUDED.project_id,
                     source_system = EXCLUDED.source_system,
                     source_iteration_id = EXCLUDED.source_iteration_id,
                     source_recipe_id = EXCLUDED.source_recipe_id,
                     segment = EXCLUDED.segment,
                     cell = EXCLUDED.cell,
                     framing = EXCLUDED.framing,
                     clothing_state = EXCLUDED.clothing_state,
                     render_stack = EXCLUDED.render_stack,
                     identity_judgement_allowed = EXCLUDED.identity_judgement_allowed,
                     prompt_quality_review_allowed = EXCLUDED.prompt_quality_review_allowed,
                     positive_prompt = EXCLUDED.positive_prompt,
                     negative_prompt = EXCLUDED.negative_prompt,
                     micro_gate = EXCLUDED.micro_gate,
                     expected_failure = EXCLUDED.expected_failure,
                     image_artifact_ref = EXCLUDED.image_artifact_ref,
                     sheet_artifact_ref = EXCLUDED.sheet_artifact_ref,
                     axes = EXCLUDED.axes,
                     hardcore_fields = EXCLUDED.hardcore_fields,
                     imported_by = EXCLUDED.imported_by
                   RETURNING {PROMPT_FEEDBACK_CASE_COLUMNS}"#
            ))
            .bind(&new.project_id)
            .bind(&new.source_system)
            .bind(&new.adapter_id)
            .bind(&new.source_iteration_id)
            .bind(&new.source_case_id)
            .bind(&new.source_recipe_id)
            .bind(&new.segment)
            .bind(&new.cell)
            .bind(&new.framing)
            .bind(&new.clothing_state)
            .bind(&new.render_stack)
            .bind(new.identity_judgement_allowed)
            .bind(new.prompt_quality_review_allowed)
            .bind(&new.positive_prompt)
            .bind(&new.negative_prompt)
            .bind(&new.micro_gate)
            .bind(&new.expected_failure)
            .bind(&new.image_artifact_ref)
            .bind(&new.sheet_artifact_ref)
            .bind(&axes)
            .bind(&new.hardcore_fields)
            .bind(&new.imported_by)
            .fetch_one(&mut *tx)
            .await?;
            let case = prompt_case_from_row(&row)?;
            self.record_event_in_tx(
                &mut tx,
                prompt_feedback_event_family::CASE_IMPORTED,
                "atelier_prompt_feedback_case",
                &case.case_id.to_string(),
                serde_json::json!({
                    "case_id": case.case_id,
                    "adapter_id": case.adapter_id,
                    "source_case_id": case.source_case_id,
                    "segment": case.segment,
                    "cell": case.cell,
                    "render_stack": case.render_stack,
                    "identity_judgement_allowed": case.identity_judgement_allowed,
                    "prompt_quality_review_allowed": case.prompt_quality_review_allowed,
                    "schema": "hsk.atelier.prompt_feedback_case@1",
                }),
            )
            .await?;
            imported.push(case);
        }
        tx.commit().await?;
        Ok(imported)
    }

    /// List prompt cases, newest first, filtered by project/segment/cell/render
    /// stack when provided.
    pub async fn list_prompt_cases(
        &self,
        filter: &PromptCaseFilter,
    ) -> AtelierResult<Vec<PromptCase>> {
        let limit = filter.limit.unwrap_or(200).clamp(1, 500);
        let rows = sqlx::query(&format!(
            r#"SELECT {PROMPT_FEEDBACK_CASE_COLUMNS}
               FROM atelier_prompt_feedback_case
               WHERE ($1::text IS NULL OR project_id = $1)
                 AND ($2::text IS NULL OR segment = $2)
                 AND ($3::text IS NULL OR cell = $3)
                 AND ($4::text IS NULL OR render_stack = $4)
               ORDER BY created_at_utc DESC, case_id ASC
               LIMIT $5"#
        ))
        .bind(&filter.project_id)
        .bind(&filter.segment)
        .bind(&filter.cell)
        .bind(&filter.render_stack)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(prompt_case_from_row).collect()
    }

    pub async fn get_prompt_case(&self, case_id: Uuid) -> AtelierResult<PromptCase> {
        let row = sqlx::query(&format!(
            r#"SELECT {PROMPT_FEEDBACK_CASE_COLUMNS}
               FROM atelier_prompt_feedback_case WHERE case_id = $1"#
        ))
        .bind(case_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("prompt case case_id={case_id}")))?;
        prompt_case_from_row(&row)
    }

    /// Record a reviewer verdict. A prompt-stress case (or any case where
    /// `identity_judgement_allowed` is false) rejects an identity judgement so a
    /// prompt-stress verdict can never become identity-success evidence.
    pub async fn record_prompt_verdict(
        &self,
        new: &NewReviewVerdict,
    ) -> AtelierResult<ReviewVerdict> {
        if new.reviewer_id.trim().is_empty() || new.reviewer_id.trim() != new.reviewer_id {
            return Err(AtelierError::Validation(
                "reviewer_id must not be empty or padded".to_string(),
            ));
        }
        let case = self.get_prompt_case(new.case_id).await?;
        // Defense in depth: reject an identity judgement both when the case flag
        // forbids it AND unconditionally for the prompt-stress segment, so the
        // invariant holds even if the boolean were ever mis-set.
        if new.is_identity_judgement
            && (case.segment == engine::SEGMENT_PROMPT_STRESS || !case.identity_judgement_allowed)
        {
            return Err(AtelierError::Validation(format!(
                "identity judgement is not allowed for case {} (segment={}); prompt-stress \
                 verdicts are prompt-quality/porn-readiness evidence only, not identity verdicts",
                case.source_case_id, case.segment
            )));
        }
        with_retry(|| self.record_prompt_verdict_txn(new)).await
    }

    async fn record_prompt_verdict_txn(
        &self,
        new: &NewReviewVerdict,
    ) -> AtelierResult<ReviewVerdict> {
        let failure_tags = to_json(&new.failure_tags)?;
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_prompt_feedback_verdict
                 (case_id, reviewer_kind, reviewer_id, verdict_kind, failure_class,
                  failure_tags, is_identity_judgement, note)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING {PROMPT_FEEDBACK_VERDICT_COLUMNS}"#
        ))
        .bind(new.case_id)
        .bind(new.reviewer_kind.as_token())
        .bind(&new.reviewer_id)
        .bind(new.verdict_kind.as_token())
        .bind(&new.failure_class)
        .bind(&failure_tags)
        .bind(new.is_identity_judgement)
        .bind(&new.note)
        .fetch_one(&mut *tx)
        .await?;
        let verdict = verdict_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            prompt_feedback_event_family::VERDICT_RECORDED,
            "atelier_prompt_feedback_verdict",
            &verdict.verdict_id.to_string(),
            serde_json::json!({
                "verdict_id": verdict.verdict_id,
                "case_id": verdict.case_id,
                "reviewer_kind": verdict.reviewer_kind.as_token(),
                "verdict_kind": verdict.verdict_kind.as_token(),
                "failure_class": verdict.failure_class,
                "is_identity_judgement": verdict.is_identity_judgement,
                "schema": "hsk.atelier.prompt_feedback_verdict@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(verdict)
    }

    pub async fn list_prompt_verdicts(&self, case_id: Uuid) -> AtelierResult<Vec<ReviewVerdict>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {PROMPT_FEEDBACK_VERDICT_COLUMNS}
               FROM atelier_prompt_feedback_verdict
               WHERE case_id = $1
               ORDER BY created_at_utc DESC, verdict_id ASC"#
        ))
        .bind(case_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(verdict_from_row).collect()
    }

    /// Register (or update) a versioned rule pack, keyed on `(rule_pack_id,
    /// version)`. The content hash pins the rule descriptors for determinism.
    pub async fn register_rule_pack(
        &self,
        rule_pack_id: &str,
        version: i32,
        title: &str,
        description: Option<&str>,
        rules: &[RewriteRuleSpec],
        registered_by: &str,
    ) -> AtelierResult<RulePack> {
        if rule_pack_id.trim().is_empty() || version < 1 || registered_by.trim().is_empty() {
            return Err(AtelierError::Validation(
                "rule pack requires a non-empty id, version >= 1, and registered_by".to_string(),
            ));
        }
        let rules_json = to_json(&rules)?;
        let content_hash = format!(
            "sha256:{}",
            sha256_hex(serde_json::to_string(&rules_json).unwrap_or_default().as_bytes())
        );
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_prompt_feedback_rule_pack
                 (rule_pack_id, version, title, description, rules, content_hash, registered_by)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (rule_pack_id, version) DO UPDATE SET
                 title = EXCLUDED.title,
                 description = EXCLUDED.description,
                 rules = EXCLUDED.rules,
                 content_hash = EXCLUDED.content_hash,
                 registered_by = EXCLUDED.registered_by
               RETURNING rule_pack_id, version, title, description, rules, content_hash,
                         registered_by, created_at_utc"#
        ))
        .bind(rule_pack_id)
        .bind(version)
        .bind(title)
        .bind(description)
        .bind(&rules_json)
        .bind(&content_hash)
        .bind(registered_by)
        .fetch_one(&mut *tx)
        .await?;
        let pack = rule_pack_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            prompt_feedback_event_family::RULEPACK_REGISTERED,
            "atelier_prompt_feedback_rule_pack",
            &format!("{}@{}", pack.rule_pack_id, pack.version),
            serde_json::json!({
                "rule_pack_id": pack.rule_pack_id,
                "version": pack.version,
                "content_hash": pack.content_hash,
                "rule_count": pack.rules.len(),
                "schema": "hsk.atelier.prompt_feedback_rule_pack@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(pack)
    }

    /// Ensure the built-in seed rule pack (`prompt-feedback.seed` v1) exists.
    pub async fn ensure_seed_rule_pack(&self, registered_by: &str) -> AtelierResult<RulePack> {
        self.register_rule_pack(
            engine::SEED_RULE_PACK_ID,
            engine::SEED_RULE_PACK_VERSION,
            "Prompt feedback seed rules",
            Some(
                "First-slice deterministic rewrite rules from the Leeseo prompt-feedback handoff.",
            ),
            &seed_rule_pack_specs(),
            registered_by,
        )
        .await
    }

    pub async fn list_rule_packs(&self) -> AtelierResult<Vec<RulePack>> {
        let rows = sqlx::query(
            r#"SELECT rule_pack_id, version, title, description, rules, content_hash,
                      registered_by, created_at_utc
               FROM atelier_prompt_feedback_rule_pack
               ORDER BY rule_pack_id ASC, version DESC"#,
        )
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(rule_pack_from_row).collect()
    }

    async fn rule_pack_exists(&self, rule_pack_id: &str, version: i32) -> AtelierResult<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                 SELECT 1 FROM atelier_prompt_feedback_rule_pack
                 WHERE rule_pack_id = $1 AND version = $2
               )"#,
        )
        .bind(rule_pack_id)
        .bind(version)
        .fetch_one(self.pool())
        .await?;
        Ok(exists)
    }

    /// Build the engine feedback signal from a case's persisted verdicts.
    async fn feedback_for_case(&self, case_id: Uuid) -> AtelierResult<Feedback> {
        let verdicts = self.list_prompt_verdicts(case_id).await?;
        let mut failure_classes: Vec<String> = Vec::new();
        let mut failure_tags: Vec<String> = Vec::new();
        let mut contact_hits = 0usize;
        for verdict in &verdicts {
            if let Some(class) = verdict.failure_class.as_deref() {
                if !failure_classes.iter().any(|c| c == class) {
                    failure_classes.push(class.to_string());
                }
            }
            for tag in &verdict.failure_tags {
                if tag == "action_claim_without_contact_proof" {
                    contact_hits += 1;
                }
                if !failure_tags.iter().any(|t| t == tag) {
                    failure_tags.push(tag.clone());
                }
            }
        }
        Ok(Feedback {
            failure_classes,
            failure_tags,
            contact_proof_recurring: contact_hits >= 2,
        })
    }

    /// Run the deterministic engine for one case against a rule pack and persist
    /// the rewrite plan + trace.
    ///
    /// First slice: only the seed rule pack (`prompt-feedback.seed` v1) is
    /// implemented by the engine, so any other `rule_pack_id`/version is rejected
    /// (F2) -- the trace can never misattribute a rule pack it did not run.
    ///
    /// Idempotency key is `(case_id, rule_pack_id, version, input_hash)` where
    /// `input_hash` folds BOTH the normalized case hash AND a deterministic hash
    /// of the current review-feedback state (F3). Because feedback changes the
    /// output (e.g. the contact rule flips to a workflow-routing hint once a
    /// contact-proof failure recurs), a different feedback state is a different
    /// rewrite row rather than a silent overwrite. The JSONL export's
    /// `original_prompt_hash` uses the PURE case hash (`outcome.input_hash`), so
    /// the prompt-hash provenance stays feedback-independent.
    pub async fn plan_prompt_rewrite(
        &self,
        case_id: Uuid,
        rule_pack_id: &str,
        rule_pack_version: i32,
        planned_by: &str,
    ) -> AtelierResult<RewritePlan> {
        if rule_pack_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "rewrite requires a rule_pack_id".to_string(),
            ));
        }
        if planned_by.trim().is_empty() {
            return Err(AtelierError::Validation(
                "rewrite requires planned_by".to_string(),
            ));
        }
        // F2: the first-slice engine only implements the seed rule pack. Reject
        // any other pack/version so a persisted trace cannot claim a pack the
        // engine did not actually run.
        if rule_pack_id != engine::SEED_RULE_PACK_ID
            || rule_pack_version != engine::SEED_RULE_PACK_VERSION
        {
            return Err(AtelierError::Validation(format!(
                "rule pack {rule_pack_id}@{rule_pack_version} is not implemented by the \
                 first-slice engine; only {}@{} is supported",
                engine::SEED_RULE_PACK_ID,
                engine::SEED_RULE_PACK_VERSION
            )));
        }
        if !self.rule_pack_exists(rule_pack_id, rule_pack_version).await? {
            return Err(AtelierError::NotFound(format!(
                "rule pack {rule_pack_id}@{rule_pack_version}"
            )));
        }
        with_retry(|| self.plan_prompt_rewrite_txn(case_id, rule_pack_id, rule_pack_version, planned_by))
            .await
    }

    async fn plan_prompt_rewrite_txn(
        &self,
        case_id: Uuid,
        rule_pack_id: &str,
        rule_pack_version: i32,
        planned_by: &str,
    ) -> AtelierResult<RewritePlan> {
        let case = self.get_prompt_case(case_id).await?;
        let feedback = self.feedback_for_case(case_id).await?;
        let outcome = evaluate(
            &case.to_engine_case(),
            &feedback,
            rule_pack_id,
            rule_pack_version,
        );
        // F3: fold the feedback state into the persisted idempotency key so a
        // re-plan after new verdicts is a distinct row, not a silent overwrite.
        let stored_input_hash = format!("{}|fb:{}", outcome.input_hash, hash_feedback(&feedback));
        let changed_fields = to_json(&outcome.changed_fields)?;
        let outcome_json = to_json(&outcome)?;
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_prompt_feedback_rewrite
                 (case_id, source_case_id, rule_pack_id, rule_pack_version, input_hash,
                  output_hash, changed_fields, rewritten_positive_prompt,
                  rewritten_negative_prompt, outcome, planned_by)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
               ON CONFLICT (case_id, rule_pack_id, rule_pack_version, input_hash) DO UPDATE SET
                 output_hash = EXCLUDED.output_hash,
                 changed_fields = EXCLUDED.changed_fields,
                 rewritten_positive_prompt = EXCLUDED.rewritten_positive_prompt,
                 rewritten_negative_prompt = EXCLUDED.rewritten_negative_prompt,
                 outcome = EXCLUDED.outcome,
                 planned_by = EXCLUDED.planned_by
               RETURNING {PROMPT_FEEDBACK_REWRITE_COLUMNS}"#
        ))
        .bind(case_id)
        .bind(&case.source_case_id)
        .bind(rule_pack_id)
        .bind(rule_pack_version)
        .bind(&stored_input_hash)
        .bind(&outcome.output_hash)
        .bind(&changed_fields)
        .bind(&outcome.rewritten.positive_prompt)
        .bind(&outcome.rewritten.negative_prompt)
        .bind(&outcome_json)
        .bind(planned_by)
        .fetch_one(&mut *tx)
        .await?;
        let plan = rewrite_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            prompt_feedback_event_family::REWRITE_PLANNED,
            "atelier_prompt_feedback_rewrite",
            &plan.rewrite_id.to_string(),
            serde_json::json!({
                "rewrite_id": plan.rewrite_id,
                "case_id": plan.case_id,
                "rule_pack_id": plan.rule_pack_id,
                "rule_pack_version": plan.rule_pack_version,
                "input_hash": plan.input_hash,
                "output_hash": plan.output_hash,
                "changed_field_count": plan.changed_fields.len(),
                "schema": "hsk.atelier.prompt_feedback_rewrite@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(plan)
    }

    /// Materialize a JSONL export for a set of cases as a hashed ArtifactStore
    /// artifact. Each case is (re)planned deterministically against the rule
    /// pack, corrected rows are rendered as JSONL, the bytes are written to the
    /// ArtifactStore under `workspace_root`, and a [`PromptExport`] receipt is
    /// recorded. The JSONL carries source case ids, rule-pack id, rewrite trace,
    /// and the original prompt hash. The export is an artifact, never authority.
    pub async fn materialize_prompt_export(
        &self,
        rule_pack_id: &str,
        rule_pack_version: i32,
        case_ids: &[Uuid],
        exported_by: &str,
        workspace_root: &Path,
    ) -> AtelierResult<PromptExport> {
        if exported_by.trim().is_empty() {
            return Err(AtelierError::Validation(
                "export requires exported_by".to_string(),
            ));
        }
        if case_ids.is_empty() {
            return Err(AtelierError::Validation(
                "export requires at least one case_id".to_string(),
            ));
        }
        if !self.rule_pack_exists(rule_pack_id, rule_pack_version).await? {
            return Err(AtelierError::NotFound(format!(
                "rule pack {rule_pack_id}@{rule_pack_version}"
            )));
        }

        let mut export_rows: Vec<ExportRow> = Vec::with_capacity(case_ids.len());
        let mut rewrite_ids: Vec<Uuid> = Vec::with_capacity(case_ids.len());
        for case_id in case_ids {
            let case = self.get_prompt_case(*case_id).await?;
            let plan = self
                .plan_prompt_rewrite(*case_id, rule_pack_id, rule_pack_version, exported_by)
                .await?;
            rewrite_ids.push(plan.rewrite_id);
            export_rows.push(ExportRow {
                schema_id: "hsk.atelier.prompt_feedback_export_row@1".to_string(),
                source_case_id: case.source_case_id.clone(),
                segment: case.segment.clone(),
                cell: case.cell.clone(),
                render_stack: case.render_stack.clone(),
                rule_pack_id: rule_pack_id.to_string(),
                rule_pack_version,
                // Pure prompt-content hash (feedback-independent provenance, F3).
                original_prompt_hash: plan.outcome.input_hash.clone(),
                rewritten_prompt_hash: plan.output_hash.clone(),
                positive_prompt: plan.rewritten_positive_prompt.clone(),
                negative_prompt: plan.rewritten_negative_prompt.clone(),
                changed_fields: plan.changed_fields.clone(),
                rule_trace: plan.outcome.clone(),
            });
        }

        let bundle = export_jsonl(export_rows)?;
        let payload_bytes = bundle.jsonl.into_bytes();
        let content_hash = sha256_hex(&payload_bytes);
        let byte_len = payload_bytes.len() as i64;

        // F6: if an identical export already exists, reuse its artifact_ref and
        // return it. This avoids writing a second identical blob (which would
        // orphan the prior artifact) and avoids repointing the existing row.
        if let Some(existing) = self
            .find_export_by_content_hash(rule_pack_id, rule_pack_version, &content_hash)
            .await?
        {
            return Ok(existing);
        }

        let artifact_id = Uuid::now_v7();
        let manifest = ArtifactManifest {
            artifact_id,
            layer: ArtifactLayer::L1,
            kind: ArtifactPayloadKind::DatasetSlice,
            mime: "application/jsonl".to_string(),
            filename_hint: Some(format!("prompt_feedback_export_{artifact_id}.jsonl")),
            created_at: chrono::Utc::now(),
            created_by_job_id: None,
            source_entity_refs: Vec::new(),
            source_artifact_refs: Vec::new(),
            content_hash: content_hash.clone(),
            size_bytes: payload_bytes.len() as u64,
            classification: ArtifactClassification::High,
            exportable: true,
            retention_ttl_days: Some(365),
            pinned: Some(true),
            hash_basis: Some(format!(
                "hsk.atelier.prompt_feedback_export@1|{rule_pack_id}@{rule_pack_version}"
            )),
            hash_exclude_paths: Vec::new(),
        };
        write_file_artifact(workspace_root, &manifest, &payload_bytes)
            .map_err(|err| AtelierError::Validation(format!("artifact write failed: {err}")))?;
        let root = artifact_root_rel(ArtifactLayer::L1, artifact_id);
        let artifact_ref = format!("artifact://{root}/payload");
        let manifest_ref = format!("artifact://{root}/artifact.json");

        let source_case_ids_json = to_json(&bundle.source_case_ids)?;
        let rewrite_ids_json = to_json(&rewrite_ids)?;
        let row_count = bundle.row_count as i32;

        // The blob is written once above; only the DB insert (which touches the
        // shared kernel_event_ledger) is retried on a transient deadlock. Retrying
        // reuses the SAME artifact_ref/content_hash, so no orphan blob is created.
        with_retry(|| {
            self.record_prompt_export_txn(
                rule_pack_id,
                rule_pack_version,
                &artifact_ref,
                &manifest_ref,
                &content_hash,
                byte_len,
                row_count,
                &source_case_ids_json,
                &rewrite_ids_json,
                exported_by,
            )
        })
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_prompt_export_txn(
        &self,
        rule_pack_id: &str,
        rule_pack_version: i32,
        artifact_ref: &str,
        manifest_ref: &str,
        content_hash: &str,
        byte_len: i64,
        row_count: i32,
        source_case_ids_json: &serde_json::Value,
        rewrite_ids_json: &serde_json::Value,
        exported_by: &str,
    ) -> AtelierResult<PromptExport> {
        let mut tx = self.pool().begin().await?;
        // ON CONFLICT DO NOTHING (not DO UPDATE): if a concurrent identical
        // export won the race after our pre-check, do not repoint the existing
        // row; fall through and return the winner.
        let inserted = sqlx::query(&format!(
            r#"INSERT INTO atelier_prompt_feedback_export
                 (rule_pack_id, rule_pack_version, artifact_ref, manifest_ref, content_hash,
                  byte_len, row_count, source_case_ids, rewrite_ids, exported_by)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               ON CONFLICT (rule_pack_id, rule_pack_version, content_hash) DO NOTHING
               RETURNING {PROMPT_FEEDBACK_EXPORT_COLUMNS}"#
        ))
        .bind(rule_pack_id)
        .bind(rule_pack_version)
        .bind(artifact_ref)
        .bind(manifest_ref)
        .bind(content_hash)
        .bind(byte_len)
        .bind(row_count)
        .bind(source_case_ids_json)
        .bind(rewrite_ids_json)
        .bind(exported_by)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = inserted else {
            // Lost the race: a concurrent identical export already committed.
            tx.rollback().await?;
            return self
                .find_export_by_content_hash(rule_pack_id, rule_pack_version, content_hash)
                .await?
                .ok_or_else(|| {
                    AtelierError::Conflict(
                        "prompt feedback export row disappeared after conflict".to_string(),
                    )
                });
        };
        let export = export_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            prompt_feedback_event_family::EXPORT_MATERIALIZED,
            "atelier_prompt_feedback_export",
            &export.export_id.to_string(),
            serde_json::json!({
                "export_id": export.export_id,
                "rule_pack_id": export.rule_pack_id,
                "rule_pack_version": export.rule_pack_version,
                "artifact_ref": export.artifact_ref,
                "content_hash": export.content_hash,
                "byte_len": export.byte_len,
                "row_count": export.row_count,
                "schema": "hsk.atelier.prompt_feedback_export@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(export)
    }

    async fn find_export_by_content_hash(
        &self,
        rule_pack_id: &str,
        rule_pack_version: i32,
        content_hash: &str,
    ) -> AtelierResult<Option<PromptExport>> {
        let row = sqlx::query(&format!(
            r#"SELECT {PROMPT_FEEDBACK_EXPORT_COLUMNS}
               FROM atelier_prompt_feedback_export
               WHERE rule_pack_id = $1 AND rule_pack_version = $2 AND content_hash = $3"#
        ))
        .bind(rule_pack_id)
        .bind(rule_pack_version)
        .bind(content_hash)
        .fetch_optional(self.pool())
        .await?;
        row.as_ref().map(export_from_row).transpose()
    }
}

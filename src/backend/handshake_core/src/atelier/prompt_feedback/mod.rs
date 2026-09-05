//! WP-CKC-posekit-overhaul MT-020: deterministic prompt-feedback kernel (SurrealDB port).
//!
//! Atelier/CKC prompt-feedback primitive (handoff:
//! `HANDSHAKE_PROMPT_FEEDBACK_KERNEL_HANDOFF_2026-06-28.md`). It imports CUI/
//! ComfyUI prompt receipts as [`PromptCase`] rows, records reviewer verdicts,
//! deterministically rewrites future prompt rows through versioned rule packs
//! (the pure [`engine`]), and exports corrected machine-readable JSONL as a
//! hashed ArtifactStore artifact.
//!
//! Storage authority is the embedded SurrealDB store (`AtelierStore`) + EventLedger
//! + ArtifactStore only (mirrors the rest of the atelier domain). Every mutation
//! writes its domain row and its atelier event in ONE statement through
//! [`AtelierStore::write_with_event`], so the two commit together or not at all.
//! The JSONL export is a materialized artifact with a content hash and provenance
//! -- never a sidecar system of record. Models are advisors here: every rewrite
//! carries a deterministic rule trace, and a prompt-stress verdict can NEVER
//! become an identity-success verdict.
//!
//! Tables: `atelier_prompt_feedback_{case,verdict,rule_pack,rewrite,export}`. The
//! rule pack record id is the array `[rule_pack_id, version]` (the PostgreSQL
//! composite primary key); rewrite/export rows carry both the scalar
//! `rule_pack_id`/`rule_pack_version` columns and the schema-asserted
//! `rule_pack_ref` record link.
//!
//! SCOPE / DEFERRED: this module is the deterministic prompt-feedback kernel.
//! The Leeseo LoRA-training infra is separate future scope
//! (`leeseo_wishlist.md`: GPU preflight/isolation, kohya experiment scaffolding,
//! pipeline orchestrator, determinism/provenance snapshots, experiment registry,
//! deterministic dataset builder, member-separation assist, faster-training
//! knobs, epoch sweep/keeper selection, training monitor, eval-harness
//! upgrades). Prompt-feedback import/export paths live here; LoRA training does
//! not.

pub mod adapter;
pub mod engine;
pub mod model;

use std::path::Path;
use std::time::Duration;

use serde::de::DeserializeOwned;
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use thiserror::Error;
use uuid::Uuid;

use crate::storage::artifacts::{
    artifact_root_rel, sha256_hex, write_file_artifact, ArtifactClassification, ArtifactLayer,
    ArtifactManifest, ArtifactPayloadKind,
};

use super::{atelier_event_sql, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore};

use self::adapter::{export_jsonl, ExportRow};
use self::engine::{evaluate, Feedback, RewriteOutcome};
use self::model::{
    NewPromptCase, NewReviewVerdict, PromptCase, PromptCaseAxes, PromptExport, ReviewVerdict,
    ReviewerKind, RewritePlan, RewriteRuleSpec, RulePack, VerdictKind,
};

/// EventLedger families emitted by the prompt-feedback kernel (to be folded into
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

const CASE_TABLE: &str = "atelier_prompt_feedback_case";
const VERDICT_TABLE: &str = "atelier_prompt_feedback_verdict";
const RULE_PACK_TABLE: &str = "atelier_prompt_feedback_rule_pack";
const REWRITE_TABLE: &str = "atelier_prompt_feedback_rewrite";
const EXPORT_TABLE: &str = "atelier_prompt_feedback_export";

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

/// True for a transient SurrealDB conflict that is safe to retry. The embedded
/// store aborts one side of a concurrent write-write conflict with a
/// "Transaction conflict ... can be retried" error; the aborted statement wrote
/// nothing (the domain row and its event live in one statement) and can be
/// re-run. The PostgreSQL deadlock/serialization retry maps onto this.
fn is_retryable_transaction_conflict(err: &AtelierError) -> bool {
    match err {
        AtelierError::Database(source) => {
            let text = source.to_string();
            text.contains("Transaction conflict") || text.contains("can be retried")
        }
        _ => false,
    }
}

/// True when the store rejected a write because `index_name` already holds the
/// key. The upsert paths in this module pre-read the existing row and then
/// CREATE or UPDATE; a concurrent writer that lands between the read and the
/// CREATE surfaces here, and the caller re-reads and retries (or returns the
/// winner).
fn is_unique_index_conflict(err: &AtelierError, index_name: &str) -> bool {
    let text = err.to_string();
    text.contains("Database index") && text.contains(index_name) && text.contains("already contains")
}

const RETRY_MAX_ATTEMPTS: u32 = 5;

/// Run a self-contained store op with bounded retry + small backoff when
/// `retryable` says the failure is transient. Every op routed through here is
/// idempotent on retry: it re-reads the row it is about to upsert, so the only
/// committed row is the one that ultimately succeeds.
async fn with_retry<T, F, Fut, P>(op: F, retryable: P) -> AtelierResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = AtelierResult<T>>,
    P: Fn(&AtelierError) -> bool,
{
    let mut attempt: u32 = 0;
    loop {
        match op().await {
            Err(err) if attempt < RETRY_MAX_ATTEMPTS && retryable(&err) => {
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(20 * u64::from(attempt))).await;
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

fn decode_json<T: DeserializeOwned>(column: &str, value: serde_json::Value) -> AtelierResult<T> {
    serde_json::from_value(value).map_err(|err| {
        AtelierError::Internal(format!("failed to decode json column {column}: {err}"))
    })
}

fn to_json<T: serde::Serialize>(value: &T) -> AtelierResult<serde_json::Value> {
    serde_json::to_value(value)
        .map_err(|err| AtelierError::Validation(format!("failed to encode json: {err}")))
}

fn version_from_row(column: &str, value: i64) -> AtelierResult<i32> {
    i32::try_from(value).map_err(|_| {
        AtelierError::Internal(format!("{column} {value} does not fit the i32 API contract"))
    })
}

fn count_from_row(column: &str, value: i64) -> AtelierResult<i32> {
    i32::try_from(value).map_err(|_| {
        AtelierError::Internal(format!("{column} {value} does not fit the i32 API contract"))
    })
}

fn case_ref(case_id: Uuid) -> RecordId {
    RecordId::new(CASE_TABLE, SurrealUuid::from(case_id))
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

// --- Row shapes (what the SELECT projections decode into) -------------------

#[derive(SurrealValue)]
struct PromptCaseRow {
    case_id: SurrealUuid,
    project_id: String,
    source_system: String,
    adapter_id: String,
    source_iteration_id: Option<String>,
    source_case_id: String,
    source_recipe_id: Option<String>,
    segment: String,
    cell: String,
    framing: String,
    clothing_state: String,
    render_stack: String,
    identity_judgement_allowed: bool,
    prompt_quality_review_allowed: bool,
    positive_prompt: String,
    negative_prompt: String,
    micro_gate: Option<String>,
    expected_failure: Option<String>,
    image_artifact_ref: Option<String>,
    sheet_artifact_ref: Option<String>,
    axes: serde_json::Value,
    hardcore_fields: serde_json::Value,
    imported_by: String,
    created_at_utc: Datetime,
}

impl TryFrom<PromptCaseRow> for PromptCase {
    type Error = AtelierError;

    fn try_from(row: PromptCaseRow) -> AtelierResult<Self> {
        Ok(PromptCase {
            case_id: row.case_id.into(),
            project_id: row.project_id,
            source_system: row.source_system,
            adapter_id: row.adapter_id,
            source_iteration_id: row.source_iteration_id,
            source_case_id: row.source_case_id,
            source_recipe_id: row.source_recipe_id,
            segment: row.segment,
            cell: row.cell,
            framing: row.framing,
            clothing_state: row.clothing_state,
            render_stack: row.render_stack,
            identity_judgement_allowed: row.identity_judgement_allowed,
            prompt_quality_review_allowed: row.prompt_quality_review_allowed,
            positive_prompt: row.positive_prompt,
            negative_prompt: row.negative_prompt,
            micro_gate: row.micro_gate,
            expected_failure: row.expected_failure,
            image_artifact_ref: row.image_artifact_ref,
            sheet_artifact_ref: row.sheet_artifact_ref,
            axes: decode_json::<PromptCaseAxes>("axes", row.axes)?,
            hardcore_fields: row.hardcore_fields,
            imported_by: row.imported_by,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct ReviewVerdictRow {
    verdict_id: SurrealUuid,
    case_id: SurrealUuid,
    reviewer_kind: String,
    reviewer_id: String,
    verdict_kind: String,
    failure_class: Option<String>,
    failure_tags: Vec<String>,
    is_identity_judgement: bool,
    note: Option<String>,
    created_at_utc: Datetime,
}

impl TryFrom<ReviewVerdictRow> for ReviewVerdict {
    type Error = AtelierError;

    fn try_from(row: ReviewVerdictRow) -> AtelierResult<Self> {
        Ok(ReviewVerdict {
            verdict_id: row.verdict_id.into(),
            case_id: row.case_id.into(),
            reviewer_kind: ReviewerKind::from_token(&row.reviewer_kind)?,
            reviewer_id: row.reviewer_id,
            verdict_kind: VerdictKind::from_token(&row.verdict_kind)?,
            failure_class: row.failure_class,
            failure_tags: row.failure_tags,
            is_identity_judgement: row.is_identity_judgement,
            note: row.note,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct RulePackRow {
    rule_pack_id: String,
    version: i64,
    title: String,
    description: Option<String>,
    rules: serde_json::Value,
    content_hash: String,
    registered_by: String,
    created_at_utc: Datetime,
}

impl TryFrom<RulePackRow> for RulePack {
    type Error = AtelierError;

    fn try_from(row: RulePackRow) -> AtelierResult<Self> {
        Ok(RulePack {
            rule_pack_id: row.rule_pack_id,
            version: version_from_row("version", row.version)?,
            title: row.title,
            description: row.description,
            rules: decode_json::<Vec<RewriteRuleSpec>>("rules", row.rules)?,
            content_hash: row.content_hash,
            registered_by: row.registered_by,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct RewritePlanRow {
    rewrite_id: SurrealUuid,
    case_id: SurrealUuid,
    source_case_id: String,
    rule_pack_id: String,
    rule_pack_version: i64,
    input_hash: String,
    output_hash: String,
    changed_fields: Vec<String>,
    rewritten_positive_prompt: String,
    rewritten_negative_prompt: String,
    outcome: serde_json::Value,
    planned_by: String,
    created_at_utc: Datetime,
}

impl TryFrom<RewritePlanRow> for RewritePlan {
    type Error = AtelierError;

    fn try_from(row: RewritePlanRow) -> AtelierResult<Self> {
        Ok(RewritePlan {
            rewrite_id: row.rewrite_id.into(),
            case_id: row.case_id.into(),
            source_case_id: row.source_case_id,
            rule_pack_id: row.rule_pack_id,
            rule_pack_version: version_from_row("rule_pack_version", row.rule_pack_version)?,
            input_hash: row.input_hash,
            output_hash: row.output_hash,
            changed_fields: row.changed_fields,
            rewritten_positive_prompt: row.rewritten_positive_prompt,
            rewritten_negative_prompt: row.rewritten_negative_prompt,
            outcome: decode_json::<RewriteOutcome>("outcome", row.outcome)?,
            planned_by: row.planned_by,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct PromptExportRow {
    export_id: SurrealUuid,
    rule_pack_id: String,
    rule_pack_version: i64,
    artifact_ref: String,
    manifest_ref: Option<String>,
    content_hash: String,
    byte_len: i64,
    row_count: i64,
    source_case_ids: Vec<String>,
    rewrite_ids: Vec<SurrealUuid>,
    exported_by: String,
    created_at_utc: Datetime,
}

impl TryFrom<PromptExportRow> for PromptExport {
    type Error = AtelierError;

    fn try_from(row: PromptExportRow) -> AtelierResult<Self> {
        Ok(PromptExport {
            export_id: row.export_id.into(),
            rule_pack_id: row.rule_pack_id,
            rule_pack_version: version_from_row("rule_pack_version", row.rule_pack_version)?,
            artifact_ref: row.artifact_ref,
            manifest_ref: row.manifest_ref,
            content_hash: row.content_hash,
            byte_len: row.byte_len,
            row_count: count_from_row("row_count", row.row_count)?,
            source_case_ids: row.source_case_ids,
            rewrite_ids: row.rewrite_ids.into_iter().map(Into::into).collect(),
            exported_by: row.exported_by,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct RecordIdRow {
    id: RecordId,
}

// --- SELECT projections -----------------------------------------------------

macro_rules! prompt_case_select {
    () => {
        "case_id, project_id, source_system, adapter_id, source_iteration_id, source_case_id, \
         source_recipe_id, segment, cell, framing, clothing_state, render_stack, \
         identity_judgement_allowed, prompt_quality_review_allowed, positive_prompt, \
         negative_prompt, micro_gate, expected_failure, image_artifact_ref, sheet_artifact_ref, \
         axes, hardcore_fields, imported_by, created_at_utc"
    };
}

macro_rules! verdict_select {
    () => {
        "verdict_id, record::id(case_id) AS case_id, reviewer_kind, reviewer_id, verdict_kind, \
         failure_class, failure_tags, is_identity_judgement, note, created_at_utc"
    };
}

macro_rules! rule_pack_select {
    () => {
        "rule_pack_id, version, title, description, rules, content_hash, registered_by, \
         created_at_utc"
    };
}

macro_rules! rewrite_select {
    () => {
        "rewrite_id, record::id(case_id) AS case_id, source_case_id, rule_pack_id, \
         rule_pack_version, input_hash, output_hash, changed_fields, rewritten_positive_prompt, \
         rewritten_negative_prompt, outcome, planned_by, created_at_utc"
    };
}

macro_rules! export_select {
    () => {
        "export_id, rule_pack_id, rule_pack_version, artifact_ref, manifest_ref, content_hash, \
         byte_len, row_count, source_case_ids, rewrite_ids, exported_by, created_at_utc"
    };
}

// --- Bindings ---------------------------------------------------------------

#[derive(Clone, SurrealValue)]
struct PromptCaseWriteBindings {
    case_rid: RecordId,
    case_id: SurrealUuid,
    project_id: String,
    source_system: String,
    adapter_id: String,
    source_iteration_id: Option<String>,
    source_case_id: String,
    source_recipe_id: Option<String>,
    segment: String,
    cell: String,
    framing: String,
    clothing_state: String,
    render_stack: String,
    identity_judgement_allowed: bool,
    prompt_quality_review_allowed: bool,
    positive_prompt: String,
    negative_prompt: String,
    micro_gate: Option<String>,
    expected_failure: Option<String>,
    image_artifact_ref: Option<String>,
    sheet_artifact_ref: Option<String>,
    axes: serde_json::Value,
    hardcore_fields: serde_json::Value,
    imported_by: String,
}

#[derive(SurrealValue)]
struct CaseSourceKeyBindings {
    adapter_id: String,
    source_case_id: String,
}

#[derive(SurrealValue)]
struct CaseIdBinding {
    case_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct CaseRefBinding {
    case_ref: RecordId,
}

#[derive(SurrealValue)]
struct ListPromptCasesBindings {
    project_id: Option<String>,
    segment: Option<String>,
    cell: Option<String>,
    render_stack: Option<String>,
    limit: i64,
}

#[derive(Clone, SurrealValue)]
struct VerdictWriteBindings {
    verdict_rid: RecordId,
    verdict_id: SurrealUuid,
    case_ref: RecordId,
    reviewer_kind: String,
    reviewer_id: String,
    verdict_kind: String,
    failure_class: Option<String>,
    failure_tags: Vec<String>,
    is_identity_judgement: bool,
    note: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct RulePackWriteBindings {
    rule_pack_id: String,
    version: i64,
    title: String,
    description: Option<String>,
    rules: serde_json::Value,
    content_hash: String,
    registered_by: String,
}

#[derive(SurrealValue)]
struct RulePackKeyBindings {
    rule_pack_id: String,
    version: i64,
}

#[derive(Clone, SurrealValue)]
struct RewriteWriteBindings {
    rewrite_rid: RecordId,
    rewrite_id: SurrealUuid,
    case_ref: RecordId,
    source_case_id: String,
    rule_pack_id: String,
    rule_pack_version: i64,
    input_hash: String,
    output_hash: String,
    changed_fields: Vec<String>,
    rewritten_positive_prompt: String,
    rewritten_negative_prompt: String,
    outcome: serde_json::Value,
    planned_by: String,
}

#[derive(SurrealValue)]
struct RewriteKeyBindings {
    case_ref: RecordId,
    rule_pack_id: String,
    rule_pack_version: i64,
    input_hash: String,
}

#[derive(Clone, SurrealValue)]
struct ExportWriteBindings {
    export_rid: RecordId,
    export_id: SurrealUuid,
    rule_pack_id: String,
    rule_pack_version: i64,
    artifact_ref: String,
    manifest_ref: Option<String>,
    content_hash: String,
    byte_len: i64,
    row_count: i64,
    source_case_ids: Vec<String>,
    rewrite_ids: Vec<SurrealUuid>,
    exported_by: String,
}

#[derive(SurrealValue)]
struct ExportContentKeyBindings {
    rule_pack_id: String,
    rule_pack_version: i64,
    content_hash: String,
}

// --- Statements -------------------------------------------------------------
//
// Every write is one `RETURN { ... }` block: the event fragment and the domain
// row land in ONE statement, so they commit together or not at all (the
// guarantee the PostgreSQL `pool.begin()` transactions provided). Upserts are a
// Rust-side pre-read of the idempotency key followed by a CREATE or an UPDATE;
// the schema's UNIQUE indexes close the race window and the caller retries.

const FIND_CASE_BY_SOURCE_STATEMENT: &str = concat!(
    "SELECT ",
    prompt_case_select!(),
    " FROM atelier_prompt_feedback_case \
       WHERE adapter_id = $adapter_id AND source_case_id = $source_case_id LIMIT 1;"
);

const CREATE_CASE_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.case_rid; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         case_id: $domain.case_id, project_id: $domain.project_id, \
         source_system: $domain.source_system, adapter_id: $domain.adapter_id, \
         source_iteration_id: $domain.source_iteration_id, source_case_id: $domain.source_case_id, \
         source_recipe_id: $domain.source_recipe_id, segment: $domain.segment, cell: $domain.cell, \
         framing: $domain.framing, clothing_state: $domain.clothing_state, \
         render_stack: $domain.render_stack, \
         identity_judgement_allowed: $domain.identity_judgement_allowed, \
         prompt_quality_review_allowed: $domain.prompt_quality_review_allowed, \
         positive_prompt: $domain.positive_prompt, negative_prompt: $domain.negative_prompt, \
         micro_gate: $domain.micro_gate, expected_failure: $domain.expected_failure, \
         image_artifact_ref: $domain.image_artifact_ref, \
         sheet_artifact_ref: $domain.sheet_artifact_ref, axes: $domain.axes, \
         hardcore_fields: $domain.hardcore_fields, imported_by: $domain.imported_by \
       }; RETURN (SELECT ",
    prompt_case_select!(),
    " FROM $rid); };"
);

/// Re-import of an existing `(adapter_id, source_case_id)`: update in place, keep
/// `case_id` and `created_at_utc` (the PostgreSQL `ON CONFLICT DO UPDATE` shape).
const UPDATE_CASE_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.case_rid; ",
    atelier_event_sql!(),
    " UPDATE $rid SET \
         project_id = $domain.project_id, source_system = $domain.source_system, \
         source_iteration_id = $domain.source_iteration_id, \
         source_recipe_id = $domain.source_recipe_id, segment = $domain.segment, \
         cell = $domain.cell, framing = $domain.framing, clothing_state = $domain.clothing_state, \
         render_stack = $domain.render_stack, \
         identity_judgement_allowed = $domain.identity_judgement_allowed, \
         prompt_quality_review_allowed = $domain.prompt_quality_review_allowed, \
         positive_prompt = $domain.positive_prompt, negative_prompt = $domain.negative_prompt, \
         micro_gate = $domain.micro_gate, expected_failure = $domain.expected_failure, \
         image_artifact_ref = $domain.image_artifact_ref, \
         sheet_artifact_ref = $domain.sheet_artifact_ref, axes = $domain.axes, \
         hardcore_fields = $domain.hardcore_fields, imported_by = $domain.imported_by; \
       RETURN (SELECT ",
    prompt_case_select!(),
    " FROM $rid); };"
);

const LIST_CASES_STATEMENT: &str = concat!(
    "SELECT ",
    prompt_case_select!(),
    " FROM atelier_prompt_feedback_case \
       WHERE ($project_id IS NONE OR project_id = $project_id) \
         AND ($segment IS NONE OR segment = $segment) \
         AND ($cell IS NONE OR cell = $cell) \
         AND ($render_stack IS NONE OR render_stack = $render_stack) \
       ORDER BY created_at_utc DESC, case_id ASC LIMIT $limit;"
);

const GET_CASE_STATEMENT: &str = concat!(
    "SELECT ",
    prompt_case_select!(),
    " FROM atelier_prompt_feedback_case WHERE case_id = $case_id LIMIT 1;"
);

const CREATE_VERDICT_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.verdict_rid; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         verdict_id: $domain.verdict_id, case_id: $domain.case_ref, \
         reviewer_kind: $domain.reviewer_kind, reviewer_id: $domain.reviewer_id, \
         verdict_kind: $domain.verdict_kind, failure_class: $domain.failure_class, \
         failure_tags: $domain.failure_tags, is_identity_judgement: $domain.is_identity_judgement, \
         note: $domain.note \
       }; RETURN (SELECT ",
    verdict_select!(),
    " FROM $rid); };"
);

const LIST_VERDICTS_STATEMENT: &str = concat!(
    "SELECT ",
    verdict_select!(),
    " FROM atelier_prompt_feedback_verdict WHERE case_id = $case_ref \
       ORDER BY created_at_utc DESC, verdict_id ASC;"
);

/// Register-or-update on the array record id `[rule_pack_id, version]`; an
/// existing pack keeps its `created_at_utc`.
const UPSERT_RULE_PACK_STATEMENT: &str = concat!(
    "RETURN { LET $rid = type::record('atelier_prompt_feedback_rule_pack', \
         [$domain.rule_pack_id, $domain.version]); ",
    atelier_event_sql!(),
    " IF (SELECT VALUE id FROM $rid)[0] = NONE { \
         CREATE $rid CONTENT { \
           rule_pack_id: $domain.rule_pack_id, version: $domain.version, title: $domain.title, \
           description: $domain.description, rules: $domain.rules, \
           content_hash: $domain.content_hash, registered_by: $domain.registered_by \
         }; \
       } ELSE { \
         UPDATE $rid SET title = $domain.title, description = $domain.description, \
           rules = $domain.rules, content_hash = $domain.content_hash, \
           registered_by = $domain.registered_by; \
       }; RETURN (SELECT ",
    rule_pack_select!(),
    " FROM $rid); };"
);

const LIST_RULE_PACKS_STATEMENT: &str = concat!(
    "SELECT ",
    rule_pack_select!(),
    " FROM atelier_prompt_feedback_rule_pack ORDER BY rule_pack_id ASC, version DESC;"
);

const RULE_PACK_EXISTS_STATEMENT: &str = "SELECT id FROM atelier_prompt_feedback_rule_pack \
     WHERE rule_pack_id = $rule_pack_id AND version = $version LIMIT 1;";

const FIND_REWRITE_BY_KEY_STATEMENT: &str = concat!(
    "SELECT ",
    rewrite_select!(),
    " FROM atelier_prompt_feedback_rewrite \
       WHERE case_id = $case_ref AND rule_pack_id = $rule_pack_id \
         AND rule_pack_version = $rule_pack_version AND input_hash = $input_hash LIMIT 1;"
);

const CREATE_REWRITE_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.rewrite_rid; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         rewrite_id: $domain.rewrite_id, case_id: $domain.case_ref, \
         source_case_id: $domain.source_case_id, rule_pack_id: $domain.rule_pack_id, \
         rule_pack_version: $domain.rule_pack_version, \
         rule_pack_ref: type::record('atelier_prompt_feedback_rule_pack', \
           [$domain.rule_pack_id, $domain.rule_pack_version]), \
         input_hash: $domain.input_hash, output_hash: $domain.output_hash, \
         changed_fields: $domain.changed_fields, \
         rewritten_positive_prompt: $domain.rewritten_positive_prompt, \
         rewritten_negative_prompt: $domain.rewritten_negative_prompt, \
         outcome: $domain.outcome, planned_by: $domain.planned_by \
       }; RETURN (SELECT ",
    rewrite_select!(),
    " FROM $rid); };"
);

/// Re-plan with an unchanged idempotency key: refresh the output columns in
/// place (the PostgreSQL `ON CONFLICT DO UPDATE` shape).
const UPDATE_REWRITE_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.rewrite_rid; ",
    atelier_event_sql!(),
    " UPDATE $rid SET output_hash = $domain.output_hash, \
         changed_fields = $domain.changed_fields, \
         rewritten_positive_prompt = $domain.rewritten_positive_prompt, \
         rewritten_negative_prompt = $domain.rewritten_negative_prompt, \
         outcome = $domain.outcome, planned_by = $domain.planned_by; \
       RETURN (SELECT ",
    rewrite_select!(),
    " FROM $rid); };"
);

const CREATE_EXPORT_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.export_rid; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         export_id: $domain.export_id, rule_pack_id: $domain.rule_pack_id, \
         rule_pack_version: $domain.rule_pack_version, \
         rule_pack_ref: type::record('atelier_prompt_feedback_rule_pack', \
           [$domain.rule_pack_id, $domain.rule_pack_version]), \
         artifact_ref: $domain.artifact_ref, manifest_ref: $domain.manifest_ref, \
         content_hash: $domain.content_hash, byte_len: $domain.byte_len, \
         row_count: $domain.row_count, source_case_ids: $domain.source_case_ids, \
         rewrite_ids: $domain.rewrite_ids, exported_by: $domain.exported_by \
       }; RETURN (SELECT ",
    export_select!(),
    " FROM $rid); };"
);

const FIND_EXPORT_BY_CONTENT_STATEMENT: &str = concat!(
    "SELECT ",
    export_select!(),
    " FROM atelier_prompt_feedback_export \
       WHERE rule_pack_id = $rule_pack_id AND rule_pack_version = $rule_pack_version \
         AND content_hash = $content_hash LIMIT 1;"
);

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

fn case_imported_event_payload(case: &PromptCase) -> serde_json::Value {
    let mut event_payload = serde_json::json!({
        "case_id": case.case_id,
        "project_id": case.project_id,
        "source_system": case.source_system,
        "adapter_id": case.adapter_id,
        "source_iteration_id": case.source_iteration_id,
        "source_case_id": case.source_case_id,
        "source_recipe_id": case.source_recipe_id,
        "segment": case.segment,
        "cell": case.cell,
        "render_stack": case.render_stack,
        "identity_judgement_allowed": case.identity_judgement_allowed,
        "prompt_quality_review_allowed": case.prompt_quality_review_allowed,
        "schema": "hsk.atelier.prompt_feedback_case@1",
    });
    if let Some(csv) = case
        .hardcore_fields
        .get("csv")
        .and_then(|value| value.as_object())
    {
        let mut csv_lineage = serde_json::Map::new();
        for key in ["source_format", "source_manifest_ref", "row_number", "row_hash"] {
            if let Some(value) = csv.get(key) {
                csv_lineage.insert(key.to_string(), value.clone());
            }
        }
        if !csv_lineage.is_empty() {
            event_payload["csv"] = serde_json::Value::Object(csv_lineage);
        }
    }
    event_payload
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
    /// it in place. One `CASE_IMPORTED` event is emitted per case, in the same
    /// statement as that case's row.
    ///
    /// The whole batch is validated before anything is written; each case then
    /// commits atomically with its own event. (The PostgreSQL version wrapped the
    /// whole batch in one transaction; the embedded store's one-statement event
    /// fragment carries one event, so the unit of atomicity here is the case.)
    pub async fn import_prompt_cases(
        &self,
        cases: &[NewPromptCase],
    ) -> AtelierResult<Vec<PromptCase>> {
        for case in cases {
            validate_new_prompt_case(case)?;
        }
        let mut imported = Vec::with_capacity(cases.len());
        for new in cases {
            let case = with_retry(
                || self.upsert_prompt_case(new),
                |err| {
                    is_retryable_transaction_conflict(err)
                        || is_unique_index_conflict(err, "ux_atelier_prompt_feedback_case_source")
                },
            )
            .await?;
            imported.push(case);
        }
        Ok(imported)
    }

    async fn find_prompt_case_by_source(
        &self,
        adapter_id: &str,
        source_case_id: &str,
    ) -> AtelierResult<Option<PromptCase>> {
        let bindings = CaseSourceKeyBindings {
            adapter_id: adapter_id.to_owned(),
            source_case_id: source_case_id.to_owned(),
        };
        let row: Option<PromptCaseRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(FIND_CASE_BY_SOURCE_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        row.map(PromptCase::try_from).transpose()
    }

    async fn upsert_prompt_case(&self, new: &NewPromptCase) -> AtelierResult<PromptCase> {
        let existing = self
            .find_prompt_case_by_source(&new.adapter_id, &new.source_case_id)
            .await?;
        let (case_id, statement) = match &existing {
            Some(existing) => (existing.case_id, UPDATE_CASE_STATEMENT),
            None => (Uuid::now_v7(), CREATE_CASE_STATEMENT),
        };
        let axes = to_json(&new.axes)?;
        let bindings = PromptCaseWriteBindings {
            case_rid: case_ref(case_id),
            case_id: SurrealUuid::from(case_id),
            project_id: new.project_id.clone(),
            source_system: new.source_system.clone(),
            adapter_id: new.adapter_id.clone(),
            source_iteration_id: new.source_iteration_id.clone(),
            source_case_id: new.source_case_id.clone(),
            source_recipe_id: new.source_recipe_id.clone(),
            segment: new.segment.clone(),
            cell: new.cell.clone(),
            framing: new.framing.clone(),
            clothing_state: new.clothing_state.clone(),
            render_stack: new.render_stack.clone(),
            identity_judgement_allowed: new.identity_judgement_allowed,
            prompt_quality_review_allowed: new.prompt_quality_review_allowed,
            positive_prompt: new.positive_prompt.clone(),
            negative_prompt: new.negative_prompt.clone(),
            micro_gate: new.micro_gate.clone(),
            expected_failure: new.expected_failure.clone(),
            image_artifact_ref: new.image_artifact_ref.clone(),
            sheet_artifact_ref: new.sheet_artifact_ref.clone(),
            axes,
            hardcore_fields: new.hardcore_fields.clone(),
            imported_by: new.imported_by.clone(),
        };
        // The event payload describes the row as it will be persisted; the
        // stored row is re-read from the statement's own RETURN.
        let projected = PromptCase {
            case_id,
            project_id: new.project_id.clone(),
            source_system: new.source_system.clone(),
            adapter_id: new.adapter_id.clone(),
            source_iteration_id: new.source_iteration_id.clone(),
            source_case_id: new.source_case_id.clone(),
            source_recipe_id: new.source_recipe_id.clone(),
            segment: new.segment.clone(),
            cell: new.cell.clone(),
            framing: new.framing.clone(),
            clothing_state: new.clothing_state.clone(),
            render_stack: new.render_stack.clone(),
            identity_judgement_allowed: new.identity_judgement_allowed,
            prompt_quality_review_allowed: new.prompt_quality_review_allowed,
            positive_prompt: new.positive_prompt.clone(),
            negative_prompt: new.negative_prompt.clone(),
            micro_gate: new.micro_gate.clone(),
            expected_failure: new.expected_failure.clone(),
            image_artifact_ref: new.image_artifact_ref.clone(),
            sheet_artifact_ref: new.sheet_artifact_ref.clone(),
            axes: new.axes.clone(),
            hardcore_fields: new.hardcore_fields.clone(),
            imported_by: new.imported_by.clone(),
            created_at_utc: chrono::Utc::now(),
        };
        let row: Option<PromptCaseRow> = self
            .write_with_event(
                statement,
                bindings,
                prompt_feedback_event_family::CASE_IMPORTED,
                CASE_TABLE,
                &case_id.to_string(),
                case_imported_event_payload(&projected),
            )
            .await?;
        row.map(PromptCase::try_from).transpose()?.ok_or_else(|| {
            AtelierError::Internal("importing a prompt case returned no row".to_owned())
        })
    }

    /// List prompt cases, newest first, filtered by project/segment/cell/render
    /// stack when provided.
    pub async fn list_prompt_cases(
        &self,
        filter: &PromptCaseFilter,
    ) -> AtelierResult<Vec<PromptCase>> {
        let limit = filter.limit.unwrap_or(200).clamp(1, 500);
        let bindings = ListPromptCasesBindings {
            project_id: filter.project_id.clone(),
            segment: filter.segment.clone(),
            cell: filter.cell.clone(),
            render_stack: filter.render_stack.clone(),
            limit,
        };
        let rows: Vec<PromptCaseRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_CASES_STATEMENT, bindings).await })
            })
            .await?;
        rows.into_iter().map(PromptCase::try_from).collect()
    }

    pub async fn get_prompt_case(&self, case_id: Uuid) -> AtelierResult<PromptCase> {
        let bindings = CaseIdBinding {
            case_id: SurrealUuid::from(case_id),
        };
        let row: Option<PromptCaseRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_CASE_STATEMENT, bindings).await })
            })
            .await?;
        row.map(PromptCase::try_from)
            .transpose()?
            .ok_or_else(|| AtelierError::NotFound(format!("prompt case case_id={case_id}")))
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
        if let Some(class) = new.failure_class.as_deref() {
            if class.trim().is_empty() || class.trim() != class {
                return Err(AtelierError::Validation(
                    "failure_class must not be empty or padded".to_string(),
                ));
            }
        }
        if let Some(note) = new.note.as_deref() {
            if note.trim().is_empty() || note.trim() != note {
                return Err(AtelierError::Validation(
                    "note must not be empty or padded".to_string(),
                ));
            }
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
        with_retry(
            || self.create_prompt_verdict(new),
            is_retryable_transaction_conflict,
        )
        .await
    }

    async fn create_prompt_verdict(&self, new: &NewReviewVerdict) -> AtelierResult<ReviewVerdict> {
        let verdict_id = Uuid::now_v7();
        let bindings = VerdictWriteBindings {
            verdict_rid: RecordId::new(VERDICT_TABLE, SurrealUuid::from(verdict_id)),
            verdict_id: SurrealUuid::from(verdict_id),
            case_ref: case_ref(new.case_id),
            reviewer_kind: new.reviewer_kind.as_token().to_owned(),
            reviewer_id: new.reviewer_id.clone(),
            verdict_kind: new.verdict_kind.as_token().to_owned(),
            failure_class: new.failure_class.clone(),
            failure_tags: new.failure_tags.clone(),
            is_identity_judgement: new.is_identity_judgement,
            note: new.note.clone(),
        };
        let row: Option<ReviewVerdictRow> = self
            .write_with_event(
                CREATE_VERDICT_STATEMENT,
                bindings,
                prompt_feedback_event_family::VERDICT_RECORDED,
                VERDICT_TABLE,
                &verdict_id.to_string(),
                serde_json::json!({
                    "verdict_id": verdict_id,
                    "case_id": new.case_id,
                    "reviewer_kind": new.reviewer_kind.as_token(),
                    "verdict_kind": new.verdict_kind.as_token(),
                    "failure_class": new.failure_class,
                    "is_identity_judgement": new.is_identity_judgement,
                    "schema": "hsk.atelier.prompt_feedback_verdict@1",
                }),
            )
            .await?;
        row.map(ReviewVerdict::try_from).transpose()?.ok_or_else(|| {
            AtelierError::Internal("recording a prompt verdict returned no row".to_owned())
        })
    }

    pub async fn list_prompt_verdicts(&self, case_id: Uuid) -> AtelierResult<Vec<ReviewVerdict>> {
        let bindings = CaseRefBinding {
            case_ref: case_ref(case_id),
        };
        let rows: Vec<ReviewVerdictRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_VERDICTS_STATEMENT, bindings).await })
            })
            .await?;
        rows.into_iter().map(ReviewVerdict::try_from).collect()
    }

    /// Register (or update) a versioned rule pack, keyed on `(rule_pack_id,
    /// version)` (the array record id). The content hash pins the rule
    /// descriptors for determinism.
    pub async fn register_rule_pack(
        &self,
        rule_pack_id: &str,
        version: i32,
        title: &str,
        description: Option<&str>,
        rules: &[RewriteRuleSpec],
        registered_by: &str,
    ) -> AtelierResult<RulePack> {
        if rule_pack_id.trim().is_empty()
            || rule_pack_id.trim() != rule_pack_id
            || version < 1
            || registered_by.trim().is_empty()
            || registered_by.trim() != registered_by
        {
            return Err(AtelierError::Validation(
                "rule pack requires a non-empty id, version >= 1, and registered_by".to_string(),
            ));
        }
        let rules_json = to_json(&rules)?;
        let content_hash = format!(
            "sha256:{}",
            sha256_hex(
                serde_json::to_string(&rules_json)
                    .unwrap_or_default()
                    .as_bytes()
            )
        );
        let bindings = RulePackWriteBindings {
            rule_pack_id: rule_pack_id.to_owned(),
            version: i64::from(version),
            title: title.to_owned(),
            description: description.map(ToOwned::to_owned),
            rules: rules_json,
            content_hash: content_hash.clone(),
            registered_by: registered_by.to_owned(),
        };
        let rule_count = rules.len();
        with_retry(
            || {
                let bindings = bindings.clone();
                let content_hash = content_hash.clone();
                async move {
                    let row: Option<RulePackRow> = self
                        .write_with_event(
                            UPSERT_RULE_PACK_STATEMENT,
                            bindings,
                            prompt_feedback_event_family::RULEPACK_REGISTERED,
                            RULE_PACK_TABLE,
                            &format!("{rule_pack_id}@{version}"),
                            serde_json::json!({
                                "rule_pack_id": rule_pack_id,
                                "version": version,
                                "content_hash": content_hash,
                                "rule_count": rule_count,
                                "schema": "hsk.atelier.prompt_feedback_rule_pack@1",
                            }),
                        )
                        .await?;
                    row.map(RulePack::try_from).transpose()?.ok_or_else(|| {
                        AtelierError::Internal("registering a rule pack returned no row".to_owned())
                    })
                }
            },
            is_retryable_transaction_conflict,
        )
        .await
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
        let rows: Vec<RulePackRow> = self
            .store()
            .with_data_operation(|ctx| {
                Box::pin(async move { ctx.query_values(LIST_RULE_PACKS_STATEMENT, ()).await })
            })
            .await?;
        rows.into_iter().map(RulePack::try_from).collect()
    }

    async fn rule_pack_exists(&self, rule_pack_id: &str, version: i32) -> AtelierResult<bool> {
        let bindings = RulePackKeyBindings {
            rule_pack_id: rule_pack_id.to_owned(),
            version: i64::from(version),
        };
        let row: Option<RecordIdRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(RULE_PACK_EXISTS_STATEMENT, bindings).await })
            })
            .await?;
        Ok(row.is_some())
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
        if planned_by.trim().is_empty() || planned_by.trim() != planned_by {
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
        if !self
            .rule_pack_exists(rule_pack_id, rule_pack_version)
            .await?
        {
            return Err(AtelierError::NotFound(format!(
                "rule pack {rule_pack_id}@{rule_pack_version}"
            )));
        }
        with_retry(
            || self.upsert_prompt_rewrite(case_id, rule_pack_id, rule_pack_version, planned_by),
            |err| {
                is_retryable_transaction_conflict(err)
                    || is_unique_index_conflict(
                        err,
                        "ux_atelier_prompt_feedback_rewrite_determinism",
                    )
            },
        )
        .await
    }

    async fn find_rewrite_by_key(
        &self,
        case_id: Uuid,
        rule_pack_id: &str,
        rule_pack_version: i32,
        input_hash: &str,
    ) -> AtelierResult<Option<RewritePlan>> {
        let bindings = RewriteKeyBindings {
            case_ref: case_ref(case_id),
            rule_pack_id: rule_pack_id.to_owned(),
            rule_pack_version: i64::from(rule_pack_version),
            input_hash: input_hash.to_owned(),
        };
        let row: Option<RewritePlanRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(FIND_REWRITE_BY_KEY_STATEMENT, bindings).await })
            })
            .await?;
        row.map(RewritePlan::try_from).transpose()
    }

    async fn upsert_prompt_rewrite(
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
        let existing = self
            .find_rewrite_by_key(case_id, rule_pack_id, rule_pack_version, &stored_input_hash)
            .await?;
        let (rewrite_id, statement) = match &existing {
            Some(existing) => (existing.rewrite_id, UPDATE_REWRITE_STATEMENT),
            None => (Uuid::now_v7(), CREATE_REWRITE_STATEMENT),
        };
        let outcome_json = to_json(&outcome)?;
        let bindings = RewriteWriteBindings {
            rewrite_rid: RecordId::new(REWRITE_TABLE, SurrealUuid::from(rewrite_id)),
            rewrite_id: SurrealUuid::from(rewrite_id),
            case_ref: case_ref(case_id),
            source_case_id: case.source_case_id.clone(),
            rule_pack_id: rule_pack_id.to_owned(),
            rule_pack_version: i64::from(rule_pack_version),
            input_hash: stored_input_hash.clone(),
            output_hash: outcome.output_hash.clone(),
            changed_fields: outcome.changed_fields.clone(),
            rewritten_positive_prompt: outcome.rewritten.positive_prompt.clone(),
            rewritten_negative_prompt: outcome.rewritten.negative_prompt.clone(),
            outcome: outcome_json,
            planned_by: planned_by.to_owned(),
        };
        let row: Option<RewritePlanRow> = self
            .write_with_event(
                statement,
                bindings,
                prompt_feedback_event_family::REWRITE_PLANNED,
                REWRITE_TABLE,
                &rewrite_id.to_string(),
                serde_json::json!({
                    "rewrite_id": rewrite_id,
                    "case_id": case_id,
                    "rule_pack_id": rule_pack_id,
                    "rule_pack_version": rule_pack_version,
                    "input_hash": stored_input_hash,
                    "output_hash": outcome.output_hash,
                    "changed_field_count": outcome.changed_fields.len(),
                    "schema": "hsk.atelier.prompt_feedback_rewrite@1",
                }),
            )
            .await?;
        row.map(RewritePlan::try_from).transpose()?.ok_or_else(|| {
            AtelierError::Internal("planning a prompt rewrite returned no row".to_owned())
        })
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
        if exported_by.trim().is_empty() || exported_by.trim() != exported_by {
            return Err(AtelierError::Validation(
                "export requires exported_by".to_string(),
            ));
        }
        if case_ids.is_empty() {
            return Err(AtelierError::Validation(
                "export requires at least one case_id".to_string(),
            ));
        }
        if !self
            .rule_pack_exists(rule_pack_id, rule_pack_version)
            .await?
        {
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

        let row_count = count_from_row("row_count", bundle.row_count as i64)?;

        // The blob is written once above; only the store write (which touches the
        // shared kernel_event_ledger) is retried on a transient conflict. Retrying
        // reuses the SAME artifact_ref/content_hash, so no orphan blob is created.
        with_retry(
            || {
                self.record_prompt_export(
                    rule_pack_id,
                    rule_pack_version,
                    &artifact_ref,
                    &manifest_ref,
                    &content_hash,
                    byte_len,
                    row_count,
                    &bundle.source_case_ids,
                    &rewrite_ids,
                    exported_by,
                )
            },
            is_retryable_transaction_conflict,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_prompt_export(
        &self,
        rule_pack_id: &str,
        rule_pack_version: i32,
        artifact_ref: &str,
        manifest_ref: &str,
        content_hash: &str,
        byte_len: i64,
        row_count: i32,
        source_case_ids: &[String],
        rewrite_ids: &[Uuid],
        exported_by: &str,
    ) -> AtelierResult<PromptExport> {
        let export_id = Uuid::now_v7();
        let bindings = ExportWriteBindings {
            export_rid: RecordId::new(EXPORT_TABLE, SurrealUuid::from(export_id)),
            export_id: SurrealUuid::from(export_id),
            rule_pack_id: rule_pack_id.to_owned(),
            rule_pack_version: i64::from(rule_pack_version),
            artifact_ref: artifact_ref.to_owned(),
            manifest_ref: Some(manifest_ref.to_owned()),
            content_hash: content_hash.to_owned(),
            byte_len,
            row_count: i64::from(row_count),
            source_case_ids: source_case_ids.to_vec(),
            rewrite_ids: rewrite_ids.iter().copied().map(SurrealUuid::from).collect(),
            exported_by: exported_by.to_owned(),
        };
        let written: AtelierResult<Option<PromptExportRow>> = self
            .write_with_event(
                CREATE_EXPORT_STATEMENT,
                bindings,
                prompt_feedback_event_family::EXPORT_MATERIALIZED,
                EXPORT_TABLE,
                &export_id.to_string(),
                serde_json::json!({
                    "export_id": export_id,
                    "rule_pack_id": rule_pack_id,
                    "rule_pack_version": rule_pack_version,
                    "artifact_ref": artifact_ref,
                    "content_hash": content_hash,
                    "byte_len": byte_len,
                    "row_count": row_count,
                    "schema": "hsk.atelier.prompt_feedback_export@1",
                }),
            )
            .await;
        match written {
            Ok(row) => row.map(PromptExport::try_from).transpose()?.ok_or_else(|| {
                AtelierError::Internal("recording a prompt export returned no row".to_owned())
            }),
            // Lost the race: a concurrent identical export committed after our
            // pre-check. The UNIQUE index rejected this row (and, with it, this
            // statement's event), so do not repoint anything; return the winner.
            Err(err) if is_unique_index_conflict(&err, "ux_atelier_prompt_feedback_export_content") => {
                self.find_export_by_content_hash(rule_pack_id, rule_pack_version, content_hash)
                    .await?
                    .ok_or_else(|| {
                        AtelierError::Conflict(
                            "prompt feedback export row disappeared after conflict".to_string(),
                        )
                    })
            }
            Err(err) => Err(err),
        }
    }

    async fn find_export_by_content_hash(
        &self,
        rule_pack_id: &str,
        rule_pack_version: i32,
        content_hash: &str,
    ) -> AtelierResult<Option<PromptExport>> {
        let bindings = ExportContentKeyBindings {
            rule_pack_id: rule_pack_id.to_owned(),
            rule_pack_version: i64::from(rule_pack_version),
            content_hash: content_hash.to_owned(),
        };
        let row: Option<PromptExportRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(FIND_EXPORT_BY_CONTENT_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        row.map(PromptExport::try_from).transpose()
    }
}

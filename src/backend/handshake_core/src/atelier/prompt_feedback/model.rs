//! WP-CKC-posekit-overhaul MT-020: prompt-feedback persistence records.
//!
//! Plain data records for the deterministic prompt-feedback kernel. The embedded
//! SurrealDB store + EventLedger is the authority (see `super`); these structs are
//! the typed shapes the store reads/writes and the API serializes. No SurrealDB
//! row wiring lives here (that is in `super`), so this module stays a clean data
//! contract.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::engine::{EngineCase, RewriteOutcome};
use super::PromptFeedbackError;

/// Who authored a review verdict (handoff data-model `reviewer_kind`).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerKind {
    Operator,
    Model,
    Subagent,
    Validator,
    Script,
}

impl ReviewerKind {
    pub const ALL: &'static [ReviewerKind] = &[
        ReviewerKind::Operator,
        ReviewerKind::Model,
        ReviewerKind::Subagent,
        ReviewerKind::Validator,
        ReviewerKind::Script,
    ];

    pub fn as_token(self) -> &'static str {
        match self {
            ReviewerKind::Operator => "operator",
            ReviewerKind::Model => "model",
            ReviewerKind::Subagent => "subagent",
            ReviewerKind::Validator => "validator",
            ReviewerKind::Script => "script",
        }
    }

    pub fn from_token(token: &str) -> Result<Self, PromptFeedbackError> {
        match token {
            "operator" => Ok(ReviewerKind::Operator),
            "model" => Ok(ReviewerKind::Model),
            "subagent" => Ok(ReviewerKind::Subagent),
            "validator" => Ok(ReviewerKind::Validator),
            "script" => Ok(ReviewerKind::Script),
            other => Err(PromptFeedbackError::Validation(format!(
                "unknown reviewer_kind token: {other}"
            ))),
        }
    }
}

/// The verdict a reviewer recorded (handoff data-model `verdict_kind`).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerdictKind {
    Success,
    Watch,
    Failure,
    Reject,
    Diagnostic,
}

impl VerdictKind {
    pub const ALL: &'static [VerdictKind] = &[
        VerdictKind::Success,
        VerdictKind::Watch,
        VerdictKind::Failure,
        VerdictKind::Reject,
        VerdictKind::Diagnostic,
    ];

    pub fn as_token(self) -> &'static str {
        match self {
            VerdictKind::Success => "success",
            VerdictKind::Watch => "watch",
            VerdictKind::Failure => "failure",
            VerdictKind::Reject => "reject",
            VerdictKind::Diagnostic => "diagnostic",
        }
    }

    pub fn from_token(token: &str) -> Result<Self, PromptFeedbackError> {
        match token {
            "success" => Ok(VerdictKind::Success),
            "watch" => Ok(VerdictKind::Watch),
            "failure" => Ok(VerdictKind::Failure),
            "reject" => Ok(VerdictKind::Reject),
            "diagnostic" => Ok(VerdictKind::Diagnostic),
            other => Err(PromptFeedbackError::Validation(format!(
                "unknown verdict_kind token: {other}"
            ))),
        }
    }
}

/// Input for importing/creating one prompt case (adapter output).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewPromptCase {
    pub project_id: String,
    pub source_system: String,
    pub adapter_id: String,
    pub source_iteration_id: Option<String>,
    pub source_case_id: String,
    pub source_recipe_id: Option<String>,
    pub segment: String,
    pub cell: String,
    pub framing: String,
    pub clothing_state: String,
    pub render_stack: String,
    pub identity_judgement_allowed: bool,
    pub prompt_quality_review_allowed: bool,
    pub positive_prompt: String,
    pub negative_prompt: String,
    pub micro_gate: Option<String>,
    pub expected_failure: Option<String>,
    /// Portable ArtifactStore/dataset ref for the rendered image (never a raw
    /// machine path).
    pub image_artifact_ref: Option<String>,
    /// Portable ref for the contact/sheet artifact.
    pub sheet_artifact_ref: Option<String>,
    /// Contact/scene/outfit axes the rule engine reads.
    pub axes: PromptCaseAxes,
    /// Free-form hardcore fields preserved from the source recipe (jsonb).
    pub hardcore_fields: serde_json::Value,
    pub imported_by: String,
}

/// The CUIPP axis fields the deterministic engine reads. Stored as a jsonb blob
/// and mirrored into [`EngineCase`].
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptCaseAxes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outfit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outfit_access: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_target_terms: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_stress_positive_tail: Option<String>,
}

/// A persisted prompt case.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptCase {
    pub case_id: Uuid,
    pub project_id: String,
    pub source_system: String,
    pub adapter_id: String,
    pub source_iteration_id: Option<String>,
    pub source_case_id: String,
    pub source_recipe_id: Option<String>,
    pub segment: String,
    pub cell: String,
    pub framing: String,
    pub clothing_state: String,
    pub render_stack: String,
    pub identity_judgement_allowed: bool,
    pub prompt_quality_review_allowed: bool,
    pub positive_prompt: String,
    pub negative_prompt: String,
    pub micro_gate: Option<String>,
    pub expected_failure: Option<String>,
    pub image_artifact_ref: Option<String>,
    pub sheet_artifact_ref: Option<String>,
    pub axes: PromptCaseAxes,
    pub hardcore_fields: serde_json::Value,
    pub imported_by: String,
    pub created_at_utc: DateTime<Utc>,
}

impl PromptCase {
    /// Project this persisted case into the pure-engine input shape.
    pub fn to_engine_case(&self) -> EngineCase {
        EngineCase {
            source_case_id: self.source_case_id.clone(),
            segment: self.segment.clone(),
            cell: self.cell.clone(),
            render_stack: self.render_stack.clone(),
            clothing_state: self.clothing_state.clone(),
            positive_prompt: self.positive_prompt.clone(),
            negative_prompt: self.negative_prompt.clone(),
            contact_level: self.axes.contact_level.clone(),
            outfit: self.axes.outfit.clone(),
            outfit_access: self.axes.outfit_access.clone(),
            setting_family: self.axes.setting_family.clone(),
            scene: self.axes.scene.clone(),
            body_target_terms: self.axes.body_target_terms.clone(),
            prompt_stress_positive_tail: self.axes.prompt_stress_positive_tail.clone(),
        }
    }
}

/// Input for recording one review verdict.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewReviewVerdict {
    pub case_id: Uuid,
    pub reviewer_kind: ReviewerKind,
    pub reviewer_id: String,
    pub verdict_kind: VerdictKind,
    pub failure_class: Option<String>,
    pub failure_tags: Vec<String>,
    /// When true the reviewer intends this as an identity judgement. Rejected for
    /// prompt-stress cases (they are prompt-quality/porn-readiness evidence only).
    #[serde(default)]
    pub is_identity_judgement: bool,
    pub note: Option<String>,
}

/// A persisted review verdict.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewVerdict {
    pub verdict_id: Uuid,
    pub case_id: Uuid,
    pub reviewer_kind: ReviewerKind,
    pub reviewer_id: String,
    pub verdict_kind: VerdictKind,
    pub failure_class: Option<String>,
    pub failure_tags: Vec<String>,
    pub is_identity_judgement: bool,
    pub note: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

/// One rule descriptor inside a versioned rule pack (a registry view of the
/// engine's seed rules).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewriteRuleSpec {
    pub rule_id: String,
    pub reason_code: String,
    pub action_kind: String,
    pub summary: String,
}

/// A versioned rule pack. The engine holds the deterministic logic; the pack is
/// the durable, content-hashed pointer to *which* rule set/version was used.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RulePack {
    pub rule_pack_id: String,
    pub version: i32,
    pub title: String,
    pub description: Option<String>,
    pub rules: Vec<RewriteRuleSpec>,
    pub content_hash: String,
    pub registered_by: String,
    pub created_at_utc: DateTime<Utc>,
}

/// A persisted deterministic rewrite (plan + trace) for one case against one
/// rule-pack version.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewritePlan {
    pub rewrite_id: Uuid,
    pub case_id: Uuid,
    pub source_case_id: String,
    pub rule_pack_id: String,
    pub rule_pack_version: i32,
    pub input_hash: String,
    pub output_hash: String,
    pub changed_fields: Vec<String>,
    pub rewritten_positive_prompt: String,
    pub rewritten_negative_prompt: String,
    /// Full engine outcome (rewritten case + trace) as jsonb.
    pub outcome: RewriteOutcome,
    pub planned_by: String,
    pub created_at_utc: DateTime<Utc>,
}

/// A materialized JSONL export receipt. The bytes live in the ArtifactStore
/// behind `artifact_ref`; `content_hash` lets a consumer verify them.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptExport {
    pub export_id: Uuid,
    pub rule_pack_id: String,
    pub rule_pack_version: i32,
    pub artifact_ref: String,
    pub manifest_ref: Option<String>,
    pub content_hash: String,
    pub byte_len: i64,
    pub row_count: i32,
    pub source_case_ids: Vec<String>,
    pub rewrite_ids: Vec<Uuid>,
    pub exported_by: String,
    pub created_at_utc: DateTime<Utc>,
}

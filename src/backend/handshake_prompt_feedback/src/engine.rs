//! WP-CKC-posekit-overhaul MT-020: deterministic prompt-feedback rule engine.
//!
//! PURE Rust domain layer. No database, no I/O, no clock, no randomness. The
//! whole point of this module is that the same input case + same feedback rows +
//! same rule-pack version produce a byte-stable rewrite and a machine-readable
//! `rule_trace`. Models never freeform-rewrite a prompt row here: every mutation
//! is a rule firing with a reason code and an input/output hash.
//!
//! Pipeline (handoff `deterministic-core` topic):
//!
//! ```text
//! normalize(input) -> validate(case) -> evaluate_rules(case, feedback, rule_pack) -> rewrite(case) -> trace
//! ```
//!
//! Variation, when a rule needs it, is selected from a *sorted* replacement pool
//! by a hash of `case_id + rule_pack_id + rule_id` (see [`select_from_pool`]),
//! never by random, so a re-run reproduces the same choice.

use crate::rules::{
    rule_artifact_repair, rule_contact_proof, rule_loose_clothing, rule_protected_eval, rule_wet_scene,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Canonical seed rule-pack id (the 5 first-slice rules from the handoff
/// `rule-examples` topic).
pub const SEED_RULE_PACK_ID: &str = "prompt-feedback.seed";
/// Canonical seed rule-pack version.
pub const SEED_RULE_PACK_VERSION: i32 = 1;

// Segment tokens (handoff data-model `segment`). Future values are allowed; the
// engine only special-cases these known ones and passes the rest through.
pub const SEGMENT_STANDARD: &str = "standard";
pub const SEGMENT_IDENTITY_CONTROL: &str = "identity_control_diagnostic";
pub const SEGMENT_PROMPT_STRESS: &str = "prompt_stress";

// Stable rule ids for the seed pack. These are the `rule_id` values carried in
// every trace entry and are the keys other surfaces cite.
pub const RULE_PROTECTED_EVAL: &str = "protected_eval_prompt_mutation";
pub const RULE_LOOSE_CLOTHING: &str = "loose_clothing_blocks_body_target";
pub const RULE_WET_SCENE: &str = "wet_scene_needs_visual_logic";
pub const RULE_CONTACT_PROOF: &str = "contact_claim_without_contact_proof";
pub const RULE_ARTIFACT_REPAIR: &str = "artifact_failure_is_not_prompt_stigma";

/// Ordered seed rule ids. Evaluation order is fixed and part of the determinism
/// contract: `protected_eval` runs first so a protected `standard` row can never
/// keep a prompt-stress mutation that a later content rule would elaborate on.
pub const SEED_RULE_IDS: &[&str] = &[
    RULE_PROTECTED_EVAL,
    RULE_LOOSE_CLOTHING,
    RULE_WET_SCENE,
    RULE_CONTACT_PROOF,
    RULE_ARTIFACT_REPAIR,
];

/// What a rule firing does to the pipeline (handoff: "whether it is a prompt
/// rewrite, validator warning, workflow-routing hint, or hard reject").
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// The rule mutated prompt content deterministically.
    PromptRewrite,
    /// The rule raised a warning but did not mutate content.
    ValidatorWarning,
    /// The rule routed the case to workflow repair / control / inpaint instead
    /// of mining more prompt prose.
    WorkflowRoutingHint,
    /// The rule rejected/stripped a forbidden mutation (protected runner).
    HardReject,
}

impl ActionKind {
    pub fn as_token(self) -> &'static str {
        match self {
            ActionKind::PromptRewrite => "prompt_rewrite",
            ActionKind::ValidatorWarning => "validator_warning",
            ActionKind::WorkflowRoutingHint => "workflow_routing_hint",
            ActionKind::HardReject => "hard_reject",
        }
    }
}

/// The deterministic-transform view of a prompt case. This is intentionally
/// decoupled from the persistence `PromptCase` so the engine has
/// zero database dependency and can be unit-tested with no fixtures.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EngineCase {
    pub source_case_id: String,
    pub segment: String,
    pub cell: String,
    pub render_stack: String,
    pub clothing_state: String,
    pub positive_prompt: String,
    pub negative_prompt: String,
    // CUIPP axis fields the rules read. All optional; absent = not asserted.
    pub contact_level: Option<String>,
    pub outfit: Option<String>,
    pub outfit_access: Option<String>,
    pub setting_family: Option<String>,
    pub scene: Option<String>,
    pub body_target_terms: Option<String>,
    /// A prompt-stress positive tail proposed for application. On a `standard`
    /// row this is the leakage the protected-eval rule strips.
    pub prompt_stress_positive_tail: Option<String>,
}

/// Accumulated reviewer feedback the engine reads (from persisted verdicts). The
/// engine treats this as advisory signal for which rules should fire; it never
/// lets a model directly rewrite content.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Feedback {
    pub failure_classes: Vec<String>,
    pub failure_tags: Vec<String>,
    /// True when a contact-proof failure has already recurred for this
    /// cell/render-stack, so the contact rule routes to control/inpaint instead
    /// of adding more prose.
    pub contact_proof_recurring: bool,
}

/// One machine-readable rewrite-trace entry (handoff `deterministic-core`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceEntry {
    pub rule_id: String,
    pub rule_pack_id: String,
    pub rule_pack_version: i32,
    pub matched_fields: Vec<String>,
    pub input_hash: String,
    pub output_hash: String,
    pub changed_fields: Vec<String>,
    pub reason_code: String,
    pub action_kind: ActionKind,
}

/// The deterministic outcome of running a rule pack over one case.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewriteOutcome {
    pub rule_pack_id: String,
    pub rule_pack_version: i32,
    pub input_hash: String,
    pub output_hash: String,
    pub changed_fields: Vec<String>,
    pub rewritten: EngineCase,
    pub trace: Vec<TraceEntry>,
}

impl RewriteOutcome {
    /// True when at least one rule mutated content or hard-rejected a mutation.
    pub fn changed(&self) -> bool {
        self.input_hash != self.output_hash || !self.changed_fields.is_empty()
    }
}

/// Stable content hash of an [`EngineCase`]. Uses canonical serde JSON (struct
/// field order is fixed) so the hash is byte-stable across runs and machines.
pub fn hash_case(case: &EngineCase) -> String {
    let canon = serde_json::to_string(case).unwrap_or_default();
    format!("sha256:{}", hex::encode(Sha256::digest(canon.as_bytes())))
}

/// Deterministic pool selection. Choose one entry from `pool` by a hash of
/// `case_id + rule_pack_id + rule_id`. The pool is sorted + deduped first so the
/// index is stable regardless of the caller's ordering. Returns `None` only for
/// an empty pool.
pub fn select_from_pool<'a>(
    pool: &[&'a str],
    case_id: &str,
    rule_pack_id: &str,
    rule_id: &str,
) -> Option<&'a str> {
    if pool.is_empty() {
        return None;
    }
    let mut sorted: Vec<&'a str> = pool.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut hasher = Sha256::new();
    hasher.update(case_id.as_bytes());
    hasher.update(b"|");
    hasher.update(rule_pack_id.as_bytes());
    hasher.update(b"|");
    hasher.update(rule_id.as_bytes());
    let digest = hasher.finalize();
    let mut idx_bytes = [0u8; 8];
    idx_bytes.copy_from_slice(&digest[..8]);
    let index = (u64::from_be_bytes(idx_bytes) as usize) % sorted.len();
    Some(sorted[index])
}

/// Trim every string field. Deterministic normalization only; no lossy rewrites.
fn normalize(mut case: EngineCase) -> EngineCase {
    fn trim_opt(value: &mut Option<String>) {
        if let Some(inner) = value.as_mut() {
            let trimmed = inner.trim().to_string();
            if trimmed.is_empty() {
                *value = None;
            } else {
                *inner = trimmed;
            }
        }
    }
    case.source_case_id = case.source_case_id.trim().to_string();
    case.segment = case.segment.trim().to_string();
    case.cell = case.cell.trim().to_string();
    case.render_stack = case.render_stack.trim().to_string();
    case.clothing_state = case.clothing_state.trim().to_string();
    case.positive_prompt = case.positive_prompt.trim().to_string();
    case.negative_prompt = case.negative_prompt.trim().to_string();
    trim_opt(&mut case.contact_level);
    trim_opt(&mut case.outfit);
    trim_opt(&mut case.outfit_access);
    trim_opt(&mut case.setting_family);
    trim_opt(&mut case.scene);
    trim_opt(&mut case.body_target_terms);
    trim_opt(&mut case.prompt_stress_positive_tail);
    case
}

pub(super) struct Firing {
    pub(super) rule_id: &'static str,
    pub(super) reason_code: &'static str,
    pub(super) action_kind: ActionKind,
    pub(super) matched_fields: Vec<String>,
    pub(super) changed_fields: Vec<String>,
}

/// Run the full deterministic pipeline for one case against a rule pack.
pub fn evaluate(
    input: &EngineCase,
    feedback: &Feedback,
    rule_pack_id: &str,
    rule_pack_version: i32,
) -> RewriteOutcome {
    let normalized = normalize(input.clone());
    let overall_input_hash = hash_case(&normalized);

    let mut working = normalized.clone();
    let mut trace: Vec<TraceEntry> = Vec::new();
    let mut all_changed: Vec<String> = Vec::new();

    for rule_id in SEED_RULE_IDS {
        let before_hash = hash_case(&working);
        let firing = match *rule_id {
            RULE_PROTECTED_EVAL => rule_protected_eval(&mut working),
            RULE_LOOSE_CLOTHING => rule_loose_clothing(&mut working, rule_pack_id),
            RULE_WET_SCENE => rule_wet_scene(&mut working, rule_pack_id),
            RULE_CONTACT_PROOF => rule_contact_proof(&mut working, feedback, rule_pack_id),
            RULE_ARTIFACT_REPAIR => rule_artifact_repair(&mut working, feedback, rule_pack_id),
            _ => None,
        };
        let Some(firing) = firing else {
            continue;
        };
        let after_hash = hash_case(&working);
        for field in &firing.changed_fields {
            if !all_changed.contains(field) {
                all_changed.push(field.clone());
            }
        }
        trace.push(TraceEntry {
            rule_id: firing.rule_id.to_string(),
            rule_pack_id: rule_pack_id.to_string(),
            rule_pack_version,
            matched_fields: firing.matched_fields,
            input_hash: before_hash,
            output_hash: after_hash,
            changed_fields: firing.changed_fields,
            reason_code: firing.reason_code.to_string(),
            action_kind: firing.action_kind,
        });
    }

    let overall_output_hash = hash_case(&working);
    RewriteOutcome {
        rule_pack_id: rule_pack_id.to_string(),
        rule_pack_version,
        input_hash: overall_input_hash,
        output_hash: overall_output_hash,
        changed_fields: all_changed,
        rewritten: working,
        trace,
    }
}

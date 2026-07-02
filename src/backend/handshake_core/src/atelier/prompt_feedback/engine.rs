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
/// decoupled from the persistence [`super::model::PromptCase`] so the engine has
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

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    let lower = haystack.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn opt_contains_any(value: &Option<String>, needles: &[&str]) -> bool {
    value
        .as_deref()
        .is_some_and(|inner| contains_any(inner, needles))
}

/// Append a clause to a prompt field only if it is not already present, keeping
/// the transform idempotent (running the engine twice does not double-append).
fn append_clause(field: &mut String, clause: &str) -> bool {
    if field.to_ascii_lowercase().contains(&clause.to_ascii_lowercase()) {
        return false;
    }
    if field.is_empty() {
        *field = clause.to_string();
    } else {
        field.push_str(", ");
        field.push_str(clause);
    }
    true
}

struct Firing {
    rule_id: &'static str,
    reason_code: &'static str,
    action_kind: ActionKind,
    matched_fields: Vec<String>,
    changed_fields: Vec<String>,
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

// --- Seed rules -----------------------------------------------------------

/// Rule 5 (handoff): protected standard eval mutation. A `standard` segment row
/// must not receive a prompt-stress positive tail. Strip it and hard-reject the
/// mutation so a prompt-stress verdict can never become an identity-success
/// verdict on the protected eval contract.
fn rule_protected_eval(case: &mut EngineCase) -> Option<Firing> {
    let is_standard = case.segment == SEGMENT_STANDARD;
    let has_stress_tail = case
        .prompt_stress_positive_tail
        .as_deref()
        .is_some_and(|tail| !tail.is_empty());
    if !(is_standard && has_stress_tail) {
        return None;
    }
    let tail = case.prompt_stress_positive_tail.take().unwrap_or_default();
    let mut changed_fields = vec!["prompt_stress_positive_tail".to_string()];
    // If the tail already leaked into positive_prompt, strip that suffix too.
    let lower_pos = case.positive_prompt.to_ascii_lowercase();
    let lower_tail = tail.trim().to_ascii_lowercase();
    if !lower_tail.is_empty() && lower_pos.contains(&lower_tail) {
        let rebuilt = strip_clause(&case.positive_prompt, tail.trim());
        if rebuilt != case.positive_prompt {
            case.positive_prompt = rebuilt;
            changed_fields.push("positive_prompt".to_string());
        }
    }
    Some(Firing {
        rule_id: RULE_PROTECTED_EVAL,
        reason_code: "protected_eval_prompt_mutation",
        action_kind: ActionKind::HardReject,
        matched_fields: vec!["segment".to_string(), "prompt_stress_positive_tail".to_string()],
        changed_fields,
    })
}

/// Remove a `, clause` (or bare `clause`) occurrence from a comma-joined prompt.
fn strip_clause(prompt: &str, clause: &str) -> String {
    let kept: Vec<&str> = prompt
        .split(',')
        .map(str::trim)
        .filter(|part| {
            !part.is_empty() && !part.eq_ignore_ascii_case(clause)
        })
        .collect();
    kept.join(", ")
}

const FUNCTIONAL_OUTFIT_POOL: &[&str] = &[
    "tight sweater stretched over the chest",
    "open sweater with no bra",
    "transparent wet knit clinging to the chest",
    "pulled-up sweater baring the breasts",
    "torn-seam top with underboob exposed",
    "soaked clinging fabric outlining the nipples",
];

/// Rule 1 (handoff): loose clothing blocks the body target. If the body target
/// is huge tits/breasts but the outfit is a loose garment with no access
/// mechanic, rewrite the outfit to a functional state and add access proof.
fn rule_loose_clothing(case: &mut EngineCase, rule_pack_id: &str) -> Option<Firing> {
    let targets_breasts = opt_contains_any(
        &case.body_target_terms,
        &["huge tits", "tits", "breast", "ponybreasts", "chest"],
    );
    let loose = opt_contains_any(
        &case.outfit,
        &["loose", "sweater", "hoodie", "oversized", "baggy", "bulky", "coat"],
    );
    let has_access = case
        .outfit_access
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        || opt_contains_any(
            &case.outfit,
            &["open", "transparent", "pulled", "torn", "soaked", "clinging", "tight", "underboob"],
        );
    if !(targets_breasts && loose) || has_access {
        return None;
    }
    let functional = select_from_pool(
        FUNCTIONAL_OUTFIT_POOL,
        &case.source_case_id,
        rule_pack_id,
        RULE_LOOSE_CLOTHING,
    )?;
    case.outfit = Some(functional.to_string());
    case.outfit_access = Some("outfit failure exposing the breast target".to_string());
    let mut changed_fields = vec!["outfit".to_string(), "outfit_access".to_string()];
    if append_clause(&mut case.positive_prompt, functional) {
        changed_fields.push("positive_prompt".to_string());
    }
    Some(Firing {
        rule_id: RULE_LOOSE_CLOTHING,
        reason_code: "target_blocked_by_outfit",
        action_kind: ActionKind::PromptRewrite,
        matched_fields: vec!["body_target_terms".to_string(), "outfit".to_string()],
        changed_fields,
    })
}

const WET_LOGIC_POOL: &[&str] = &[
    "soaked transparent fabric clinging to the body",
    "wet clothing turned see-through",
    "clothes half-removed and dripping",
    "wet shirt clinging and translucent",
];

/// Rule 2 (handoff): wet scene needs visual logic. If the setting is wet but the
/// clothing is fully covering with no transparent/clinging/removal reason,
/// rewrite the clothing to a wet see-through state.
fn rule_wet_scene(case: &mut EngineCase, rule_pack_id: &str) -> Option<Firing> {
    let wet_setting = opt_contains_any(
        &case.setting_family,
        &["shower", "wet", "rain", "pool", "bath", "locker"],
    ) || opt_contains_any(&case.scene, &["shower", "wet", "rain", "pool", "bath", "locker"]);
    let has_wet_logic = opt_contains_any(
        &case.outfit,
        &["transparent", "clinging", "soaked", "wet", "see-through", "removed", "open"],
    ) || opt_contains_any(
        &case.outfit_access,
        &["transparent", "clinging", "soaked", "wet", "removed"],
    ) || contains_any(
        &case.positive_prompt,
        &["transparent", "clinging", "soaked wet", "see-through"],
    );
    if !wet_setting || has_wet_logic {
        return None;
    }
    let logic = select_from_pool(
        WET_LOGIC_POOL,
        &case.source_case_id,
        rule_pack_id,
        RULE_WET_SCENE,
    )?;
    case.outfit_access = Some(logic.to_string());
    let mut changed_fields = vec!["outfit_access".to_string()];
    if append_clause(&mut case.positive_prompt, logic) {
        changed_fields.push("positive_prompt".to_string());
    }
    Some(Firing {
        rule_id: RULE_WET_SCENE,
        reason_code: "incoherent_wet_scene",
        action_kind: ActionKind::PromptRewrite,
        matched_fields: vec!["setting_family".to_string(), "clothing_state".to_string()],
        changed_fields,
    })
}

const CONTACT_PROOF_POOL: &[&str] = &[
    "cock pressed against her lips with visible contact",
    "hips braced and thrusting with body pressure",
    "riding with explicit cock-to-pussy contact",
    "mouth stretched around the shaft, wet contact visible",
    "cum and wetness evidence on the skin",
];

const CONTACT_PROOF_MARKERS: &[&str] = &[
    "bracing", "spreading", "riding", "thrusting", "mouth", "cock", "pussy", "cum",
    "wetness", "penetration", "contact", "pressed", "deepthroat", "insertion",
];

/// Rule 3 (handoff): contact claim without contact proof. If the contact level
/// asserts oral/penetration/explicit contact/payoff but the positive fields lack
/// body-contact proof, add concrete mechanics -- unless the same failure has
/// recurred for this cell/render stack, in which case route to control/inpaint.
fn rule_contact_proof(
    case: &mut EngineCase,
    feedback: &Feedback,
    rule_pack_id: &str,
) -> Option<Firing> {
    let claims_contact = opt_contains_any(
        &case.contact_level,
        &["oral", "penetration", "explicit_contact", "payoff_aftermath"],
    );
    let has_proof = contains_any(&case.positive_prompt, CONTACT_PROOF_MARKERS);
    if !claims_contact || has_proof {
        return None;
    }
    if feedback.contact_proof_recurring {
        // Recurring failure: stop mining prose, route to control/inpaint.
        return Some(Firing {
            rule_id: RULE_CONTACT_PROOF,
            reason_code: "action_claim_without_contact_proof",
            action_kind: ActionKind::WorkflowRoutingHint,
            matched_fields: vec!["contact_level".to_string(), "render_stack".to_string()],
            changed_fields: Vec::new(),
        });
    }
    let mechanics = select_from_pool(
        CONTACT_PROOF_POOL,
        &case.source_case_id,
        rule_pack_id,
        RULE_CONTACT_PROOF,
    )?;
    let mut changed_fields = Vec::new();
    if append_clause(&mut case.positive_prompt, mechanics) {
        changed_fields.push("positive_prompt".to_string());
    }
    Some(Firing {
        rule_id: RULE_CONTACT_PROOF,
        reason_code: "action_claim_without_contact_proof",
        action_kind: ActionKind::PromptRewrite,
        matched_fields: vec!["contact_level".to_string(), "positive_prompt".to_string()],
        changed_fields,
    })
}

const ARTIFACT_NEGATIVE_POOL: &[&str] = &[
    "bad hands, extra fingers",
    "smeared fingers, deformed hands",
    "broken limbs, mangled anatomy",
    "plastic skin, smeared skin",
];

const ARTIFACT_TAGS: &[&str] = &[
    "bad_hands", "smeared_fingers", "broken_limbs", "bad_genital_detail",
    "plastic_skin", "face_smear", "source_context_smear",
];

/// Rule 4 (handoff): artifact failure is not prompt stigma. When a reviewer tags
/// a technical artifact (bad hands, smear, broken anatomy...), preserve the
/// content intent and route to workflow repair / negative pack, never a
/// permanent content ban.
fn rule_artifact_repair(
    case: &mut EngineCase,
    feedback: &Feedback,
    rule_pack_id: &str,
) -> Option<Firing> {
    let has_artifact_tag = feedback
        .failure_tags
        .iter()
        .any(|tag| ARTIFACT_TAGS.contains(&tag.as_str()))
        || feedback
            .failure_classes
            .iter()
            .any(|class| class == "technical_artifact");
    if !has_artifact_tag {
        return None;
    }
    let negative = select_from_pool(
        ARTIFACT_NEGATIVE_POOL,
        &case.source_case_id,
        rule_pack_id,
        RULE_ARTIFACT_REPAIR,
    )?;
    // Preserve positive content intent; only strengthen the negative pack.
    let mut changed_fields = Vec::new();
    if append_clause(&mut case.negative_prompt, negative) {
        changed_fields.push("negative_prompt".to_string());
    }
    Some(Firing {
        rule_id: RULE_ARTIFACT_REPAIR,
        reason_code: "artifact_requires_workflow_repair",
        action_kind: ActionKind::WorkflowRoutingHint,
        matched_fields: vec!["failure_tags".to_string()],
        changed_fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standard_case() -> EngineCase {
        EngineCase {
            source_case_id: "no_detail:0_closeup:1".to_string(),
            segment: SEGMENT_STANDARD.to_string(),
            cell: "0_closeup".to_string(),
            render_stack: "no_detail".to_string(),
            clothing_state: "clothed".to_string(),
            positive_prompt: "face-readable close-up".to_string(),
            negative_prompt: "lowres".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn protected_eval_strips_prompt_stress_tail_on_standard_row() {
        let mut case = standard_case();
        case.prompt_stress_positive_tail = Some("open blouse no bra, riding".to_string());
        let outcome = evaluate(&case, &Feedback::default(), SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION);
        assert!(outcome.rewritten.prompt_stress_positive_tail.is_none());
        let firing = outcome
            .trace
            .iter()
            .find(|entry| entry.rule_id == RULE_PROTECTED_EVAL)
            .expect("protected eval must fire on standard row with a stress tail");
        assert_eq!(firing.action_kind, ActionKind::HardReject);
        assert_eq!(firing.reason_code, "protected_eval_prompt_mutation");
    }

    #[test]
    fn protected_eval_does_not_fire_on_prompt_stress_segment() {
        let mut case = standard_case();
        case.segment = SEGMENT_PROMPT_STRESS.to_string();
        case.prompt_stress_positive_tail = Some("open blouse no bra".to_string());
        let outcome = evaluate(&case, &Feedback::default(), SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION);
        assert!(!outcome
            .trace
            .iter()
            .any(|entry| entry.rule_id == RULE_PROTECTED_EVAL));
        assert!(outcome.rewritten.prompt_stress_positive_tail.is_some());
    }

    #[test]
    fn loose_clothing_rewrites_outfit_to_functional_state() {
        let mut case = standard_case();
        case.segment = SEGMENT_PROMPT_STRESS.to_string();
        case.body_target_terms = Some("huge tits".to_string());
        case.outfit = Some("baggy oversized sweater".to_string());
        let outcome = evaluate(&case, &Feedback::default(), SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION);
        let firing = outcome
            .trace
            .iter()
            .find(|entry| entry.rule_id == RULE_LOOSE_CLOTHING)
            .expect("loose clothing must fire");
        assert_eq!(firing.reason_code, "target_blocked_by_outfit");
        assert!(outcome.rewritten.outfit_access.is_some());
    }

    #[test]
    fn wet_scene_adds_visual_logic() {
        let mut case = standard_case();
        case.segment = SEGMENT_PROMPT_STRESS.to_string();
        case.setting_family = Some("shower".to_string());
        case.outfit = Some("full winter coat".to_string());
        let outcome = evaluate(&case, &Feedback::default(), SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION);
        let firing = outcome
            .trace
            .iter()
            .find(|entry| entry.rule_id == RULE_WET_SCENE)
            .expect("wet scene must fire");
        assert_eq!(firing.reason_code, "incoherent_wet_scene");
    }

    #[test]
    fn contact_claim_without_proof_adds_mechanics_then_routes_when_recurring() {
        let mut case = standard_case();
        case.segment = SEGMENT_PROMPT_STRESS.to_string();
        case.contact_level = Some("oral".to_string());
        let outcome = evaluate(&case, &Feedback::default(), SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION);
        let firing = outcome
            .trace
            .iter()
            .find(|entry| entry.rule_id == RULE_CONTACT_PROOF)
            .expect("contact rule must fire");
        assert_eq!(firing.action_kind, ActionKind::PromptRewrite);

        let recurring = Feedback {
            contact_proof_recurring: true,
            ..Default::default()
        };
        let routed = evaluate(&case, &recurring, SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION);
        let routed_firing = routed
            .trace
            .iter()
            .find(|entry| entry.rule_id == RULE_CONTACT_PROOF)
            .expect("contact rule must fire when recurring");
        assert_eq!(routed_firing.action_kind, ActionKind::WorkflowRoutingHint);
    }

    #[test]
    fn artifact_tag_routes_to_workflow_repair_without_content_ban() {
        let case = standard_case();
        let feedback = Feedback {
            failure_tags: vec!["bad_hands".to_string()],
            ..Default::default()
        };
        let outcome = evaluate(&case, &feedback, SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION);
        let firing = outcome
            .trace
            .iter()
            .find(|entry| entry.rule_id == RULE_ARTIFACT_REPAIR)
            .expect("artifact repair must fire");
        assert_eq!(firing.action_kind, ActionKind::WorkflowRoutingHint);
        // Positive content intent preserved.
        assert_eq!(outcome.rewritten.positive_prompt, case.positive_prompt);
    }

    #[test]
    fn rewrite_is_byte_identical_across_runs() {
        let mut case = standard_case();
        case.segment = SEGMENT_PROMPT_STRESS.to_string();
        case.body_target_terms = Some("huge tits".to_string());
        case.outfit = Some("loose hoodie".to_string());
        case.contact_level = Some("penetration".to_string());
        let a = evaluate(&case, &Feedback::default(), SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION);
        let b = evaluate(&case, &Feedback::default(), SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
            "same input + rule pack must yield byte-identical rewrite"
        );
        assert_eq!(a.output_hash, b.output_hash);
    }

    #[test]
    fn pool_selection_is_deterministic() {
        let pool = ["c", "a", "b"];
        let first = select_from_pool(&pool, "case-1", SEED_RULE_PACK_ID, RULE_LOOSE_CLOTHING);
        let again = select_from_pool(&pool, "case-1", SEED_RULE_PACK_ID, RULE_LOOSE_CLOTHING);
        assert_eq!(first, again);
        assert!(first.is_some());
    }
}

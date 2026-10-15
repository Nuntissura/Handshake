use super::engine::{
    select_from_pool, ActionKind, EngineCase, Feedback, Firing, RULE_ARTIFACT_REPAIR,
    RULE_CONTACT_PROOF, RULE_LOOSE_CLOTHING, RULE_PROTECTED_EVAL, RULE_WET_SCENE, SEGMENT_STANDARD,
};

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
    if field
        .to_ascii_lowercase()
        .contains(&clause.to_ascii_lowercase())
    {
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

// --- Seed rules -----------------------------------------------------------

/// Rule 5 (handoff): protected standard eval mutation. A `standard` segment row
/// must not receive a prompt-stress positive tail. Strip it and hard-reject the
/// mutation so a prompt-stress verdict can never become an identity-success
/// verdict on the protected eval contract.
pub(super) fn rule_protected_eval(case: &mut EngineCase) -> Option<Firing> {
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
        matched_fields: vec![
            "segment".to_string(),
            "prompt_stress_positive_tail".to_string(),
        ],
        changed_fields,
    })
}

/// Remove a `, clause` (or bare `clause`) occurrence from a comma-joined prompt.
fn strip_clause(prompt: &str, clause: &str) -> String {
    let kept: Vec<&str> = prompt
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty() && !part.eq_ignore_ascii_case(clause))
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
pub(super) fn rule_loose_clothing(case: &mut EngineCase, rule_pack_id: &str) -> Option<Firing> {
    let targets_breasts = opt_contains_any(
        &case.body_target_terms,
        &["huge tits", "tits", "breast", "ponybreasts", "chest"],
    );
    let loose = opt_contains_any(
        &case.outfit,
        &[
            "loose",
            "sweater",
            "hoodie",
            "oversized",
            "baggy",
            "bulky",
            "coat",
        ],
    );
    let has_access = case
        .outfit_access
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        || opt_contains_any(
            &case.outfit,
            &[
                "open",
                "transparent",
                "pulled",
                "torn",
                "soaked",
                "clinging",
                "tight",
                "underboob",
            ],
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
pub(super) fn rule_wet_scene(case: &mut EngineCase, rule_pack_id: &str) -> Option<Firing> {
    let wet_setting = opt_contains_any(
        &case.setting_family,
        &["shower", "wet", "rain", "pool", "bath", "locker"],
    ) || opt_contains_any(
        &case.scene,
        &["shower", "wet", "rain", "pool", "bath", "locker"],
    );
    let has_wet_logic = opt_contains_any(
        &case.outfit,
        &[
            "transparent",
            "clinging",
            "soaked",
            "wet",
            "see-through",
            "removed",
            "open",
        ],
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
    "bracing",
    "spreading",
    "riding",
    "thrusting",
    "mouth",
    "cock",
    "pussy",
    "cum",
    "wetness",
    "penetration",
    "contact",
    "pressed",
    "deepthroat",
    "insertion",
];

/// Rule 3 (handoff): contact claim without contact proof. If the contact level
/// asserts oral/penetration/explicit contact/payoff but the positive fields lack
/// body-contact proof, add concrete mechanics -- unless the same failure has
/// recurred for this cell/render stack, in which case route to control/inpaint.
pub(super) fn rule_contact_proof(
    case: &mut EngineCase,
    feedback: &Feedback,
    rule_pack_id: &str,
) -> Option<Firing> {
    let claims_contact = opt_contains_any(
        &case.contact_level,
        &[
            "oral",
            "penetration",
            "explicit_contact",
            "payoff_aftermath",
        ],
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
    "bad_hands",
    "smeared_fingers",
    "broken_limbs",
    "bad_genital_detail",
    "plastic_skin",
    "face_smear",
    "source_context_smear",
];

/// Rule 4 (handoff): artifact failure is not prompt stigma. When a reviewer tags
/// a technical artifact (bad hands, smear, broken anatomy...), preserve the
/// content intent and route to workflow repair / negative pack, never a
/// permanent content ban.
pub(super) fn rule_artifact_repair(
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

use handshake_prompt_feedback::*;

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
    let outcome = evaluate(
        &case,
        &Feedback::default(),
        SEED_RULE_PACK_ID,
        SEED_RULE_PACK_VERSION,
    );
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
    let outcome = evaluate(
        &case,
        &Feedback::default(),
        SEED_RULE_PACK_ID,
        SEED_RULE_PACK_VERSION,
    );
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
    let outcome = evaluate(
        &case,
        &Feedback::default(),
        SEED_RULE_PACK_ID,
        SEED_RULE_PACK_VERSION,
    );
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
    let outcome = evaluate(
        &case,
        &Feedback::default(),
        SEED_RULE_PACK_ID,
        SEED_RULE_PACK_VERSION,
    );
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
    let outcome = evaluate(
        &case,
        &Feedback::default(),
        SEED_RULE_PACK_ID,
        SEED_RULE_PACK_VERSION,
    );
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
    let a = evaluate(
        &case,
        &Feedback::default(),
        SEED_RULE_PACK_ID,
        SEED_RULE_PACK_VERSION,
    );
    let b = evaluate(
        &case,
        &Feedback::default(),
        SEED_RULE_PACK_ID,
        SEED_RULE_PACK_VERSION,
    );
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

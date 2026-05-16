use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    visual_debugging_loop::{
        project_visual_debugging_loop, validate_visual_debugging_loop, ValidatorSteeringV1,
        VisualComparisonMode, VisualDebugEvidenceArtifactV1, VisualDebuggingLoopV1,
        VisualDebuggingThresholdConfigV1, VisualDebuggingTriggerKind, VisualDebuggingTriggerV1,
    },
};

#[test]
fn kernel_visual_debugging_loop_projects_triggers_thresholds_and_evidence() {
    let loop_config = sample_loop();
    validate_visual_debugging_loop(&loop_config).expect("visual loop validates");

    let projection = project_visual_debugging_loop(&loop_config).expect("projection builds");

    assert!(projection
        .trigger_kinds
        .contains(&VisualDebuggingTriggerKind::PostCommit));
    assert!(projection
        .trigger_kinds
        .contains(&VisualDebuggingTriggerKind::PostAction));
    assert!(projection
        .visual_diff_artifact_refs
        .contains(&"artifact://visual-diffs/diff-1.png".to_string()));
    assert!(projection.validator_steer_required);
    assert!(!projection.mutates_gui_authority);
}

#[test]
fn kernel_visual_debugging_loop_projects_threshold_exceeded_steering() {
    let projection = project_visual_debugging_loop(&sample_loop()).expect("projection builds");

    assert!(projection
        .threshold_exceeded_evidence_ids
        .contains(&"visual-evidence-1".to_string()));
    assert_eq!(
        projection.threshold_config_ref,
        "packet://WP-GUI/visual-thresholds"
    );
}

#[test]
fn kernel_visual_debugging_loop_rejects_missing_baseline_or_steering() {
    let mut loop_config = sample_loop();
    loop_config
        .triggers
        .retain(|trigger| trigger.kind != VisualDebuggingTriggerKind::PostAction);
    loop_config.threshold_config.max_pixel_diff_basis_points = 0;
    loop_config.evidence_artifacts[0].baseline_ref.clear();
    loop_config.evidence_artifacts[0].stored_in_artifact_system = false;
    loop_config
        .validator_steering
        .threshold_exceeded_sends_steer = false;

    let errors = validate_visual_debugging_loop(&loop_config).expect_err("unsafe loop must fail");

    assert!(errors.iter().any(|error| error.field == "triggers.kind"));
    assert!(errors
        .iter()
        .any(|error| error.field == "threshold_config.max_pixel_diff_basis_points"));
    assert!(errors
        .iter()
        .any(|error| error.field == "evidence_artifacts.baseline_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "evidence_artifacts.stored_in_artifact_system"));
    assert!(errors
        .iter()
        .any(|error| error.field == "validator_steering.threshold_exceeded_sends_steer"));
}

#[test]
fn kernel_visual_debugging_loop_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.visual_debugging_loop.project")
        .expect("visual debugging loop projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "visual_debugging_validator_steering"));
}

fn sample_loop() -> VisualDebuggingLoopV1 {
    VisualDebuggingLoopV1 {
        schema_id: "hsk.kernel.visual_debugging_loop@1".to_string(),
        loop_id: "visual-debugging-mt046".to_string(),
        folded_stub_ids: vec!["WP-1-Visual-Debugging-Loop-v1".to_string()],
        gui_bearing_wp_id: "WP-GUI".to_string(),
        triggers: vec![
            trigger(
                "trigger.post_commit",
                VisualDebuggingTriggerKind::PostCommit,
            ),
            trigger(
                "trigger.post_action",
                VisualDebuggingTriggerKind::PostAction,
            ),
        ],
        threshold_config: VisualDebuggingThresholdConfigV1 {
            threshold_config_ref: "packet://WP-GUI/visual-thresholds".to_string(),
            max_pixel_diff_basis_points: 250,
            max_layout_shift_basis_points: 100,
            structural_mismatch_limit: 0,
        },
        evidence_artifacts: vec![VisualDebugEvidenceArtifactV1 {
            evidence_id: "visual-evidence-1".to_string(),
            wp_id: "WP-GUI".to_string(),
            commit_ref: "git://commit/abc123".to_string(),
            screenshot_ref: "artifact://screenshots/screen-1.png".to_string(),
            baseline_ref: "artifact://baselines/screen-1.png".to_string(),
            visual_diff_artifact_ref: "artifact://visual-diffs/diff-1.png".to_string(),
            comparison_mode: VisualComparisonMode::PixelDiff,
            mismatch_basis_points: 300,
            stored_in_artifact_system: true,
        }],
        validator_steering: ValidatorSteeringV1 {
            enabled: true,
            target_role: "VALIDATOR".to_string(),
            receipt_kind: "STEER".to_string(),
            code_diff_ref: "git://diff/abc123".to_string(),
            visual_diff_ref: "artifact://visual-diffs/diff-1.png".to_string(),
            visual_evidence_required: true,
            threshold_exceeded_sends_steer: true,
        },
        product_authority_refs: vec![
            "kernel.product_screenshot_capture".to_string(),
            "kernel.action_catalog".to_string(),
            "artifact_store.visual_evidence".to_string(),
            "validator.steering".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Visual-Debugging-Loop-v1.contract.json".to_string(),
        ],
    }
}

fn trigger(trigger_id: &str, kind: VisualDebuggingTriggerKind) -> VisualDebuggingTriggerV1 {
    VisualDebuggingTriggerV1 {
        trigger_id: trigger_id.to_string(),
        kind,
        screenshot_request_ref: format!("screenshot-request://{trigger_id}"),
        baseline_ref: "artifact://baselines/screen-1.png".to_string(),
        capture_after_ref: format!("event://{trigger_id}/after"),
    }
}

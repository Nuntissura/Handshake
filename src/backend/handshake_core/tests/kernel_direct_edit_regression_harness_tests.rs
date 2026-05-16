use handshake_core::kernel::{
    action_catalog::kernel002_action_catalog,
    direct_edit_guard::{
        guard_direct_edit_attempt, run_direct_edit_regression_harness, DirectEditAttemptV1,
        DirectEditDecisionStatus, DirectEditRegressionCaseV1, DirectEditRegressionHarnessResultV1,
        DirectEditTargetClass,
    },
};

#[test]
fn kernel_direct_edit_regression_harness_covers_common_bypass_paths() {
    let catalog = kernel002_action_catalog();
    let result =
        run_direct_edit_regression_harness("direct-edit-mt048", &regression_cases(), &catalog);

    assert_eq!(
        result.schema_id,
        "hsk.kernel.direct_edit_regression_harness_result@1"
    );
    assert_eq!(result.case_results.len(), 7);
    assert!(result.all_paths_guarded);
    assert!(result.unguarded_case_ids.is_empty());
    assert_case(
        &result,
        "raw-model-patch",
        DirectEditDecisionStatus::Denied,
        Some("kernel.direct_edit.deny"),
    );
    assert_case(
        &result,
        "generated-file-write",
        DirectEditDecisionStatus::Wrapped,
        Some("kernel.mirror_advisory.capture"),
    );
    assert_case(
        &result,
        "mirror-edit",
        DirectEditDecisionStatus::Wrapped,
        Some("kernel.mirror_advisory.capture"),
    );
    assert_case(
        &result,
        "crdt-edit",
        DirectEditDecisionStatus::Wrapped,
        Some("kernel.crdt_workspace.propose_patch"),
    );
    assert_case(
        &result,
        "mailbox-reply",
        DirectEditDecisionStatus::Wrapped,
        Some("kernel.role_mailbox_loop_control.project"),
    );
    assert_case(
        &result,
        "dcc-quick-action",
        DirectEditDecisionStatus::Wrapped,
        Some("kernel.visual_debugging_loop.project"),
    );
    assert_case(
        &result,
        "git-action",
        DirectEditDecisionStatus::Wrapped,
        Some("kernel.git_engine_decision_gate.project"),
    );
}

#[test]
fn kernel_direct_edit_regression_harness_denies_unregistered_dcc_quick_action() {
    let catalog = kernel002_action_catalog();
    let decision = guard_direct_edit_attempt(
        &attempt(
            "bad-dcc-action",
            DirectEditTargetClass::DccQuickAction,
            "dcc:run_unregistered_action",
        ),
        &catalog,
    );

    assert_eq!(decision.status, DirectEditDecisionStatus::Denied);
    assert_eq!(
        decision
            .denial
            .as_ref()
            .map(|denial| denial.denial_code.as_str()),
        Some("dcc_quick_action_unregistered")
    );
    assert!(!decision.evidence_refs.is_empty());
}

fn regression_cases() -> Vec<DirectEditRegressionCaseV1> {
    vec![
        DirectEditRegressionCaseV1 {
            case_id: "raw-model-patch".to_string(),
            attempt: attempt(
                "raw-model-patch",
                DirectEditTargetClass::AuthorityArtifact,
                "raw_patch",
            ),
        },
        DirectEditRegressionCaseV1 {
            case_id: "generated-file-write".to_string(),
            attempt: attempt(
                "generated-file-write",
                DirectEditTargetClass::GeneratedFile,
                "generated_file_write",
            ),
        },
        DirectEditRegressionCaseV1 {
            case_id: "mirror-edit".to_string(),
            attempt: attempt(
                "mirror-edit",
                DirectEditTargetClass::GeneratedMirror,
                "mirror_edit",
            ),
        },
        DirectEditRegressionCaseV1 {
            case_id: "crdt-edit".to_string(),
            attempt: attempt(
                "crdt-edit",
                DirectEditTargetClass::CrdtWorkspace,
                "crdt_edit",
            ),
        },
        DirectEditRegressionCaseV1 {
            case_id: "mailbox-reply".to_string(),
            attempt: attempt(
                "mailbox-reply",
                DirectEditTargetClass::RoleMailboxReply,
                "mailbox_reply",
            ),
        },
        DirectEditRegressionCaseV1 {
            case_id: "dcc-quick-action".to_string(),
            attempt: attempt(
                "dcc-quick-action",
                DirectEditTargetClass::DccQuickAction,
                "registered_action:kernel.visual_debugging_loop.project",
            ),
        },
        DirectEditRegressionCaseV1 {
            case_id: "git-action".to_string(),
            attempt: attempt("git-action", DirectEditTargetClass::GitAction, "git:commit"),
        },
    ]
}

fn attempt(
    attempt_id: &str,
    target_class: DirectEditTargetClass,
    operation: &str,
) -> DirectEditAttemptV1 {
    DirectEditAttemptV1 {
        attempt_id: attempt_id.to_string(),
        actor_id: "actor-model-1".to_string(),
        actor_kind: "model".to_string(),
        role_id: "CODER".to_string(),
        target_path: ".GOV/task_packets/WP-KERNEL-002/packet.json".to_string(),
        target_class,
        operation: operation.to_string(),
        trace_id: format!("trace-{attempt_id}"),
    }
}

fn assert_case(
    result: &DirectEditRegressionHarnessResultV1,
    case_id: &str,
    status: DirectEditDecisionStatus,
    lawful_action_id: Option<&str>,
) {
    let case = result
        .case_results
        .iter()
        .find(|case| case.case_id == case_id)
        .expect("case result exists");
    assert_eq!(case.status, status);
    assert_eq!(case.lawful_action_id.as_deref(), lawful_action_id);
    assert!(!case.evidence_refs.is_empty());
}

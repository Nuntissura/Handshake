use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    role_mailbox_loop_control::{
        latest_micro_task_loop_checkpoint, preview_micro_task_loop_action,
        project_micro_task_loop_summary, validate_role_mailbox_micro_task_loop_control,
        CompletionReportTranscriptionPosture, LoopActionKind, MicroTaskLoopCheckpointV1,
        MicroTaskLoopControlV1, MicroTaskLoopDeadLetterPosture, MicroTaskLoopState,
        MicroTaskMailboxPayloadKind, MicroTaskRetryBudgetV1, MicroTaskVerifierOutcomeKind,
        MicroTaskVerifierOutcomeV1,
    },
};

#[test]
fn micro_task_loop_control_validates_compact_checkpoint_and_verifier_outcome() {
    let control = sample_control();

    validate_role_mailbox_micro_task_loop_control(&control).expect("loop control validates");
    let latest =
        latest_micro_task_loop_checkpoint(&control, "MT-028").expect("latest checkpoint exists");

    assert_eq!(latest.sequence, 3);
    assert_eq!(latest.loop_state, MicroTaskLoopState::Escalated);
    assert_eq!(latest.retry_budget.remaining_attempts, 0);
    assert_eq!(
        latest.completion_transcription_posture,
        CompletionReportTranscriptionPosture::Pending
    );
    assert!(!control.transcript_replay_required);
    assert!(control.compact_summary_first);
}

#[test]
fn loop_summary_projects_retry_escalation_completion_and_dead_letter_without_authority() {
    let control = sample_control();
    let summary = project_micro_task_loop_summary(&control, "MT-028").expect("summary projects");

    assert_eq!(summary.remaining_retries, 0);
    assert_eq!(
        summary.verifier_outcome_kind,
        Some(MicroTaskVerifierOutcomeKind::RequiresEscalation)
    );
    assert_eq!(summary.escalation_target.as_deref(), Some("VALIDATOR"));
    assert_eq!(
        summary.completion_transcription_posture,
        CompletionReportTranscriptionPosture::Pending
    );
    assert_eq!(
        summary.dead_letter_posture,
        MicroTaskLoopDeadLetterPosture::RequiresRemediation
    );
    assert!(!summary.mutates_authority);
    assert!(summary.replayable_from_checkpoint);
}

#[test]
fn loop_action_preview_exposes_governed_transition_before_quick_action() {
    let control = sample_control();

    let preview = preview_micro_task_loop_action(&control, "MT-028", LoopActionKind::Escalate)
        .expect("escalation preview exists");

    assert_eq!(preview.action_kind, LoopActionKind::Escalate);
    assert_eq!(
        preview.kernel_action_id.as_deref(),
        Some("kernel.workflow_transition.preview")
    );
    assert_eq!(
        preview.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert!(preview.preview_required);
    assert!(!preview.mutates_authority);
}

#[test]
fn loop_control_rejects_transcript_replay_projection_authority_and_invalid_budget() {
    let mut control = sample_control();
    control.transcript_replay_required = true;
    control.work_packet_projection_authoritative = true;
    control.task_board_projection_authoritative = true;
    control.checkpoints[0].retry_budget.remaining_attempts = 4;
    control.checkpoints[2].latest_verifier_outcome_id = None;

    let errors = validate_role_mailbox_micro_task_loop_control(&control)
        .expect_err("unsafe loop control must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "transcript_replay_required"));
    assert!(errors
        .iter()
        .any(|error| error.field == "work_packet_projection_authoritative"));
    assert!(errors
        .iter()
        .any(|error| error.field == "task_board_projection_authoritative"));
    assert!(errors
        .iter()
        .any(|error| error.field == "retry_budget.remaining_attempts"));
    assert!(errors
        .iter()
        .any(|error| error.field == "latest_verifier_outcome_id"));
}

#[test]
fn kernel_action_catalog_exposes_role_mailbox_loop_control_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.role_mailbox_loop_control.project")
        .expect("Role Mailbox loop control projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "micro_task_loop_checkpoint"));
}

fn sample_control() -> MicroTaskLoopControlV1 {
    MicroTaskLoopControlV1 {
        schema_id: "hsk.kernel.role_mailbox_micro_task_loop_control@1".to_string(),
        control_id: "kernel002-role-mailbox-loop-control-mt028".to_string(),
        folded_stub_id: "WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1".to_string(),
        checkpoints: vec![
            checkpoint(
                "checkpoint-1",
                1,
                MicroTaskLoopState::VerificationNeeded,
                3,
                1,
                Some("verifier-outcome-1"),
                None,
                None,
                CompletionReportTranscriptionPosture::NotRequired,
                MicroTaskLoopDeadLetterPosture::None,
            ),
            checkpoint(
                "checkpoint-2",
                2,
                MicroTaskLoopState::Retrying,
                3,
                2,
                Some("verifier-outcome-2"),
                None,
                None,
                CompletionReportTranscriptionPosture::NotRequired,
                MicroTaskLoopDeadLetterPosture::None,
            ),
            checkpoint(
                "checkpoint-3",
                3,
                MicroTaskLoopState::Escalated,
                3,
                3,
                Some("verifier-outcome-3"),
                Some("VALIDATOR"),
                Some("completion-report-1"),
                CompletionReportTranscriptionPosture::Pending,
                MicroTaskLoopDeadLetterPosture::RequiresRemediation,
            ),
        ],
        verifier_outcomes: vec![
            outcome(
                "verifier-outcome-1",
                MicroTaskVerifierOutcomeKind::NeedsEvidence,
                "Add validation evidence before retry.",
            ),
            outcome(
                "verifier-outcome-2",
                MicroTaskVerifierOutcomeKind::FailedRetryable,
                "Retry allowed after fix.",
            ),
            outcome(
                "verifier-outcome-3",
                MicroTaskVerifierOutcomeKind::RequiresEscalation,
                "Retry budget exhausted; escalate.",
            ),
        ],
        compact_summary_first: true,
        transcript_replay_required: false,
        work_packet_projection_authoritative: false,
        task_board_projection_authoritative: false,
        local_small_model_fields: vec![
            "compact_summary".to_string(),
            "retry_budget".to_string(),
            "verifier_outcome".to_string(),
            "escalation_target".to_string(),
            "completion_transcription_posture".to_string(),
            "dead_letter_posture".to_string(),
        ],
        product_authority_refs: vec![
            "kernel.role_mailbox_contract".to_string(),
            "kernel.locus_work_tracking".to_string(),
            "kernel.workflow_transition_registry".to_string(),
            "kernel.dcc_layout_projection_registry".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1.contract.json"
                .to_string(),
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1.md".to_string(),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn checkpoint(
    checkpoint_id: &str,
    sequence: u64,
    loop_state: MicroTaskLoopState,
    max_attempts: u32,
    attempts_used: u32,
    latest_verifier_outcome_id: Option<&str>,
    escalation_target: Option<&str>,
    completion_report_id: Option<&str>,
    completion_transcription_posture: CompletionReportTranscriptionPosture,
    dead_letter_posture: MicroTaskLoopDeadLetterPosture,
) -> MicroTaskLoopCheckpointV1 {
    MicroTaskLoopCheckpointV1 {
        checkpoint_id: checkpoint_id.to_string(),
        sequence,
        wp_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
        mt_id: "MT-028".to_string(),
        role_mailbox_thread_id: "role-mailbox-thread-mt028".to_string(),
        loop_state,
        retry_budget: MicroTaskRetryBudgetV1 {
            max_attempts,
            attempts_used,
            remaining_attempts: max_attempts - attempts_used,
        },
        latest_verifier_outcome_id: latest_verifier_outcome_id.map(str::to_string),
        escalation_target: escalation_target.map(str::to_string),
        completion_report_id: completion_report_id.map(str::to_string),
        completion_transcription_posture,
        dead_letter_posture,
        compact_summary: "Bounded checkpoint summary for a local small model.".to_string(),
        replay_ref: format!("checkpoint://{checkpoint_id}"),
        message_payload_kinds: vec![
            MicroTaskMailboxPayloadKind::MicroTaskFeedback,
            MicroTaskMailboxPayloadKind::MicroTaskVerificationNeeded,
            MicroTaskMailboxPayloadKind::MicroTaskEscalation,
            MicroTaskMailboxPayloadKind::MicroTaskCompletionReport,
        ],
    }
}

fn outcome(
    outcome_id: &str,
    kind: MicroTaskVerifierOutcomeKind,
    summary: &str,
) -> MicroTaskVerifierOutcomeV1 {
    MicroTaskVerifierOutcomeV1 {
        outcome_id: outcome_id.to_string(),
        kind,
        verifier_session_id: "validator-session-1".to_string(),
        summary: summary.to_string(),
        evidence_refs: vec!["evidence://mt028/verifier".to_string()],
        next_action: "kernel.workflow_transition.preview".to_string(),
    }
}

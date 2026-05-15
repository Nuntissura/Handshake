use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    role_mailbox_contract::RoleMailboxAuthorityBoundary,
    role_mailbox_triage_queue::{
        preview_role_mailbox_remediation_action, project_task_board_pressure_overlays,
        validate_role_mailbox_triage_queue_controls, DeadLetterDisposition, ExpiryPosture,
        PressureLevel, RemediationActionKind, RemediationActionV1, ReminderScheduleV1,
        RoleMailboxTriageQueueControlsV1, RoleMailboxTriageQueueItemV1, TaskBoardPressureOverlayV1,
        TriageQueueState,
    },
};

#[test]
fn triage_queue_controls_validate_field_backed_reminder_snooze_expiry_and_dead_letter_state() {
    let controls = sample_controls();

    validate_role_mailbox_triage_queue_controls(&controls).expect("triage controls validate");

    let item = &controls.queue_items[0];
    assert_eq!(item.queue_state, TriageQueueState::DeadLetterRemediation);
    assert_eq!(item.expiry_posture, ExpiryPosture::Expired);
    assert_eq!(item.dead_letter_disposition, DeadLetterDisposition::Reroute);
    assert!(item.field_backed_projection);
    assert!(!item.transcript_parsing_required);
}

#[test]
fn triage_queue_projects_task_board_pressure_without_authority_transfer() {
    let controls = sample_controls();
    let overlays = project_task_board_pressure_overlays(&controls).expect("overlays project");

    assert_eq!(overlays.len(), 1);
    assert_eq!(overlays[0].pressure_level, PressureLevel::Critical);
    assert!(overlays[0].projection_only);
    assert!(!overlays[0].mutates_task_board);
}

#[test]
fn triage_queue_action_preview_distinguishes_local_archive_from_governed_followup() {
    let controls = sample_controls();

    let archive = preview_role_mailbox_remediation_action(
        &controls,
        "role-mailbox-thread-mt029",
        RemediationActionKind::Archive,
    )
    .expect("archive preview exists");
    assert_eq!(archive.boundary, RoleMailboxAuthorityBoundary::MailboxLocal);
    assert_eq!(archive.authority_effect, AuthorityEffect::None);
    assert_eq!(archive.kernel_action_id, None);

    let followup = preview_role_mailbox_remediation_action(
        &controls,
        "role-mailbox-thread-mt029",
        RemediationActionKind::GovernedFollowUp,
    )
    .expect("governed follow-up preview exists");
    assert_eq!(
        followup.boundary,
        RoleMailboxAuthorityBoundary::GovernedActionRequired
    );
    assert_eq!(
        followup.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        followup.kernel_action_id.as_deref(),
        Some("kernel.workflow_transition.preview")
    );
}

#[test]
fn triage_queue_rejects_unfielded_transcript_or_authority_leaking_projections() {
    let mut controls = sample_controls();
    controls.task_board_pressure_authoritative = true;
    controls.work_packet_followup_authoritative = true;
    controls.queue_items[0].field_backed_projection = false;
    controls.queue_items[0].transcript_parsing_required = true;
    controls.queue_items[0].mailbox_state_authoritative_for_linked_work = true;
    controls.queue_items[0].reminder_schedule.cadence_minutes = 0;

    let errors = validate_role_mailbox_triage_queue_controls(&controls)
        .expect_err("unsafe triage controls must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "task_board_pressure_authoritative"));
    assert!(errors
        .iter()
        .any(|error| error.field == "work_packet_followup_authoritative"));
    assert!(errors
        .iter()
        .any(|error| error.field == "field_backed_projection"));
    assert!(errors
        .iter()
        .any(|error| error.field == "transcript_parsing_required"));
    assert!(errors
        .iter()
        .any(|error| error.field == "reminder_schedule.cadence_minutes"));
}

#[test]
fn kernel_action_catalog_exposes_role_mailbox_triage_queue_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.role_mailbox_triage_queue.project")
        .expect("Role Mailbox triage queue projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "mailbox_triage_queue_state"));
}

fn sample_controls() -> RoleMailboxTriageQueueControlsV1 {
    RoleMailboxTriageQueueControlsV1 {
        schema_id: "hsk.kernel.role_mailbox_triage_queue_controls@1".to_string(),
        controls_id: "kernel002-role-mailbox-triage-queue-mt029".to_string(),
        folded_stub_id: "WP-1-Role-Mailbox-Triage-Queue-Controls-v1".to_string(),
        queue_items: vec![RoleMailboxTriageQueueItemV1 {
            thread_id: "role-mailbox-thread-mt029".to_string(),
            linked_record_ids: vec!["MT-029".to_string()],
            queue_state: TriageQueueState::DeadLetterRemediation,
            reminder_schedule: ReminderScheduleV1 {
                schedule_id: "reminder-mt029".to_string(),
                cadence_minutes: 60,
                next_reminder_at: "2026-05-14T18:00:00Z".to_string(),
                expires_at: "2026-05-14T19:00:00Z".to_string(),
                snoozed_until: Some("2026-05-14T17:45:00Z".to_string()),
                enabled: true,
            },
            snooze_posture:
                handshake_core::kernel::role_mailbox_triage_queue::SnoozePosture::Snoozed,
            expiry_posture: ExpiryPosture::Expired,
            dead_letter_disposition: DeadLetterDisposition::Reroute,
            remediation_actions: sample_actions(),
            task_board_pressure: TaskBoardPressureOverlayV1 {
                overlay_id: "pressure-mt029".to_string(),
                linked_task_board_row_id: "task-board-row-mt029".to_string(),
                pressure_level: PressureLevel::Critical,
                waiting_reason: "dead-letter remediation required".to_string(),
                projection_only: true,
                mutates_task_board: false,
                source_field_refs: vec![
                    "queue_state".to_string(),
                    "dead_letter_disposition".to_string(),
                    "expiry_posture".to_string(),
                ],
            },
            work_packet_followup_summary: "Dead-lettered mailbox follow-up requires reroute."
                .to_string(),
            locus_join_refs: vec!["locus://MT-029".to_string()],
            compact_summary: "Critical mailbox pressure for MT-029.".to_string(),
            field_backed_projection: true,
            transcript_parsing_required: false,
            mailbox_state_authoritative_for_linked_work: false,
        }],
        compact_summary_first: true,
        task_board_pressure_authoritative: false,
        work_packet_followup_authoritative: false,
        product_authority_refs: vec![
            "kernel.role_mailbox_contract".to_string(),
            "kernel.role_mailbox_loop_control".to_string(),
            "kernel.locus_work_tracking".to_string(),
            "kernel.workflow_transition_registry".to_string(),
            "kernel.dcc_layout_projection_registry".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Triage-Queue-Controls-v1.contract.json"
                .to_string(),
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Triage-Queue-Controls-v1.md".to_string(),
        ],
    }
}

fn sample_actions() -> Vec<RemediationActionV1> {
    vec![
        action(
            RemediationActionKind::Reminder,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            None,
        ),
        action(
            RemediationActionKind::Unsnooze,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            None,
        ),
        action(
            RemediationActionKind::RetryDelivery,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            None,
        ),
        action(
            RemediationActionKind::Reroute,
            RoleMailboxAuthorityBoundary::GovernedActionRequired,
            Some("kernel.workflow_transition.preview"),
        ),
        action(
            RemediationActionKind::Archive,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            None,
        ),
        action(
            RemediationActionKind::RequestTranscription,
            RoleMailboxAuthorityBoundary::TranscriptionRequired,
            None,
        ),
        action(
            RemediationActionKind::GovernedFollowUp,
            RoleMailboxAuthorityBoundary::GovernedActionRequired,
            Some("kernel.workflow_transition.preview"),
        ),
    ]
}

fn action(
    kind: RemediationActionKind,
    boundary: RoleMailboxAuthorityBoundary,
    kernel_action_id: Option<&str>,
) -> RemediationActionV1 {
    RemediationActionV1 {
        action_id: format!("action-{kind:?}").to_lowercase(),
        kind,
        boundary,
        preview_required: true,
        kernel_action_id: kernel_action_id.map(str::to_string),
        field_paths: vec![
            "queue_state".to_string(),
            "dead_letter_disposition".to_string(),
        ],
    }
}

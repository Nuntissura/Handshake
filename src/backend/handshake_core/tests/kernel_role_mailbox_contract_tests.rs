use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::{ApprovalPosture, AuthorityEffect},
    role_mailbox_contract::{
        preview_role_mailbox_response, project_role_mailbox_triage,
        validate_role_mailbox_thread_contract, DeadLetterPosture, DuePosture, LinkedRecordRefV1,
        RoleMailboxActionRequestV1, RoleMailboxAllowedResponseKind, RoleMailboxAllowedResponseV1,
        RoleMailboxAuthorityBoundary, RoleMailboxDeliveryState, RoleMailboxMessageFamily,
        RoleMailboxThreadContractV1, RoleMailboxThreadLifecycleState,
    },
};

#[test]
fn mailbox_thread_contract_validates_lifecycle_delivery_due_and_dead_letter_state() {
    let thread = sample_thread();

    validate_role_mailbox_thread_contract(&thread).expect("mailbox contract validates");
    assert_eq!(
        thread.lifecycle_state,
        RoleMailboxThreadLifecycleState::AwaitingResponse
    );
    assert_eq!(
        thread.latest_delivery_state,
        RoleMailboxDeliveryState::DeadLettered
    );
    assert_eq!(thread.due_posture, DuePosture::Expired);
    assert_eq!(
        thread.dead_letter_posture,
        DeadLetterPosture::RequiresRemediation
    );
}

#[test]
fn mailbox_triage_projection_distinguishes_local_governed_and_transcription_actions() {
    let thread = sample_thread();
    let triage = project_role_mailbox_triage(&thread).expect("triage projects");

    assert!(!triage.mutates_linked_authority);
    assert!(triage
        .mailbox_local_actions
        .contains(&RoleMailboxAllowedResponseKind::Acknowledge));
    assert!(triage
        .governed_actions
        .contains(&RoleMailboxAllowedResponseKind::Resolve));
    assert!(triage
        .transcription_actions
        .contains(&RoleMailboxAllowedResponseKind::RequestTranscription));
    assert!(triage.dead_letter_visible);
    assert_eq!(triage.linked_record_ids, vec!["MT-027"]);
}

#[test]
fn mailbox_response_preview_keeps_local_replies_from_mutating_linked_authority() {
    let thread = sample_thread();

    let snooze = preview_role_mailbox_response(
        &thread,
        RoleMailboxAllowedResponseKind::Snooze,
        "action-snooze",
    )
    .expect("snooze preview exists");
    assert_eq!(snooze.boundary, RoleMailboxAuthorityBoundary::MailboxLocal);
    assert_eq!(snooze.authority_effect, AuthorityEffect::None);
    assert!(!snooze.mutates_linked_authority);
    assert_eq!(snooze.kernel_action_id, None);

    let resolve = preview_role_mailbox_response(
        &thread,
        RoleMailboxAllowedResponseKind::Resolve,
        "action-resolve",
    )
    .expect("resolve preview exists");
    assert_eq!(
        resolve.boundary,
        RoleMailboxAuthorityBoundary::GovernedActionRequired
    );
    assert_eq!(
        resolve.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        resolve.kernel_action_id.as_deref(),
        Some("kernel.workflow_transition.preview")
    );
}

#[test]
fn mailbox_contract_requires_typed_microtask_message_families() {
    let thread = sample_thread();

    for family in [
        RoleMailboxMessageFamily::Request,
        RoleMailboxMessageFamily::Feedback,
        RoleMailboxMessageFamily::Verification,
        RoleMailboxMessageFamily::Escalation,
        RoleMailboxMessageFamily::CompletionReport,
    ] {
        assert!(
            thread.message_families.contains(&family),
            "missing message family: {family:?}"
        );
    }
}

#[test]
fn mailbox_contract_rejects_transcript_authority_and_optional_action_metadata() {
    let mut thread = sample_thread();
    thread.mailbox_local_actions_mutate_linked_authority = true;
    thread.transcript_order_authority_allowed = true;
    thread.action_requests[3].kernel_action_id = None;
    thread.action_requests[6].transcription_target_required = false;

    let errors = validate_role_mailbox_thread_contract(&thread)
        .expect_err("unsafe mailbox contract must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "mailbox_local_actions_mutate_linked_authority"));
    assert!(errors
        .iter()
        .any(|error| error.field == "transcript_order_authority_allowed"));
    assert!(errors
        .iter()
        .any(|error| error.field == "action_requests.kernel_action_id"));
    assert!(errors
        .iter()
        .any(|error| error.field == "action_requests.transcription_target_required"));
}

#[test]
fn kernel_action_catalog_exposes_role_mailbox_contract_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.role_mailbox_contract.project")
        .expect("Role Mailbox contract projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "mailbox_authority_boundary"));
}

fn sample_thread() -> RoleMailboxThreadContractV1 {
    RoleMailboxThreadContractV1 {
        schema_id: "hsk.kernel.role_mailbox_thread_contract@1".to_string(),
        contract_id: "kernel002-role-mailbox-contract-mt027".to_string(),
        folded_stub_id: "WP-1-Role-Mailbox-Message-Thread-Contract-v1".to_string(),
        thread_id: "role-mailbox-thread-mt027".to_string(),
        lifecycle_state: RoleMailboxThreadLifecycleState::AwaitingResponse,
        latest_delivery_state: RoleMailboxDeliveryState::DeadLettered,
        due_posture: DuePosture::Expired,
        dead_letter_posture: DeadLetterPosture::RequiresRemediation,
        linked_records: vec![LinkedRecordRefV1 {
            record_id: "MT-027".to_string(),
            record_kind: "micro_task".to_string(),
            authority_ref: "kernel.locus_work_tracking".to_string(),
        }],
        allowed_responses: vec![
            response(
                RoleMailboxAllowedResponseKind::Acknowledge,
                RoleMailboxAuthorityBoundary::MailboxLocal,
            ),
            response(
                RoleMailboxAllowedResponseKind::Snooze,
                RoleMailboxAuthorityBoundary::MailboxLocal,
            ),
            response(
                RoleMailboxAllowedResponseKind::Reply,
                RoleMailboxAuthorityBoundary::MailboxLocal,
            ),
            response(
                RoleMailboxAllowedResponseKind::Escalate,
                RoleMailboxAuthorityBoundary::GovernedActionRequired,
            ),
            response(
                RoleMailboxAllowedResponseKind::Delegate,
                RoleMailboxAuthorityBoundary::GovernedActionRequired,
            ),
            response(
                RoleMailboxAllowedResponseKind::Resolve,
                RoleMailboxAuthorityBoundary::GovernedActionRequired,
            ),
            response(
                RoleMailboxAllowedResponseKind::RequestTranscription,
                RoleMailboxAuthorityBoundary::TranscriptionRequired,
            ),
        ],
        action_requests: vec![
            action(
                "action-ack",
                RoleMailboxAllowedResponseKind::Acknowledge,
                RoleMailboxAuthorityBoundary::MailboxLocal,
                None,
                false,
            ),
            action(
                "action-snooze",
                RoleMailboxAllowedResponseKind::Snooze,
                RoleMailboxAuthorityBoundary::MailboxLocal,
                None,
                false,
            ),
            action(
                "action-reply",
                RoleMailboxAllowedResponseKind::Reply,
                RoleMailboxAuthorityBoundary::MailboxLocal,
                None,
                false,
            ),
            action(
                "action-escalate",
                RoleMailboxAllowedResponseKind::Escalate,
                RoleMailboxAuthorityBoundary::GovernedActionRequired,
                Some("kernel.workflow_transition.preview"),
                false,
            ),
            action(
                "action-delegate",
                RoleMailboxAllowedResponseKind::Delegate,
                RoleMailboxAuthorityBoundary::GovernedActionRequired,
                Some("kernel.workflow_transition.preview"),
                false,
            ),
            action(
                "action-resolve",
                RoleMailboxAllowedResponseKind::Resolve,
                RoleMailboxAuthorityBoundary::GovernedActionRequired,
                Some("kernel.workflow_transition.preview"),
                false,
            ),
            action(
                "action-transcribe",
                RoleMailboxAllowedResponseKind::RequestTranscription,
                RoleMailboxAuthorityBoundary::TranscriptionRequired,
                None,
                true,
            ),
        ],
        message_families: vec![
            RoleMailboxMessageFamily::Request,
            RoleMailboxMessageFamily::Feedback,
            RoleMailboxMessageFamily::Verification,
            RoleMailboxMessageFamily::Escalation,
            RoleMailboxMessageFamily::CompletionReport,
        ],
        dcc_triage_fields: vec![
            "thread_lifecycle_state".to_string(),
            "message_delivery_state".to_string(),
            "allowed_responses".to_string(),
            "due_posture".to_string(),
            "dead_letter_posture".to_string(),
            "action_request_boundary".to_string(),
        ],
        mailbox_local_actions_mutate_linked_authority: false,
        transcript_order_authority_allowed: false,
        product_authority_refs: vec![
            "kernel.action_catalog".to_string(),
            "kernel.workflow_transition_registry".to_string(),
            "kernel.locus_work_tracking".to_string(),
            "kernel.dcc_layout_projection_registry".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Message-Thread-Contract-v1.contract.json"
                .to_string(),
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Message-Thread-Contract-v1.md".to_string(),
        ],
    }
}

fn response(
    kind: RoleMailboxAllowedResponseKind,
    boundary: RoleMailboxAuthorityBoundary,
) -> RoleMailboxAllowedResponseV1 {
    RoleMailboxAllowedResponseV1 {
        response_kind: kind,
        boundary,
        requires_action_request: true,
        display_order: kind as u32,
    }
}

fn action(
    request_id: &str,
    response_kind: RoleMailboxAllowedResponseKind,
    boundary: RoleMailboxAuthorityBoundary,
    kernel_action_id: Option<&str>,
    transcription_target_required: bool,
) -> RoleMailboxActionRequestV1 {
    RoleMailboxActionRequestV1 {
        request_id: request_id.to_string(),
        response_kind,
        boundary,
        target_record_ids: vec!["MT-027".to_string()],
        kernel_action_id: kernel_action_id.map(str::to_string),
        approval_posture: ApprovalPosture::NoApprovalRequired,
        evidence_refs: vec!["receipt://mailbox/mt027".to_string()],
        transcription_target_required,
    }
}

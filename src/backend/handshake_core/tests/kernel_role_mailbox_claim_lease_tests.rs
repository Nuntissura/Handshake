use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    role_mailbox_claim_lease::{
        preview_role_mailbox_claim_lease_action, project_role_mailbox_claim_lease,
        validate_role_mailbox_claim_lease_controls, ClaimLeaseActionKind, ClaimLeaseActionRoute,
        ClaimLeaseActionRuleV1, ClaimLeaseState, ClaimantIdentityV1,
        RoleMailboxClaimLeaseControlsV1, RoleMailboxClaimLeaseV1, RoleMailboxClaimMode,
        RoleMailboxExecutorKind, RoleMailboxResponseAuthorityScope, RoleMailboxResponseAuthorityV1,
        RoleMailboxTakeoverLegality, RoleMailboxTakeoverPolicy,
    },
    role_mailbox_contract::{RoleMailboxAllowedResponseKind, RoleMailboxAuthorityBoundary},
};

#[test]
fn claim_lease_controls_validate_claimant_mode_expiry_takeover_and_responder_eligibility() {
    let controls = sample_controls();

    validate_role_mailbox_claim_lease_controls(&controls).expect("claim lease controls validate");

    let lease = &controls.leases[0];
    assert_eq!(lease.claim_mode, RoleMailboxClaimMode::ExclusiveLease);
    assert_eq!(lease.lease_state, ClaimLeaseState::Expired);
    assert_eq!(lease.lease_age_seconds, 3_900);
    assert!(lease.lease_expired);
    assert_eq!(
        lease.takeover_legality,
        RoleMailboxTakeoverLegality::RequiresApproval
    );
    assert!(lease
        .response_authorities
        .iter()
        .any(
            |authority| authority.executor_kind == RoleMailboxExecutorKind::LocalSmallModel
                && authority.response_authority_scope
                    == RoleMailboxResponseAuthorityScope::ReadAndTriageOnly
                && !authority.can_reply
                && authority.actor_ineligible_reason.is_some()
        ));
}

#[test]
fn claim_lease_projection_explains_claimant_without_work_state_authority_transfer() {
    let controls = sample_controls();
    let projections = project_role_mailbox_claim_lease(&controls).expect("claim lease projects");

    assert_eq!(projections.len(), 1);
    assert_eq!(
        projections[0].current_claimant_actor_id.as_deref(),
        Some("cloud-model-7")
    );
    assert_eq!(
        projections[0].claim_mode,
        RoleMailboxClaimMode::ExclusiveLease
    );
    assert_eq!(
        projections[0].takeover_legality,
        RoleMailboxTakeoverLegality::RequiresApproval
    );
    assert!(projections[0].stale_or_expired);
    assert!(!projections[0].mutates_locus_authority);
    assert!(!projections[0].mutates_task_board_authority);
    assert!(!projections[0].mutates_work_packet_authority);
    assert!(!projections[0].mutates_micro_task_authority);
}

#[test]
fn claim_lease_action_preview_distinguishes_local_automation_and_governed_routes() {
    let controls = sample_controls();

    let release = preview_role_mailbox_claim_lease_action(
        &controls,
        "role-mailbox-thread-mt030",
        RoleMailboxExecutorKind::CloudModel,
        ClaimLeaseActionKind::Release,
    )
    .expect("release preview exists");
    assert_eq!(release.route, ClaimLeaseActionRoute::MailboxLocal);
    assert_eq!(release.authority_effect, AuthorityEffect::None);
    assert_eq!(release.kernel_action_id, None);

    let automation_claim = preview_role_mailbox_claim_lease_action(
        &controls,
        "role-mailbox-thread-mt030",
        RoleMailboxExecutorKind::WorkflowAutomation,
        ClaimLeaseActionKind::Claim,
    )
    .expect("automation claim preview exists");
    assert_eq!(
        automation_claim.route,
        ClaimLeaseActionRoute::AutomationTriggering
    );
    assert_eq!(
        automation_claim.boundary,
        RoleMailboxAuthorityBoundary::MailboxLocal
    );
    assert!(!automation_claim.mutates_work_state_authority);

    let takeover = preview_role_mailbox_claim_lease_action(
        &controls,
        "role-mailbox-thread-mt030",
        RoleMailboxExecutorKind::Validator,
        ClaimLeaseActionKind::Takeover,
    )
    .expect("takeover preview exists");
    assert_eq!(takeover.route, ClaimLeaseActionRoute::Governed);
    assert_eq!(
        takeover.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        takeover.kernel_action_id.as_deref(),
        Some("kernel.workflow_transition.preview")
    );
}

#[test]
fn claim_lease_rejects_authority_leaks_and_ambiguous_takeover_rules() {
    let mut controls = sample_controls();
    controls.locus_projection_authoritative = true;
    controls.task_board_projection_authoritative = true;
    controls.work_packet_projection_authoritative = true;
    controls.micro_task_projection_authoritative = true;
    controls.leases[0].lease_age_seconds = 0;
    controls.leases[0].lease_expired = false;
    controls.leases[0].takeover_legality = RoleMailboxTakeoverLegality::Allowed;
    controls.leases[0].response_authorities[0].actor_ineligible_reason = None;

    let errors = validate_role_mailbox_claim_lease_controls(&controls)
        .expect_err("unsafe claim lease controls must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "locus_projection_authoritative"));
    assert!(errors
        .iter()
        .any(|error| error.field == "task_board_projection_authoritative"));
    assert!(errors.iter().any(|error| error.field == "lease_expired"));
    assert!(errors
        .iter()
        .any(|error| error.field == "takeover_legality"));
    assert!(errors
        .iter()
        .any(|error| error.field == "response_authorities.actor_ineligible_reason"));
}

#[test]
fn kernel_action_catalog_exposes_role_mailbox_claim_lease_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.role_mailbox_claim_lease.project")
        .expect("Role Mailbox claim lease projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "mailbox_claim_lease_state"));
}

fn sample_controls() -> RoleMailboxClaimLeaseControlsV1 {
    RoleMailboxClaimLeaseControlsV1 {
        schema_id: "hsk.kernel.role_mailbox_claim_lease_controls@1".to_string(),
        controls_id: "kernel002-role-mailbox-claim-lease-mt030".to_string(),
        folded_stub_id: "WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1".to_string(),
        leases: vec![RoleMailboxClaimLeaseV1 {
            claim_id: "claim-mt030".to_string(),
            thread_id: "role-mailbox-thread-mt030".to_string(),
            executor_kind: RoleMailboxExecutorKind::CloudModel,
            executor_identity_ref: "actor://cloud-model-7".to_string(),
            claim_mode: RoleMailboxClaimMode::ExclusiveLease,
            claimed_at: "2026-05-14T16:55:00Z".to_string(),
            lease_state: ClaimLeaseState::Expired,
            current_claimant: Some(ClaimantIdentityV1 {
                actor_id: "cloud-model-7".to_string(),
                actor_kind: RoleMailboxExecutorKind::CloudModel,
                session_id: "session-cloud-model-7".to_string(),
                display_name: "Cloud Model 7".to_string(),
            }),
            lease_age_seconds: 3_900,
            lease_expires_at: "2026-05-14T18:00:00Z".to_string(),
            lease_expired: true,
            takeover_policy: RoleMailboxTakeoverPolicy::StaleLeaseRequiresApproval,
            takeover_legality: RoleMailboxTakeoverLegality::RequiresApproval,
            takeover_reason: Some("lease expired during validator handoff".to_string()),
            prior_claimant: Some(ClaimantIdentityV1 {
                actor_id: "local-small-model-2".to_string(),
                actor_kind: RoleMailboxExecutorKind::LocalSmallModel,
                session_id: "session-local-small-model-2".to_string(),
                display_name: "Local Small Model 2".to_string(),
            }),
            last_handback_reason: Some("needs stronger model review".to_string()),
            allowed_executor_kinds: vec![
                RoleMailboxExecutorKind::LocalSmallModel,
                RoleMailboxExecutorKind::LocalLargeModel,
                RoleMailboxExecutorKind::CloudModel,
                RoleMailboxExecutorKind::Reviewer,
                RoleMailboxExecutorKind::Validator,
                RoleMailboxExecutorKind::Operator,
                RoleMailboxExecutorKind::WorkflowAutomation,
            ],
            response_authorities: sample_authorities(),
            action_rules: sample_action_rules(),
            linked_work_packet_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
                .to_string(),
            linked_micro_task_id: "MT-030".to_string(),
            locus_join_refs: vec!["locus://MT-030".to_string()],
            task_board_projection_ref: "task-board-row-mt030".to_string(),
            work_packet_followup_ref:
                "wp://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
            micro_task_queue_ref: "mt://MT-030".to_string(),
            compact_summary: "Expired cloud-model lease requires validator takeover preview."
                .to_string(),
        }],
        compact_summary_first: true,
        locus_projection_authoritative: false,
        task_board_projection_authoritative: false,
        work_packet_projection_authoritative: false,
        micro_task_projection_authoritative: false,
        product_authority_refs: vec![
            "kernel.role_mailbox_contract".to_string(),
            "kernel.role_mailbox_triage_queue".to_string(),
            "kernel.workflow_transition_registry".to_string(),
            "kernel.locus_work_tracking".to_string(),
            "kernel.dcc_layout_projection_registry".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1.contract.json"
                .to_string(),
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1.md"
                .to_string(),
        ],
    }
}

fn sample_authorities() -> Vec<RoleMailboxResponseAuthorityV1> {
    vec![
        authority(
            RoleMailboxExecutorKind::LocalSmallModel,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            RoleMailboxResponseAuthorityScope::ReadAndTriageOnly,
            true,
            false,
            false,
            Some("local model may claim triage but cannot reply to expired lease"),
        ),
        authority(
            RoleMailboxExecutorKind::LocalLargeModel,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            RoleMailboxResponseAuthorityScope::DraftReply,
            true,
            false,
            false,
            Some("local large model may draft but not resolve expired lease"),
        ),
        authority(
            RoleMailboxExecutorKind::CloudModel,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            RoleMailboxResponseAuthorityScope::Reply,
            true,
            true,
            false,
            None,
        ),
        authority(
            RoleMailboxExecutorKind::Reviewer,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            RoleMailboxResponseAuthorityScope::ReviewOnly,
            false,
            false,
            false,
            Some("reviewer observes but cannot seize execution"),
        ),
        authority(
            RoleMailboxExecutorKind::Validator,
            RoleMailboxAuthorityBoundary::GovernedActionRequired,
            RoleMailboxResponseAuthorityScope::TakeoverAndReply,
            true,
            true,
            true,
            None,
        ),
        authority(
            RoleMailboxExecutorKind::Operator,
            RoleMailboxAuthorityBoundary::GovernedActionRequired,
            RoleMailboxResponseAuthorityScope::FullControl,
            true,
            true,
            true,
            None,
        ),
        authority(
            RoleMailboxExecutorKind::WorkflowAutomation,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            RoleMailboxResponseAuthorityScope::AutomationRoutingOnly,
            true,
            false,
            false,
            Some("automation may claim only to enqueue routing work"),
        ),
    ]
}

fn authority(
    executor_kind: RoleMailboxExecutorKind,
    boundary: RoleMailboxAuthorityBoundary,
    response_authority_scope: RoleMailboxResponseAuthorityScope,
    can_claim: bool,
    can_reply: bool,
    can_takeover: bool,
    actor_ineligible_reason: Option<&str>,
) -> RoleMailboxResponseAuthorityV1 {
    RoleMailboxResponseAuthorityV1 {
        authority_id: format!("authority-{executor_kind:?}").to_lowercase(),
        executor_kind,
        allowed_responses: vec![
            RoleMailboxAllowedResponseKind::Acknowledge,
            RoleMailboxAllowedResponseKind::Snooze,
            RoleMailboxAllowedResponseKind::Reply,
        ],
        linked_action_request_ids: vec!["action-request-mt030".to_string()],
        evidence_refs: vec!["evidence://claim-lease-mt030".to_string()],
        response_authority_scope,
        boundary,
        can_claim,
        can_reply,
        can_takeover,
        actor_ineligible_reason: actor_ineligible_reason.map(str::to_string),
    }
}

fn sample_action_rules() -> Vec<ClaimLeaseActionRuleV1> {
    vec![
        action(
            ClaimLeaseActionKind::Release,
            RoleMailboxExecutorKind::CloudModel,
            ClaimLeaseActionRoute::MailboxLocal,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            None,
        ),
        action(
            ClaimLeaseActionKind::Renew,
            RoleMailboxExecutorKind::CloudModel,
            ClaimLeaseActionRoute::MailboxLocal,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            None,
        ),
        action(
            ClaimLeaseActionKind::Claim,
            RoleMailboxExecutorKind::WorkflowAutomation,
            ClaimLeaseActionRoute::AutomationTriggering,
            RoleMailboxAuthorityBoundary::MailboxLocal,
            None,
        ),
        action(
            ClaimLeaseActionKind::Takeover,
            RoleMailboxExecutorKind::Validator,
            ClaimLeaseActionRoute::Governed,
            RoleMailboxAuthorityBoundary::GovernedActionRequired,
            Some("kernel.workflow_transition.preview"),
        ),
        action(
            ClaimLeaseActionKind::Reply,
            RoleMailboxExecutorKind::Operator,
            ClaimLeaseActionRoute::Governed,
            RoleMailboxAuthorityBoundary::GovernedActionRequired,
            Some("kernel.workflow_transition.preview"),
        ),
    ]
}

fn action(
    kind: ClaimLeaseActionKind,
    executor_kind: RoleMailboxExecutorKind,
    route: ClaimLeaseActionRoute,
    boundary: RoleMailboxAuthorityBoundary,
    kernel_action_id: Option<&str>,
) -> ClaimLeaseActionRuleV1 {
    ClaimLeaseActionRuleV1 {
        action_id: format!("action-{executor_kind:?}-{kind:?}").to_lowercase(),
        kind,
        executor_kind,
        route,
        boundary,
        preview_required: true,
        kernel_action_id: kernel_action_id.map(str::to_string),
        field_paths: vec!["claim_mode".to_string(), "lease_state".to_string()],
    }
}

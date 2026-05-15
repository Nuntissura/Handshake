use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{
    action_envelope::AuthorityEffect,
    role_mailbox_contract::{RoleMailboxAllowedResponseKind, RoleMailboxAuthorityBoundary},
};

pub const FOLDED_ROLE_MAILBOX_CLAIM_LEASE_STUB_ID: &str =
    "WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1";

const REQUIRED_EXECUTOR_KINDS: [RoleMailboxExecutorKind; 7] = [
    RoleMailboxExecutorKind::LocalSmallModel,
    RoleMailboxExecutorKind::LocalLargeModel,
    RoleMailboxExecutorKind::CloudModel,
    RoleMailboxExecutorKind::Reviewer,
    RoleMailboxExecutorKind::Validator,
    RoleMailboxExecutorKind::Operator,
    RoleMailboxExecutorKind::WorkflowAutomation,
];

const REQUIRED_ACTION_KINDS: [ClaimLeaseActionKind; 5] = [
    ClaimLeaseActionKind::Claim,
    ClaimLeaseActionKind::Release,
    ClaimLeaseActionKind::Renew,
    ClaimLeaseActionKind::Takeover,
    ClaimLeaseActionKind::Reply,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleMailboxExecutorKind {
    LocalSmallModel,
    LocalLargeModel,
    CloudModel,
    Reviewer,
    Validator,
    Operator,
    WorkflowAutomation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleMailboxClaimMode {
    ExclusiveLease,
    SharedObserver,
    BroadcastRequest,
    HandoffReservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimLeaseState {
    Unclaimed,
    Active,
    Released,
    Expired,
    TakenOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleMailboxTakeoverPolicy {
    Never,
    ExpiredLeaseAutomatic,
    StaleLeaseRequiresApproval,
    ReviewerApprovalRequired,
    ValidatorApprovalRequired,
    OperatorApprovalRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleMailboxTakeoverLegality {
    Blocked,
    Allowed,
    RequiresApproval,
    ActorIneligible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleMailboxResponseAuthorityScope {
    ReadAndTriageOnly,
    DraftReply,
    Reply,
    ReviewOnly,
    TakeoverAndReply,
    FullControl,
    AutomationRoutingOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimLeaseActionKind {
    Claim,
    Release,
    Renew,
    Takeover,
    Reply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimLeaseActionRoute {
    MailboxLocal,
    AutomationTriggering,
    Governed,
    Blocked,
    ActorIneligible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimantIdentityV1 {
    pub actor_id: String,
    pub actor_kind: RoleMailboxExecutorKind,
    pub session_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxResponseAuthorityV1 {
    pub authority_id: String,
    pub executor_kind: RoleMailboxExecutorKind,
    pub allowed_responses: Vec<RoleMailboxAllowedResponseKind>,
    pub linked_action_request_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub response_authority_scope: RoleMailboxResponseAuthorityScope,
    pub boundary: RoleMailboxAuthorityBoundary,
    pub can_claim: bool,
    pub can_reply: bool,
    pub can_takeover: bool,
    pub actor_ineligible_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimLeaseActionRuleV1 {
    pub action_id: String,
    pub kind: ClaimLeaseActionKind,
    pub executor_kind: RoleMailboxExecutorKind,
    pub route: ClaimLeaseActionRoute,
    pub boundary: RoleMailboxAuthorityBoundary,
    pub preview_required: bool,
    pub kernel_action_id: Option<String>,
    pub field_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxClaimLeaseV1 {
    pub claim_id: String,
    pub thread_id: String,
    pub executor_kind: RoleMailboxExecutorKind,
    pub executor_identity_ref: String,
    pub claim_mode: RoleMailboxClaimMode,
    pub claimed_at: String,
    pub lease_state: ClaimLeaseState,
    pub current_claimant: Option<ClaimantIdentityV1>,
    pub lease_age_seconds: u64,
    pub lease_expires_at: String,
    pub lease_expired: bool,
    pub takeover_policy: RoleMailboxTakeoverPolicy,
    pub takeover_legality: RoleMailboxTakeoverLegality,
    pub takeover_reason: Option<String>,
    pub prior_claimant: Option<ClaimantIdentityV1>,
    pub last_handback_reason: Option<String>,
    pub allowed_executor_kinds: Vec<RoleMailboxExecutorKind>,
    pub response_authorities: Vec<RoleMailboxResponseAuthorityV1>,
    pub action_rules: Vec<ClaimLeaseActionRuleV1>,
    pub linked_work_packet_id: String,
    pub linked_micro_task_id: String,
    pub locus_join_refs: Vec<String>,
    pub task_board_projection_ref: String,
    pub work_packet_followup_ref: String,
    pub micro_task_queue_ref: String,
    pub compact_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxClaimLeaseControlsV1 {
    pub schema_id: String,
    pub controls_id: String,
    pub folded_stub_id: String,
    pub leases: Vec<RoleMailboxClaimLeaseV1>,
    pub compact_summary_first: bool,
    pub locus_projection_authoritative: bool,
    pub task_board_projection_authoritative: bool,
    pub work_packet_projection_authoritative: bool,
    pub micro_task_projection_authoritative: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxClaimLeaseProjectionV1 {
    pub schema_id: String,
    pub thread_id: String,
    pub claim_id: String,
    pub current_claimant_actor_id: Option<String>,
    pub current_claimant_kind: Option<RoleMailboxExecutorKind>,
    pub claim_mode: RoleMailboxClaimMode,
    pub lease_state: ClaimLeaseState,
    pub lease_age_seconds: u64,
    pub lease_expires_at: String,
    pub stale_or_expired: bool,
    pub takeover_legality: RoleMailboxTakeoverLegality,
    pub prior_claimant_actor_id: Option<String>,
    pub last_handback_reason: Option<String>,
    pub responder_eligibility: Vec<RoleMailboxResponseAuthorityV1>,
    pub locus_join_refs: Vec<String>,
    pub task_board_projection_ref: String,
    pub work_packet_followup_ref: String,
    pub micro_task_queue_ref: String,
    pub compact_summary: String,
    pub mutates_locus_authority: bool,
    pub mutates_task_board_authority: bool,
    pub mutates_work_packet_authority: bool,
    pub mutates_micro_task_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxClaimLeaseActionPreviewV1 {
    pub schema_id: String,
    pub thread_id: String,
    pub action_id: String,
    pub action_kind: ClaimLeaseActionKind,
    pub executor_kind: RoleMailboxExecutorKind,
    pub route: ClaimLeaseActionRoute,
    pub boundary: RoleMailboxAuthorityBoundary,
    pub authority_effect: AuthorityEffect,
    pub kernel_action_id: Option<String>,
    pub preview_required: bool,
    pub actor_ineligible_reason: Option<String>,
    pub field_paths: Vec<String>,
    pub mutates_work_state_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxClaimLeaseValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_role_mailbox_claim_lease_controls(
    controls: &RoleMailboxClaimLeaseControlsV1,
) -> Result<(), Vec<RoleMailboxClaimLeaseValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &controls.schema_id);
    require_non_empty(&mut errors, "controls_id", &controls.controls_id);
    require_non_empty(&mut errors, "folded_stub_id", &controls.folded_stub_id);
    require_vec(&mut errors, "leases", &controls.leases);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &controls.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &controls.folded_source_refs,
    );

    if controls.folded_stub_id != FOLDED_ROLE_MAILBOX_CLAIM_LEASE_STUB_ID {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "folded_stub_id",
            message: "claim lease controls must bind the folded Role Mailbox claim-lease stub",
        });
    }
    if !contains_text(
        &controls.folded_source_refs,
        FOLDED_ROLE_MAILBOX_CLAIM_LEASE_STUB_ID,
    ) {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "folded_source_refs",
            message: "folded Role Mailbox claim-lease source must be preserved",
        });
    }
    if !controls.compact_summary_first {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "compact_summary_first",
            message: "claim lease projection must be compact-summary-first",
        });
    }
    if controls.locus_projection_authoritative {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "locus_projection_authoritative",
            message: "mailbox claim projection must not become Locus authority",
        });
    }
    if controls.task_board_projection_authoritative {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "task_board_projection_authoritative",
            message: "mailbox claim projection must not become Task Board authority",
        });
    }
    if controls.work_packet_projection_authoritative {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "work_packet_projection_authoritative",
            message: "mailbox claim projection must not become Work Packet authority",
        });
    }
    if controls.micro_task_projection_authoritative {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "micro_task_projection_authoritative",
            message: "mailbox claim projection must not become Micro-Task authority",
        });
    }

    validate_refs(&mut errors, controls);
    validate_leases(&mut errors, &controls.leases);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_role_mailbox_claim_lease(
    controls: &RoleMailboxClaimLeaseControlsV1,
) -> Result<Vec<RoleMailboxClaimLeaseProjectionV1>, Vec<RoleMailboxClaimLeaseValidationError>> {
    validate_role_mailbox_claim_lease_controls(controls)?;

    Ok(controls
        .leases
        .iter()
        .map(|lease| RoleMailboxClaimLeaseProjectionV1 {
            schema_id: "hsk.kernel.role_mailbox_claim_lease_projection@1".to_string(),
            thread_id: lease.thread_id.clone(),
            claim_id: lease.claim_id.clone(),
            current_claimant_actor_id: lease
                .current_claimant
                .as_ref()
                .map(|claimant| claimant.actor_id.clone()),
            current_claimant_kind: lease
                .current_claimant
                .as_ref()
                .map(|claimant| claimant.actor_kind),
            claim_mode: lease.claim_mode,
            lease_state: lease.lease_state,
            lease_age_seconds: lease.lease_age_seconds,
            lease_expires_at: lease.lease_expires_at.clone(),
            stale_or_expired: lease.lease_expired
                || matches!(lease.lease_state, ClaimLeaseState::Expired),
            takeover_legality: lease.takeover_legality,
            prior_claimant_actor_id: lease
                .prior_claimant
                .as_ref()
                .map(|claimant| claimant.actor_id.clone()),
            last_handback_reason: lease.last_handback_reason.clone(),
            responder_eligibility: lease.response_authorities.clone(),
            locus_join_refs: lease.locus_join_refs.clone(),
            task_board_projection_ref: lease.task_board_projection_ref.clone(),
            work_packet_followup_ref: lease.work_packet_followup_ref.clone(),
            micro_task_queue_ref: lease.micro_task_queue_ref.clone(),
            compact_summary: lease.compact_summary.clone(),
            mutates_locus_authority: false,
            mutates_task_board_authority: false,
            mutates_work_packet_authority: false,
            mutates_micro_task_authority: false,
        })
        .collect())
}

pub fn preview_role_mailbox_claim_lease_action(
    controls: &RoleMailboxClaimLeaseControlsV1,
    thread_id: &str,
    executor_kind: RoleMailboxExecutorKind,
    action_kind: ClaimLeaseActionKind,
) -> Result<RoleMailboxClaimLeaseActionPreviewV1, Vec<RoleMailboxClaimLeaseValidationError>> {
    validate_role_mailbox_claim_lease_controls(controls)?;

    let Some(lease) = controls
        .leases
        .iter()
        .find(|lease| lease.thread_id == thread_id)
    else {
        return Err(vec![RoleMailboxClaimLeaseValidationError {
            field: "thread_id",
            message: "requested Role Mailbox claim-lease thread is not registered",
        }]);
    };
    let Some(action_rule) = lease
        .action_rules
        .iter()
        .find(|rule| rule.executor_kind == executor_kind && rule.kind == action_kind)
    else {
        return Err(vec![RoleMailboxClaimLeaseValidationError {
            field: "action_rules.kind",
            message: "requested claim lease action is not registered for the executor kind",
        }]);
    };

    let actor_ineligible_reason = lease
        .response_authorities
        .iter()
        .find(|authority| authority.executor_kind == executor_kind)
        .and_then(|authority| authority.actor_ineligible_reason.clone());

    Ok(RoleMailboxClaimLeaseActionPreviewV1 {
        schema_id: "hsk.kernel.role_mailbox_claim_lease_action_preview@1".to_string(),
        thread_id: lease.thread_id.clone(),
        action_id: action_rule.action_id.clone(),
        action_kind: action_rule.kind,
        executor_kind: action_rule.executor_kind,
        route: action_rule.route,
        boundary: action_rule.boundary,
        authority_effect: authority_effect_for_route(action_rule.route),
        kernel_action_id: action_rule.kernel_action_id.clone(),
        preview_required: action_rule.preview_required,
        actor_ineligible_reason,
        field_paths: action_rule.field_paths.clone(),
        mutates_work_state_authority: false,
    })
}

fn validate_refs(
    errors: &mut Vec<RoleMailboxClaimLeaseValidationError>,
    controls: &RoleMailboxClaimLeaseControlsV1,
) {
    for required_ref in [
        "kernel.role_mailbox_contract",
        "kernel.role_mailbox_triage_queue",
        "kernel.workflow_transition_registry",
        "kernel.locus_work_tracking",
        "kernel.dcc_layout_projection_registry",
    ] {
        if !contains_exact(&controls.product_authority_refs, required_ref) {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "product_authority_refs",
                message: "claim lease controls must cite Role Mailbox, triage, workflow, Locus, and DCC authority refs",
            });
        }
    }
}

fn validate_leases(
    errors: &mut Vec<RoleMailboxClaimLeaseValidationError>,
    leases: &[RoleMailboxClaimLeaseV1],
) {
    let mut claim_ids = HashSet::new();
    let mut active_exclusive_threads = HashSet::new();

    for lease in leases {
        if !claim_ids.insert(lease.claim_id.as_str()) {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "claim_id",
                message: "claim ids must be unique",
            });
        }
        if matches!(
            lease.claim_mode,
            RoleMailboxClaimMode::ExclusiveLease | RoleMailboxClaimMode::HandoffReservation
        ) && lease.lease_state == ClaimLeaseState::Active
            && !lease.lease_expired
            && !active_exclusive_threads.insert(lease.thread_id.as_str())
        {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "thread_id",
                message: "exclusive leases and handoff reservations allow one active claimant per thread",
            });
        }

        require_non_empty(errors, "claim_id", &lease.claim_id);
        require_non_empty(errors, "thread_id", &lease.thread_id);
        require_non_empty(
            errors,
            "executor_identity_ref",
            &lease.executor_identity_ref,
        );
        require_non_empty(errors, "claimed_at", &lease.claimed_at);
        require_non_empty(errors, "lease_expires_at", &lease.lease_expires_at);
        require_non_empty(
            errors,
            "linked_work_packet_id",
            &lease.linked_work_packet_id,
        );
        require_non_empty(errors, "linked_micro_task_id", &lease.linked_micro_task_id);
        require_vec(errors, "locus_join_refs", &lease.locus_join_refs);
        require_non_empty(
            errors,
            "task_board_projection_ref",
            &lease.task_board_projection_ref,
        );
        require_non_empty(
            errors,
            "work_packet_followup_ref",
            &lease.work_packet_followup_ref,
        );
        require_non_empty(errors, "micro_task_queue_ref", &lease.micro_task_queue_ref);
        require_non_empty(errors, "compact_summary", &lease.compact_summary);
        require_vec(
            errors,
            "allowed_executor_kinds",
            &lease.allowed_executor_kinds,
        );
        require_vec(errors, "response_authorities", &lease.response_authorities);
        require_vec(errors, "action_rules", &lease.action_rules);

        validate_claimant(errors, lease);
        validate_executor_kinds(errors, lease);
        validate_lease_expiry(errors, lease);
        validate_response_authorities(errors, lease);
        validate_action_rules(errors, lease);
    }
}

fn validate_claimant(
    errors: &mut Vec<RoleMailboxClaimLeaseValidationError>,
    lease: &RoleMailboxClaimLeaseV1,
) {
    if requires_claimant(lease.lease_state) && lease.current_claimant.is_none() {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "current_claimant",
            message: "active, expired, and taken-over claims require claimant identity",
        });
    }

    if let Some(claimant) = &lease.current_claimant {
        require_non_empty(errors, "current_claimant.actor_id", &claimant.actor_id);
        require_non_empty(errors, "current_claimant.session_id", &claimant.session_id);
        require_non_empty(
            errors,
            "current_claimant.display_name",
            &claimant.display_name,
        );
        if claimant.actor_kind != lease.executor_kind {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "current_claimant.actor_kind",
                message: "current claimant kind must match lease executor kind",
            });
        }
    }

    if let Some(prior_claimant) = &lease.prior_claimant {
        require_non_empty(errors, "prior_claimant.actor_id", &prior_claimant.actor_id);
        require_non_empty(
            errors,
            "prior_claimant.session_id",
            &prior_claimant.session_id,
        );
        require_non_empty(
            errors,
            "prior_claimant.display_name",
            &prior_claimant.display_name,
        );
    }
}

fn validate_executor_kinds(
    errors: &mut Vec<RoleMailboxClaimLeaseValidationError>,
    lease: &RoleMailboxClaimLeaseV1,
) {
    let allowed: HashSet<RoleMailboxExecutorKind> =
        lease.allowed_executor_kinds.iter().copied().collect();
    for required_kind in REQUIRED_EXECUTOR_KINDS {
        if !allowed.contains(&required_kind) {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "allowed_executor_kinds",
                message: "local-small, local-large, cloud, reviewer, validator, operator, and automation executor kinds are required",
            });
        }
    }
    if !allowed.contains(&lease.executor_kind) {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "executor_kind",
            message: "lease executor kind must be present in the allowed executor kind list",
        });
    }
}

fn validate_lease_expiry(
    errors: &mut Vec<RoleMailboxClaimLeaseValidationError>,
    lease: &RoleMailboxClaimLeaseV1,
) {
    let stale_or_expired = lease.lease_expired || lease.lease_state == ClaimLeaseState::Expired;
    if lease.lease_state == ClaimLeaseState::Expired && !lease.lease_expired {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "lease_expired",
            message: "expired lease state requires the lease-expired field",
        });
    }
    if stale_or_expired && lease.lease_age_seconds == 0 {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "lease_age_seconds",
            message: "expired or stale leases require a non-zero lease age",
        });
    }
    if stale_or_expired
        && lease.takeover_policy == RoleMailboxTakeoverPolicy::StaleLeaseRequiresApproval
        && lease.takeover_legality != RoleMailboxTakeoverLegality::RequiresApproval
    {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "takeover_legality",
            message: "stale leases under approval policy require an approval-gated takeover",
        });
    }
    if lease.takeover_legality == RoleMailboxTakeoverLegality::RequiresApproval
        && lease.takeover_reason.as_deref().is_none_or(str::is_empty)
    {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "takeover_reason",
            message: "approval-gated takeover requires an explicit reason",
        });
    }
    if lease.takeover_policy == RoleMailboxTakeoverPolicy::Never
        && lease.takeover_legality != RoleMailboxTakeoverLegality::Blocked
    {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field: "takeover_legality",
            message: "takeover policy Never must be projected as blocked",
        });
    }
}

fn validate_response_authorities(
    errors: &mut Vec<RoleMailboxClaimLeaseValidationError>,
    lease: &RoleMailboxClaimLeaseV1,
) {
    let mut authority_ids = HashSet::new();
    let mut authority_kinds = HashSet::new();
    for authority in &lease.response_authorities {
        if !authority_ids.insert(authority.authority_id.as_str()) {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "response_authorities.authority_id",
                message: "response authority ids must be unique",
            });
        }
        authority_kinds.insert(authority.executor_kind);
        require_non_empty(
            errors,
            "response_authorities.authority_id",
            &authority.authority_id,
        );
        require_vec(
            errors,
            "response_authorities.allowed_responses",
            &authority.allowed_responses,
        );
        require_vec(
            errors,
            "response_authorities.linked_action_request_ids",
            &authority.linked_action_request_ids,
        );
        require_vec(
            errors,
            "response_authorities.evidence_refs",
            &authority.evidence_refs,
        );

        if constrained_authority_scope(authority.response_authority_scope)
            && authority
                .actor_ineligible_reason
                .as_deref()
                .is_none_or(str::is_empty)
        {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "response_authorities.actor_ineligible_reason",
                message: "read-only, draft-only, review-only, and automation-only scopes require actor-ineligible reason text",
            });
        }
        if authority.response_authority_scope
            == RoleMailboxResponseAuthorityScope::ReadAndTriageOnly
            && authority.can_reply
        {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "response_authorities.can_reply",
                message: "read-and-triage visibility must not imply reply authority",
            });
        }
        if authority.boundary == RoleMailboxAuthorityBoundary::GovernedActionRequired
            && !matches!(
                authority.response_authority_scope,
                RoleMailboxResponseAuthorityScope::TakeoverAndReply
                    | RoleMailboxResponseAuthorityScope::FullControl
            )
        {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "response_authorities.response_authority_scope",
                message:
                    "governed response authority must expose takeover/reply or full-control scope",
            });
        }
    }

    for required_kind in REQUIRED_EXECUTOR_KINDS {
        if !authority_kinds.contains(&required_kind) {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "response_authorities.executor_kind",
                message: "every required executor kind needs explicit response authority",
            });
        }
    }
}

fn validate_action_rules(
    errors: &mut Vec<RoleMailboxClaimLeaseValidationError>,
    lease: &RoleMailboxClaimLeaseV1,
) {
    let mut action_ids = HashSet::new();
    let mut action_kinds = HashSet::new();
    let allowed_executor_kinds: HashSet<RoleMailboxExecutorKind> =
        lease.allowed_executor_kinds.iter().copied().collect();

    for rule in &lease.action_rules {
        if !action_ids.insert(rule.action_id.as_str()) {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "action_rules.action_id",
                message: "claim lease action ids must be unique",
            });
        }
        action_kinds.insert(rule.kind);
        require_non_empty(errors, "action_rules.action_id", &rule.action_id);
        require_vec(errors, "action_rules.field_paths", &rule.field_paths);

        if !allowed_executor_kinds.contains(&rule.executor_kind) {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "action_rules.executor_kind",
                message: "action rule executor kind must be allowed by the lease",
            });
        }
        if !rule.preview_required {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "action_rules.preview_required",
                message: "claim, release, renew, takeover, and reply actions must be previewed",
            });
        }
        validate_action_route(errors, rule);
    }

    for required_kind in REQUIRED_ACTION_KINDS {
        if !action_kinds.contains(&required_kind) {
            errors.push(RoleMailboxClaimLeaseValidationError {
                field: "action_rules.kind",
                message: "claim, release, renew, takeover, and reply action previews are required",
            });
        }
    }
}

fn validate_action_route(
    errors: &mut Vec<RoleMailboxClaimLeaseValidationError>,
    rule: &ClaimLeaseActionRuleV1,
) {
    match rule.route {
        ClaimLeaseActionRoute::MailboxLocal | ClaimLeaseActionRoute::AutomationTriggering => {
            if rule.boundary != RoleMailboxAuthorityBoundary::MailboxLocal {
                errors.push(RoleMailboxClaimLeaseValidationError {
                    field: "action_rules.boundary",
                    message: "mailbox-local and automation-triggering actions must use mailbox-local boundary",
                });
            }
            if rule.kernel_action_id.is_some() {
                errors.push(RoleMailboxClaimLeaseValidationError {
                    field: "action_rules.kernel_action_id",
                    message: "mailbox-local and automation-triggering actions must not carry governed kernel action ids",
                });
            }
        }
        ClaimLeaseActionRoute::Governed => {
            if rule.boundary != RoleMailboxAuthorityBoundary::GovernedActionRequired {
                errors.push(RoleMailboxClaimLeaseValidationError {
                    field: "action_rules.boundary",
                    message: "governed claim lease actions require governed action boundary",
                });
            }
            if rule.kernel_action_id.as_deref().is_none_or(str::is_empty) {
                errors.push(RoleMailboxClaimLeaseValidationError {
                    field: "action_rules.kernel_action_id",
                    message: "governed claim lease actions require a catalog action id",
                });
            }
        }
        ClaimLeaseActionRoute::Blocked | ClaimLeaseActionRoute::ActorIneligible => {
            if rule.kernel_action_id.is_some() {
                errors.push(RoleMailboxClaimLeaseValidationError {
                    field: "action_rules.kernel_action_id",
                    message:
                        "blocked and actor-ineligible actions must not carry kernel action ids",
                });
            }
        }
    }
}

fn requires_claimant(state: ClaimLeaseState) -> bool {
    matches!(
        state,
        ClaimLeaseState::Active | ClaimLeaseState::Expired | ClaimLeaseState::TakenOver
    )
}

fn constrained_authority_scope(scope: RoleMailboxResponseAuthorityScope) -> bool {
    matches!(
        scope,
        RoleMailboxResponseAuthorityScope::ReadAndTriageOnly
            | RoleMailboxResponseAuthorityScope::DraftReply
            | RoleMailboxResponseAuthorityScope::ReviewOnly
            | RoleMailboxResponseAuthorityScope::AutomationRoutingOnly
    )
}

fn authority_effect_for_route(route: ClaimLeaseActionRoute) -> AuthorityEffect {
    match route {
        ClaimLeaseActionRoute::MailboxLocal
        | ClaimLeaseActionRoute::AutomationTriggering
        | ClaimLeaseActionRoute::Blocked
        | ClaimLeaseActionRoute::ActorIneligible => AuthorityEffect::None,
        ClaimLeaseActionRoute::Governed => AuthorityEffect::PrePromotionEvidenceOnly,
    }
}

fn require_non_empty(
    errors: &mut Vec<RoleMailboxClaimLeaseValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<RoleMailboxClaimLeaseValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(RoleMailboxClaimLeaseValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}

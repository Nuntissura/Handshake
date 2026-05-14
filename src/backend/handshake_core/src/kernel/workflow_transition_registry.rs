use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::action_envelope::AuthorityEffect;

const FOLDED_WORKFLOW_TRANSITION_STUB: &str = "WP-1-Workflow-Transition-Automation-Registry-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowMutationKind {
    WorkPacket,
    MicroTask,
    TaskBoardProjection,
    RoleMailboxQueue,
    DevCommandCenterAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApprovalBoundary {
    None,
    ValidatorApproval,
    HumanApproval,
    PromotionGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueueAutomationMode {
    Automatic,
    RequiresApproval,
    PreviewOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueueAutomationTriggerKind {
    MailboxReply,
    DependencyCleared,
    ValidationOutcome,
    RetryDue,
    ApprovalDecision,
    EscalationAcknowledgement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueueAutomationSourceKind {
    ProductWorkflowEvent,
    RoleMailboxStableEvent,
    DependencyStateRecord,
    ValidationReceipt,
    RetrySchedule,
    ApprovalReceipt,
    EscalationReceipt,
    BoardLaneName,
    MailboxChronology,
    PacketProse,
}

impl QueueAutomationSourceKind {
    pub fn is_authoritative(self) -> bool {
        matches!(
            self,
            QueueAutomationSourceKind::ProductWorkflowEvent
                | QueueAutomationSourceKind::RoleMailboxStableEvent
                | QueueAutomationSourceKind::DependencyStateRecord
                | QueueAutomationSourceKind::ValidationReceipt
                | QueueAutomationSourceKind::RetrySchedule
                | QueueAutomationSourceKind::ApprovalReceipt
                | QueueAutomationSourceKind::EscalationReceipt
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccTransitionPreviewPosture {
    ViewOnly,
    Automatic,
    ActorIneligible,
    ApprovalRequired,
    Lawful,
    MissingRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccTransitionPreviewV1 {
    pub preview_id: String,
    pub panel_id: String,
    pub summary: String,
    pub posture: DccTransitionPreviewPosture,
    pub approval_boundary: ApprovalBoundary,
    pub authority_effect: AuthorityEffect,
    pub primary_state_fields: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTransitionRuleV1 {
    pub rule_id: String,
    pub workflow_family_id: String,
    pub mutation_kind: WorkflowMutationKind,
    pub from_state_id: String,
    pub to_state_id: String,
    pub governed_action_id: String,
    pub eligible_actor_kinds: Vec<String>,
    pub approval_boundary: ApprovalBoundary,
    pub validation_hooks: Vec<String>,
    pub dcc_preview: DccTransitionPreviewV1,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueAutomationRuleV1 {
    pub rule_id: String,
    pub trigger_kind: QueueAutomationTriggerKind,
    pub trigger_source_kind: QueueAutomationSourceKind,
    pub transition_rule_id: String,
    pub mode: QueueAutomationMode,
    pub stable_source_ids: Vec<String>,
    pub dcc_preview_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutorEligibilityPolicyV1 {
    pub policy_id: String,
    pub workflow_family_id: String,
    pub actor_kind: String,
    pub executor_profile_id: String,
    pub eligible_transition_rule_ids: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub local_small_model_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTransitionRegistryV1 {
    pub schema_id: String,
    pub registry_id: String,
    pub version: u32,
    pub transition_rules: Vec<WorkflowTransitionRuleV1>,
    pub queue_automation_rules: Vec<QueueAutomationRuleV1>,
    pub executor_policies: Vec<ExecutorEligibilityPolicyV1>,
}

impl WorkflowTransitionRegistryV1 {
    pub fn transition_rule(&self, rule_id: &str) -> Option<&WorkflowTransitionRuleV1> {
        self.transition_rules
            .iter()
            .find(|rule| rule.rule_id == rule_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowTransitionRegistryValidationError {
    DuplicateTransitionRuleId {
        rule_id: String,
    },
    DuplicateAutomationRuleId {
        rule_id: String,
    },
    DuplicateExecutorPolicyId {
        policy_id: String,
    },
    MissingTransitionField {
        rule_id: String,
        field: &'static str,
    },
    MissingAutomationField {
        rule_id: String,
        field: &'static str,
    },
    MissingExecutorPolicyField {
        policy_id: String,
        field: &'static str,
    },
    UnknownAutomationTransition {
        rule_id: String,
        transition_rule_id: String,
    },
    UnknownExecutorTransition {
        policy_id: String,
        transition_rule_id: String,
    },
    AutomationCrossesApprovalBoundary {
        rule_id: String,
        transition_rule_id: String,
        approval_boundary: ApprovalBoundary,
    },
    NonAuthoritativeAutomationSource {
        rule_id: String,
        source_kind: QueueAutomationSourceKind,
    },
}

pub fn kernel002_workflow_transition_registry() -> WorkflowTransitionRegistryV1 {
    WorkflowTransitionRegistryV1 {
        schema_id: "hsk.kernel.workflow_transition_registry@1".to_string(),
        registry_id: "kernel002-workflow-transition-registry-v1".to_string(),
        version: 1,
        transition_rules: vec![
            transition_rule(
                "kernel.mt.claim",
                WorkflowMutationKind::MicroTask,
                "MT_PENDING",
                "MT_CLAIMED",
                "kernel.microtask.claim",
                &["CODER", "KERNEL_BUILDER"],
                ApprovalBoundary::None,
                &["transition_rule_registered", "actor_eligibility"],
            ),
            transition_rule(
                "kernel.mt.complete",
                WorkflowMutationKind::MicroTask,
                "MT_CLAIMED",
                "MT_COMPLETED",
                "kernel.microtask.complete",
                &["CODER", "KERNEL_BUILDER"],
                ApprovalBoundary::None,
                &[
                    "transition_rule_registered",
                    "actor_eligibility",
                    "evidence_attached",
                ],
            ),
            transition_rule(
                "kernel.mt.validator_verdict",
                WorkflowMutationKind::MicroTask,
                "MT_COMPLETED",
                "MT_VALIDATED",
                "kernel.microtask.validator_verdict",
                &["WP_VALIDATOR", "INTEGRATION_VALIDATOR", "KERNEL_BUILDER"],
                ApprovalBoundary::ValidatorApproval,
                &[
                    "transition_rule_registered",
                    "actor_eligibility",
                    "approval_boundary",
                ],
            ),
            transition_rule(
                "kernel.mt.dependency_cleared",
                WorkflowMutationKind::MicroTask,
                "MT_BLOCKED",
                "MT_PENDING",
                "kernel.microtask.dependency_cleared",
                &["SYSTEM", "ORCHESTRATOR", "KERNEL_BUILDER"],
                ApprovalBoundary::None,
                &[
                    "transition_rule_registered",
                    "dependency_state_record",
                    "actor_eligibility",
                ],
            ),
            transition_rule(
                "kernel.mailbox.reply_ready",
                WorkflowMutationKind::RoleMailboxQueue,
                "WAITING_ON_ROLE_REPLY",
                "READY_FOR_REVIEW",
                "kernel.role_mailbox.reply_received",
                &["SYSTEM", "ORCHESTRATOR", "KERNEL_BUILDER"],
                ApprovalBoundary::None,
                &[
                    "transition_rule_registered",
                    "role_mailbox_stable_event",
                    "actor_eligibility",
                ],
            ),
        ],
        queue_automation_rules: vec![
            queue_automation_rule(
                "kernel.queue.dependency_cleared",
                QueueAutomationTriggerKind::DependencyCleared,
                QueueAutomationSourceKind::DependencyStateRecord,
                "kernel.mt.dependency_cleared",
                QueueAutomationMode::Automatic,
                &["dependency-state-record-id"],
            ),
            queue_automation_rule(
                "kernel.queue.mailbox_reply_ready",
                QueueAutomationTriggerKind::MailboxReply,
                QueueAutomationSourceKind::RoleMailboxStableEvent,
                "kernel.mailbox.reply_ready",
                QueueAutomationMode::Automatic,
                &["role-mailbox-event-id"],
            ),
            queue_automation_rule(
                "kernel.queue.validator_verdict_preview",
                QueueAutomationTriggerKind::ValidationOutcome,
                QueueAutomationSourceKind::ValidationReceipt,
                "kernel.mt.validator_verdict",
                QueueAutomationMode::RequiresApproval,
                &["validator-receipt-id"],
            ),
        ],
        executor_policies: vec![
            executor_policy(
                "kernel.executor.local_small_model",
                "LOCAL_SMALL_MODEL",
                "local-small-model",
                &["kernel.mt.claim", "kernel.mt.dependency_cleared"],
                &["kernel.workflow.preview", "kernel.microtask.claim"],
                true,
            ),
            executor_policy(
                "kernel.executor.validator",
                "INTEGRATION_VALIDATOR",
                "validator",
                &["kernel.mt.validator_verdict"],
                &["kernel.workflow.preview", "kernel.validation.verdict"],
                false,
            ),
            executor_policy(
                "kernel.executor.kernel_builder",
                "KERNEL_BUILDER",
                "kernel-builder",
                &[
                    "kernel.mt.claim",
                    "kernel.mt.complete",
                    "kernel.mt.validator_verdict",
                    "kernel.mt.dependency_cleared",
                    "kernel.mailbox.reply_ready",
                ],
                &["kernel.workflow.preview", "kernel.workflow.transition"],
                false,
            ),
        ],
    }
}

pub fn validate_workflow_transition_registry(
    registry: &WorkflowTransitionRegistryV1,
) -> Result<(), Vec<WorkflowTransitionRegistryValidationError>> {
    let mut errors = Vec::new();
    let mut transition_ids = HashSet::new();

    for rule in &registry.transition_rules {
        if !transition_ids.insert(rule.rule_id.clone()) {
            errors.push(
                WorkflowTransitionRegistryValidationError::DuplicateTransitionRuleId {
                    rule_id: rule.rule_id.clone(),
                },
            );
        }
        validate_transition_rule(rule, &mut errors);
    }

    let mut automation_ids = HashSet::new();
    for automation in &registry.queue_automation_rules {
        if !automation_ids.insert(automation.rule_id.clone()) {
            errors.push(
                WorkflowTransitionRegistryValidationError::DuplicateAutomationRuleId {
                    rule_id: automation.rule_id.clone(),
                },
            );
        }
        validate_queue_automation_rule(automation, registry, &mut errors);
    }

    let mut policy_ids = HashSet::new();
    for policy in &registry.executor_policies {
        if !policy_ids.insert(policy.policy_id.clone()) {
            errors.push(
                WorkflowTransitionRegistryValidationError::DuplicateExecutorPolicyId {
                    policy_id: policy.policy_id.clone(),
                },
            );
        }
        validate_executor_policy(policy, registry, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn preview_workflow_transition(
    registry: &WorkflowTransitionRegistryV1,
    rule_id: &str,
    actor_kind: &str,
) -> Result<DccTransitionPreviewV1, Vec<WorkflowTransitionRegistryValidationError>> {
    let Some(rule) = registry.transition_rule(rule_id) else {
        return Ok(DccTransitionPreviewV1 {
            preview_id: format!("dcc.preview.missing.{rule_id}"),
            panel_id: "workflow-transition-preview".to_string(),
            summary: "No registered transition rule exists for the requested mutation.".to_string(),
            posture: DccTransitionPreviewPosture::MissingRule,
            approval_boundary: ApprovalBoundary::None,
            authority_effect: AuthorityEffect::ProjectionOnly,
            primary_state_fields: preview_fields(),
            explanation:
                "Mutation must stop because legality cannot be inferred from views or prose."
                    .to_string(),
        });
    };

    let mut preview = rule.dcc_preview.clone();
    preview.authority_effect = AuthorityEffect::ProjectionOnly;

    if !rule
        .eligible_actor_kinds
        .iter()
        .any(|eligible| eligible == actor_kind)
    {
        preview.posture = DccTransitionPreviewPosture::ActorIneligible;
        preview.explanation = format!(
            "{actor_kind} is not eligible for transition rule {}",
            rule.rule_id
        );
        return Ok(preview);
    }

    if rule.approval_boundary != ApprovalBoundary::None {
        preview.posture = DccTransitionPreviewPosture::ApprovalRequired;
        preview.explanation = format!(
            "Transition {} is lawful only after {:?}",
            rule.rule_id, rule.approval_boundary
        );
        return Ok(preview);
    }

    if registry.queue_automation_rules.iter().any(|automation| {
        automation.transition_rule_id == rule.rule_id
            && automation.mode == QueueAutomationMode::Automatic
    }) {
        preview.posture = DccTransitionPreviewPosture::Automatic;
        preview.explanation = format!(
            "Transition {} can move automatically when its trigger fires.",
            rule.rule_id
        );
    } else {
        preview.posture = DccTransitionPreviewPosture::Lawful;
        preview.explanation = format!(
            "Transition {} is lawful for {actor_kind} through the governed action.",
            rule.rule_id
        );
    }

    Ok(preview)
}

fn validate_transition_rule(
    rule: &WorkflowTransitionRuleV1,
    errors: &mut Vec<WorkflowTransitionRegistryValidationError>,
) {
    require_transition_field(errors, rule, "rule_id", &rule.rule_id);
    require_transition_field(errors, rule, "workflow_family_id", &rule.workflow_family_id);
    require_transition_field(errors, rule, "from_state_id", &rule.from_state_id);
    require_transition_field(errors, rule, "to_state_id", &rule.to_state_id);
    require_transition_field(errors, rule, "governed_action_id", &rule.governed_action_id);
    require_transition_vec(
        errors,
        rule,
        "eligible_actor_kinds",
        &rule.eligible_actor_kinds,
    );
    require_transition_vec(errors, rule, "validation_hooks", &rule.validation_hooks);
    require_transition_field(
        errors,
        rule,
        "dcc_preview.panel_id",
        &rule.dcc_preview.panel_id,
    );
    require_transition_field(
        errors,
        rule,
        "dcc_preview.summary",
        &rule.dcc_preview.summary,
    );
    require_transition_vec(
        errors,
        rule,
        "dcc_preview.primary_state_fields",
        &rule.dcc_preview.primary_state_fields,
    );

    if !rule
        .folded_source_refs
        .iter()
        .any(|source| source.contains(FOLDED_WORKFLOW_TRANSITION_STUB))
    {
        errors.push(
            WorkflowTransitionRegistryValidationError::MissingTransitionField {
                rule_id: rule.rule_id.clone(),
                field: "folded_source_refs",
            },
        );
    }
}

fn validate_queue_automation_rule(
    automation: &QueueAutomationRuleV1,
    registry: &WorkflowTransitionRegistryV1,
    errors: &mut Vec<WorkflowTransitionRegistryValidationError>,
) {
    require_automation_field(errors, automation, "rule_id", &automation.rule_id);
    require_automation_field(
        errors,
        automation,
        "transition_rule_id",
        &automation.transition_rule_id,
    );
    require_automation_field(
        errors,
        automation,
        "dcc_preview_id",
        &automation.dcc_preview_id,
    );
    if automation.stable_source_ids.is_empty() {
        errors.push(
            WorkflowTransitionRegistryValidationError::MissingAutomationField {
                rule_id: automation.rule_id.clone(),
                field: "stable_source_ids",
            },
        );
    }

    if !automation.trigger_source_kind.is_authoritative() {
        errors.push(
            WorkflowTransitionRegistryValidationError::NonAuthoritativeAutomationSource {
                rule_id: automation.rule_id.clone(),
                source_kind: automation.trigger_source_kind,
            },
        );
    }

    match registry.transition_rule(&automation.transition_rule_id) {
        Some(rule)
            if automation.mode == QueueAutomationMode::Automatic
                && rule.approval_boundary != ApprovalBoundary::None =>
        {
            errors.push(
                WorkflowTransitionRegistryValidationError::AutomationCrossesApprovalBoundary {
                    rule_id: automation.rule_id.clone(),
                    transition_rule_id: automation.transition_rule_id.clone(),
                    approval_boundary: rule.approval_boundary,
                },
            );
        }
        Some(_) => {}
        None => errors.push(
            WorkflowTransitionRegistryValidationError::UnknownAutomationTransition {
                rule_id: automation.rule_id.clone(),
                transition_rule_id: automation.transition_rule_id.clone(),
            },
        ),
    }
}

fn validate_executor_policy(
    policy: &ExecutorEligibilityPolicyV1,
    registry: &WorkflowTransitionRegistryV1,
    errors: &mut Vec<WorkflowTransitionRegistryValidationError>,
) {
    require_policy_field(errors, policy, "policy_id", &policy.policy_id);
    require_policy_field(
        errors,
        policy,
        "workflow_family_id",
        &policy.workflow_family_id,
    );
    require_policy_field(errors, policy, "actor_kind", &policy.actor_kind);
    require_policy_field(
        errors,
        policy,
        "executor_profile_id",
        &policy.executor_profile_id,
    );

    if policy.eligible_transition_rule_ids.is_empty() {
        errors.push(
            WorkflowTransitionRegistryValidationError::MissingExecutorPolicyField {
                policy_id: policy.policy_id.clone(),
                field: "eligible_transition_rule_ids",
            },
        );
    }

    if policy.required_capabilities.is_empty() {
        errors.push(
            WorkflowTransitionRegistryValidationError::MissingExecutorPolicyField {
                policy_id: policy.policy_id.clone(),
                field: "required_capabilities",
            },
        );
    }

    for transition_rule_id in &policy.eligible_transition_rule_ids {
        if registry.transition_rule(transition_rule_id).is_none() {
            errors.push(
                WorkflowTransitionRegistryValidationError::UnknownExecutorTransition {
                    policy_id: policy.policy_id.clone(),
                    transition_rule_id: transition_rule_id.clone(),
                },
            );
        }
    }
}

fn transition_rule(
    rule_id: &str,
    mutation_kind: WorkflowMutationKind,
    from_state_id: &str,
    to_state_id: &str,
    governed_action_id: &str,
    eligible_actor_kinds: &[&str],
    approval_boundary: ApprovalBoundary,
    validation_hooks: &[&str],
) -> WorkflowTransitionRuleV1 {
    WorkflowTransitionRuleV1 {
        rule_id: rule_id.to_string(),
        workflow_family_id: "kernel002.software_delivery".to_string(),
        mutation_kind,
        from_state_id: from_state_id.to_string(),
        to_state_id: to_state_id.to_string(),
        governed_action_id: governed_action_id.to_string(),
        eligible_actor_kinds: eligible_actor_kinds
            .iter()
            .map(|actor| (*actor).to_string())
            .collect(),
        approval_boundary,
        validation_hooks: validation_hooks
            .iter()
            .map(|hook| (*hook).to_string())
            .collect(),
        dcc_preview: DccTransitionPreviewV1 {
            preview_id: format!("dcc.preview.{rule_id}"),
            panel_id: "workflow-transition-preview".to_string(),
            summary: format!("{from_state_id} -> {to_state_id} through {governed_action_id}"),
            posture: DccTransitionPreviewPosture::ViewOnly,
            approval_boundary,
            authority_effect: AuthorityEffect::ProjectionOnly,
            primary_state_fields: preview_fields(),
            explanation:
                "Preview posture is resolved against actor eligibility and approval boundary."
                    .to_string(),
        },
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Workflow-Transition-Automation-Registry-v1.contract.json"
                .to_string(),
            ".GOV/task_packets/stubs/WP-1-Workflow-Transition-Automation-Registry-v1.md"
                .to_string(),
        ],
    }
}

fn queue_automation_rule(
    rule_id: &str,
    trigger_kind: QueueAutomationTriggerKind,
    trigger_source_kind: QueueAutomationSourceKind,
    transition_rule_id: &str,
    mode: QueueAutomationMode,
    stable_source_ids: &[&str],
) -> QueueAutomationRuleV1 {
    QueueAutomationRuleV1 {
        rule_id: rule_id.to_string(),
        trigger_kind,
        trigger_source_kind,
        transition_rule_id: transition_rule_id.to_string(),
        mode,
        stable_source_ids: stable_source_ids
            .iter()
            .map(|source_id| (*source_id).to_string())
            .collect(),
        dcc_preview_id: format!("dcc.preview.{transition_rule_id}"),
    }
}

fn executor_policy(
    policy_id: &str,
    actor_kind: &str,
    executor_profile_id: &str,
    eligible_transition_rule_ids: &[&str],
    required_capabilities: &[&str],
    local_small_model_allowed: bool,
) -> ExecutorEligibilityPolicyV1 {
    ExecutorEligibilityPolicyV1 {
        policy_id: policy_id.to_string(),
        workflow_family_id: "kernel002.software_delivery".to_string(),
        actor_kind: actor_kind.to_string(),
        executor_profile_id: executor_profile_id.to_string(),
        eligible_transition_rule_ids: eligible_transition_rule_ids
            .iter()
            .map(|rule_id| (*rule_id).to_string())
            .collect(),
        required_capabilities: required_capabilities
            .iter()
            .map(|capability| (*capability).to_string())
            .collect(),
        local_small_model_allowed,
    }
}

fn preview_fields() -> Vec<String> {
    [
        "rule_id",
        "from_state_id",
        "to_state_id",
        "governed_action_id",
        "eligible_actor_kinds",
        "approval_boundary",
    ]
    .iter()
    .map(|field| (*field).to_string())
    .collect()
}

fn require_transition_field(
    errors: &mut Vec<WorkflowTransitionRegistryValidationError>,
    rule: &WorkflowTransitionRuleV1,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(
            WorkflowTransitionRegistryValidationError::MissingTransitionField {
                rule_id: rule.rule_id.clone(),
                field,
            },
        );
    }
}

fn require_transition_vec<T>(
    errors: &mut Vec<WorkflowTransitionRegistryValidationError>,
    rule: &WorkflowTransitionRuleV1,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(
            WorkflowTransitionRegistryValidationError::MissingTransitionField {
                rule_id: rule.rule_id.clone(),
                field,
            },
        );
    }
}

fn require_automation_field(
    errors: &mut Vec<WorkflowTransitionRegistryValidationError>,
    automation: &QueueAutomationRuleV1,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(
            WorkflowTransitionRegistryValidationError::MissingAutomationField {
                rule_id: automation.rule_id.clone(),
                field,
            },
        );
    }
}

fn require_policy_field(
    errors: &mut Vec<WorkflowTransitionRegistryValidationError>,
    policy: &ExecutorEligibilityPolicyV1,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(
            WorkflowTransitionRegistryValidationError::MissingExecutorPolicyField {
                policy_id: policy.policy_id.clone(),
                field,
            },
        );
    }
}

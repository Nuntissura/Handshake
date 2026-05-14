use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{
    action_envelope::AuthorityEffect, role_mailbox_contract::RoleMailboxAuthorityBoundary,
};

pub const FOLDED_ROLE_MAILBOX_TRIAGE_QUEUE_STUB_ID: &str =
    "WP-1-Role-Mailbox-Triage-Queue-Controls-v1";

const REQUIRED_REMEDIATION_ACTIONS: [RemediationActionKind; 7] = [
    RemediationActionKind::Reminder,
    RemediationActionKind::Unsnooze,
    RemediationActionKind::RetryDelivery,
    RemediationActionKind::Reroute,
    RemediationActionKind::Archive,
    RemediationActionKind::RequestTranscription,
    RemediationActionKind::GovernedFollowUp,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriageQueueState {
    Waiting,
    ReminderDue,
    Snoozed,
    Expired,
    DeadLetterRemediation,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SnoozePosture {
    NotSnoozed,
    Snoozed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpiryPosture {
    NotExpired,
    DueSoon,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeadLetterDisposition {
    None,
    RetryDelivery,
    Reroute,
    Archive,
    RequestTranscription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PressureLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RemediationActionKind {
    Reminder,
    Unsnooze,
    RetryDelivery,
    Reroute,
    Archive,
    RequestTranscription,
    GovernedFollowUp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderScheduleV1 {
    pub schedule_id: String,
    pub cadence_minutes: u32,
    pub next_reminder_at: String,
    pub expires_at: String,
    pub snoozed_until: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardPressureOverlayV1 {
    pub overlay_id: String,
    pub linked_task_board_row_id: String,
    pub pressure_level: PressureLevel,
    pub waiting_reason: String,
    pub projection_only: bool,
    pub mutates_task_board: bool,
    pub source_field_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationActionV1 {
    pub action_id: String,
    pub kind: RemediationActionKind,
    pub boundary: RoleMailboxAuthorityBoundary,
    pub preview_required: bool,
    pub kernel_action_id: Option<String>,
    pub field_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxTriageQueueItemV1 {
    pub thread_id: String,
    pub linked_record_ids: Vec<String>,
    pub queue_state: TriageQueueState,
    pub reminder_schedule: ReminderScheduleV1,
    pub snooze_posture: SnoozePosture,
    pub expiry_posture: ExpiryPosture,
    pub dead_letter_disposition: DeadLetterDisposition,
    pub remediation_actions: Vec<RemediationActionV1>,
    pub task_board_pressure: TaskBoardPressureOverlayV1,
    pub work_packet_followup_summary: String,
    pub locus_join_refs: Vec<String>,
    pub compact_summary: String,
    pub field_backed_projection: bool,
    pub transcript_parsing_required: bool,
    pub mailbox_state_authoritative_for_linked_work: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxTriageQueueControlsV1 {
    pub schema_id: String,
    pub controls_id: String,
    pub folded_stub_id: String,
    pub queue_items: Vec<RoleMailboxTriageQueueItemV1>,
    pub compact_summary_first: bool,
    pub task_board_pressure_authoritative: bool,
    pub work_packet_followup_authoritative: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxRemediationActionPreviewV1 {
    pub schema_id: String,
    pub thread_id: String,
    pub action_id: String,
    pub action_kind: RemediationActionKind,
    pub boundary: RoleMailboxAuthorityBoundary,
    pub authority_effect: AuthorityEffect,
    pub preview_required: bool,
    pub kernel_action_id: Option<String>,
    pub field_paths: Vec<String>,
    pub mutates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxTriageQueueValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_role_mailbox_triage_queue_controls(
    controls: &RoleMailboxTriageQueueControlsV1,
) -> Result<(), Vec<RoleMailboxTriageQueueValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &controls.schema_id);
    require_non_empty(&mut errors, "controls_id", &controls.controls_id);
    require_non_empty(&mut errors, "folded_stub_id", &controls.folded_stub_id);
    require_vec(&mut errors, "queue_items", &controls.queue_items);
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

    if controls.folded_stub_id != FOLDED_ROLE_MAILBOX_TRIAGE_QUEUE_STUB_ID {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "folded_stub_id",
            message: "triage queue controls must bind the folded Role Mailbox triage queue stub",
        });
    }
    if !contains_text(
        &controls.folded_source_refs,
        FOLDED_ROLE_MAILBOX_TRIAGE_QUEUE_STUB_ID,
    ) {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "folded_source_refs",
            message: "folded Role Mailbox triage queue source must be preserved",
        });
    }
    if !controls.compact_summary_first {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "compact_summary_first",
            message: "triage queue inspection must be compact-summary-first",
        });
    }
    if controls.task_board_pressure_authoritative {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "task_board_pressure_authoritative",
            message: "Task Board pressure overlays are projections and must not become authority",
        });
    }
    if controls.work_packet_followup_authoritative {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "work_packet_followup_authoritative",
            message:
                "Work Packet follow-up summaries are projections and must not become authority",
        });
    }

    validate_refs(&mut errors, controls);
    validate_queue_items(&mut errors, &controls.queue_items);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_task_board_pressure_overlays(
    controls: &RoleMailboxTriageQueueControlsV1,
) -> Result<Vec<TaskBoardPressureOverlayV1>, Vec<RoleMailboxTriageQueueValidationError>> {
    validate_role_mailbox_triage_queue_controls(controls)?;

    Ok(controls
        .queue_items
        .iter()
        .map(|item| item.task_board_pressure.clone())
        .collect())
}

pub fn preview_role_mailbox_remediation_action(
    controls: &RoleMailboxTriageQueueControlsV1,
    thread_id: &str,
    action_kind: RemediationActionKind,
) -> Result<RoleMailboxRemediationActionPreviewV1, Vec<RoleMailboxTriageQueueValidationError>> {
    validate_role_mailbox_triage_queue_controls(controls)?;

    let Some(item) = controls
        .queue_items
        .iter()
        .find(|item| item.thread_id == thread_id)
    else {
        return Err(vec![RoleMailboxTriageQueueValidationError {
            field: "thread_id",
            message: "requested Role Mailbox triage queue thread is not registered",
        }]);
    };
    let Some(action) = item
        .remediation_actions
        .iter()
        .find(|action| action.kind == action_kind)
    else {
        return Err(vec![RoleMailboxTriageQueueValidationError {
            field: "remediation_actions.kind",
            message: "requested remediation action kind is not registered for the thread",
        }]);
    };

    Ok(RoleMailboxRemediationActionPreviewV1 {
        schema_id: "hsk.kernel.role_mailbox_remediation_action_preview@1".to_string(),
        thread_id: item.thread_id.clone(),
        action_id: action.action_id.clone(),
        action_kind: action.kind,
        boundary: action.boundary,
        authority_effect: authority_effect_for_boundary(action.boundary),
        preview_required: action.preview_required,
        kernel_action_id: action.kernel_action_id.clone(),
        field_paths: action.field_paths.clone(),
        mutates_authority: false,
    })
}

fn validate_refs(
    errors: &mut Vec<RoleMailboxTriageQueueValidationError>,
    controls: &RoleMailboxTriageQueueControlsV1,
) {
    for required_ref in [
        "kernel.role_mailbox_contract",
        "kernel.role_mailbox_loop_control",
        "kernel.locus_work_tracking",
        "kernel.workflow_transition_registry",
        "kernel.dcc_layout_projection_registry",
    ] {
        if !contains_exact(&controls.product_authority_refs, required_ref) {
            errors.push(RoleMailboxTriageQueueValidationError {
                field: "product_authority_refs",
                message: "triage queue controls must cite Role Mailbox, loop-control, Locus, workflow, and DCC projection authority refs",
            });
        }
    }
}

fn validate_queue_items(
    errors: &mut Vec<RoleMailboxTriageQueueValidationError>,
    items: &[RoleMailboxTriageQueueItemV1],
) {
    let mut thread_ids = HashSet::new();
    for item in items {
        if !thread_ids.insert(item.thread_id.as_str()) {
            errors.push(RoleMailboxTriageQueueValidationError {
                field: "thread_id",
                message: "triage queue thread ids must be unique",
            });
        }

        require_non_empty(errors, "thread_id", &item.thread_id);
        require_vec(errors, "linked_record_ids", &item.linked_record_ids);
        require_non_empty(
            errors,
            "work_packet_followup_summary",
            &item.work_packet_followup_summary,
        );
        require_vec(errors, "locus_join_refs", &item.locus_join_refs);
        require_non_empty(errors, "compact_summary", &item.compact_summary);
        require_vec(errors, "remediation_actions", &item.remediation_actions);

        if !item.field_backed_projection {
            errors.push(RoleMailboxTriageQueueValidationError {
                field: "field_backed_projection",
                message: "triage queue state must be backed by explicit fields",
            });
        }
        if item.transcript_parsing_required {
            errors.push(RoleMailboxTriageQueueValidationError {
                field: "transcript_parsing_required",
                message: "triage queue controls must not require transcript parsing",
            });
        }
        if item.mailbox_state_authoritative_for_linked_work {
            errors.push(RoleMailboxTriageQueueValidationError {
                field: "mailbox_state_authoritative_for_linked_work",
                message: "mailbox triage state must not become authority for linked work",
            });
        }

        validate_reminder_schedule(errors, item);
        validate_task_board_pressure(errors, &item.task_board_pressure);
        validate_dead_letter_posture(errors, item);
        validate_remediation_actions(errors, &item.remediation_actions);
    }
}

fn validate_reminder_schedule(
    errors: &mut Vec<RoleMailboxTriageQueueValidationError>,
    item: &RoleMailboxTriageQueueItemV1,
) {
    require_non_empty(
        errors,
        "reminder_schedule.schedule_id",
        &item.reminder_schedule.schedule_id,
    );
    require_non_empty(
        errors,
        "reminder_schedule.next_reminder_at",
        &item.reminder_schedule.next_reminder_at,
    );
    require_non_empty(
        errors,
        "reminder_schedule.expires_at",
        &item.reminder_schedule.expires_at,
    );

    if !item.reminder_schedule.enabled {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "reminder_schedule.enabled",
            message: "triage queue reminder schedules must be explicitly enabled before projection",
        });
    }
    if item.reminder_schedule.cadence_minutes == 0 {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "reminder_schedule.cadence_minutes",
            message: "reminder cadence must be greater than zero",
        });
    }
    if item.snooze_posture == SnoozePosture::Snoozed
        && item
            .reminder_schedule
            .snoozed_until
            .as_deref()
            .is_none_or(str::is_empty)
    {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "reminder_schedule.snoozed_until",
            message: "snoozed triage queue items require a snoozed-until field",
        });
    }
}

fn validate_task_board_pressure(
    errors: &mut Vec<RoleMailboxTriageQueueValidationError>,
    overlay: &TaskBoardPressureOverlayV1,
) {
    require_non_empty(
        errors,
        "task_board_pressure.overlay_id",
        &overlay.overlay_id,
    );
    require_non_empty(
        errors,
        "task_board_pressure.linked_task_board_row_id",
        &overlay.linked_task_board_row_id,
    );
    require_non_empty(
        errors,
        "task_board_pressure.waiting_reason",
        &overlay.waiting_reason,
    );
    require_vec(
        errors,
        "task_board_pressure.source_field_refs",
        &overlay.source_field_refs,
    );

    if !overlay.projection_only {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "task_board_pressure.projection_only",
            message: "Task Board pressure overlays must be projection-only",
        });
    }
    if overlay.mutates_task_board {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "task_board_pressure.mutates_task_board",
            message: "Task Board pressure overlays must not mutate Task Board state",
        });
    }
}

fn validate_dead_letter_posture(
    errors: &mut Vec<RoleMailboxTriageQueueValidationError>,
    item: &RoleMailboxTriageQueueItemV1,
) {
    if item.expiry_posture == ExpiryPosture::Expired
        && !matches!(
            item.queue_state,
            TriageQueueState::Expired | TriageQueueState::DeadLetterRemediation
        )
    {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "expiry_posture",
            message: "expired triage items must be projected as expired or dead-letter remediation",
        });
    }
    if item.queue_state == TriageQueueState::DeadLetterRemediation
        && item.dead_letter_disposition == DeadLetterDisposition::None
    {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "dead_letter_disposition",
            message: "dead-letter remediation requires an explicit disposition",
        });
    }
    if item.queue_state != TriageQueueState::DeadLetterRemediation
        && item.dead_letter_disposition != DeadLetterDisposition::None
    {
        errors.push(RoleMailboxTriageQueueValidationError {
            field: "dead_letter_disposition",
            message: "dead-letter disposition must align with dead-letter remediation state",
        });
    }
}

fn validate_remediation_actions(
    errors: &mut Vec<RoleMailboxTriageQueueValidationError>,
    actions: &[RemediationActionV1],
) {
    let mut action_ids = HashSet::new();
    let mut action_kinds = HashSet::new();

    for action in actions {
        if !action_ids.insert(action.action_id.as_str()) {
            errors.push(RoleMailboxTriageQueueValidationError {
                field: "remediation_actions.action_id",
                message: "remediation action ids must be unique",
            });
        }
        action_kinds.insert(action.kind);
        require_non_empty(errors, "remediation_actions.action_id", &action.action_id);
        require_vec(
            errors,
            "remediation_actions.field_paths",
            &action.field_paths,
        );

        if !action.preview_required {
            errors.push(RoleMailboxTriageQueueValidationError {
                field: "remediation_actions.preview_required",
                message: "triage queue remediation actions must be previewed before execution",
            });
        }
        if expected_boundary(action.kind) != action.boundary {
            errors.push(RoleMailboxTriageQueueValidationError {
                field: "remediation_actions.boundary",
                message: "remediation action boundary does not match triage action law",
            });
        }

        match action.boundary {
            RoleMailboxAuthorityBoundary::MailboxLocal => {
                if action.kernel_action_id.is_some() {
                    errors.push(RoleMailboxTriageQueueValidationError {
                        field: "remediation_actions.kernel_action_id",
                        message:
                            "mailbox-local remediation actions must not carry kernel action ids",
                    });
                }
            }
            RoleMailboxAuthorityBoundary::GovernedActionRequired => {
                if action.kernel_action_id.as_deref().is_none_or(str::is_empty) {
                    errors.push(RoleMailboxTriageQueueValidationError {
                        field: "remediation_actions.kernel_action_id",
                        message: "governed remediation actions require a catalog action id",
                    });
                }
            }
            RoleMailboxAuthorityBoundary::TranscriptionRequired => {
                if action.kernel_action_id.is_some() {
                    errors.push(RoleMailboxTriageQueueValidationError {
                        field: "remediation_actions.kernel_action_id",
                        message:
                            "transcription remediation actions must not impersonate kernel actions",
                    });
                }
            }
        }
    }

    for required_kind in REQUIRED_REMEDIATION_ACTIONS {
        if !action_kinds.contains(&required_kind) {
            errors.push(RoleMailboxTriageQueueValidationError {
                field: "remediation_actions.kind",
                message: "reminder, unsnooze, retry, reroute, archive, transcription, and governed follow-up actions are required",
            });
        }
    }
}

fn expected_boundary(kind: RemediationActionKind) -> RoleMailboxAuthorityBoundary {
    match kind {
        RemediationActionKind::Reminder
        | RemediationActionKind::Unsnooze
        | RemediationActionKind::RetryDelivery
        | RemediationActionKind::Archive => RoleMailboxAuthorityBoundary::MailboxLocal,
        RemediationActionKind::Reroute | RemediationActionKind::GovernedFollowUp => {
            RoleMailboxAuthorityBoundary::GovernedActionRequired
        }
        RemediationActionKind::RequestTranscription => {
            RoleMailboxAuthorityBoundary::TranscriptionRequired
        }
    }
}

fn authority_effect_for_boundary(boundary: RoleMailboxAuthorityBoundary) -> AuthorityEffect {
    match boundary {
        RoleMailboxAuthorityBoundary::MailboxLocal => AuthorityEffect::None,
        RoleMailboxAuthorityBoundary::GovernedActionRequired => {
            AuthorityEffect::PrePromotionEvidenceOnly
        }
        RoleMailboxAuthorityBoundary::TranscriptionRequired => {
            AuthorityEffect::PrePromotionEvidenceOnly
        }
    }
}

fn require_non_empty(
    errors: &mut Vec<RoleMailboxTriageQueueValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(RoleMailboxTriageQueueValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<RoleMailboxTriageQueueValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(RoleMailboxTriageQueueValidationError {
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

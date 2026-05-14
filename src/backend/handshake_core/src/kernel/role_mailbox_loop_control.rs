use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::action_envelope::{ApprovalPosture, AuthorityEffect};

pub const FOLDED_ROLE_MAILBOX_LOOP_CONTROL_STUB_ID: &str =
    "WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1";

const REQUIRED_PAYLOAD_KINDS: [MicroTaskMailboxPayloadKind; 4] = [
    MicroTaskMailboxPayloadKind::MicroTaskFeedback,
    MicroTaskMailboxPayloadKind::MicroTaskVerificationNeeded,
    MicroTaskMailboxPayloadKind::MicroTaskEscalation,
    MicroTaskMailboxPayloadKind::MicroTaskCompletionReport,
];

const REQUIRED_LOCAL_SMALL_MODEL_FIELDS: [&str; 6] = [
    "compact_summary",
    "retry_budget",
    "verifier_outcome",
    "escalation_target",
    "completion_transcription_posture",
    "dead_letter_posture",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MicroTaskLoopState {
    WaitingForExecution,
    Running,
    VerificationNeeded,
    Retrying,
    Escalated,
    Completed,
    DeadLettered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MicroTaskVerifierOutcomeKind {
    Passed,
    FailedRetryable,
    NeedsEvidence,
    RequiresEscalation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MicroTaskMailboxPayloadKind {
    MicroTaskFeedback,
    MicroTaskVerificationNeeded,
    MicroTaskEscalation,
    MicroTaskCompletionReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompletionReportTranscriptionPosture {
    NotRequired,
    Pending,
    Transcribed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MicroTaskLoopDeadLetterPosture {
    None,
    Retryable,
    RequiresRemediation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopActionKind {
    Retry,
    Escalate,
    Complete,
    RemediateDeadLetter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskRetryBudgetV1 {
    pub max_attempts: u32,
    pub attempts_used: u32,
    pub remaining_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskVerifierOutcomeV1 {
    pub outcome_id: String,
    pub kind: MicroTaskVerifierOutcomeKind,
    pub verifier_session_id: String,
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskLoopCheckpointV1 {
    pub checkpoint_id: String,
    pub sequence: u64,
    pub wp_id: String,
    pub mt_id: String,
    pub role_mailbox_thread_id: String,
    pub loop_state: MicroTaskLoopState,
    pub retry_budget: MicroTaskRetryBudgetV1,
    pub latest_verifier_outcome_id: Option<String>,
    pub escalation_target: Option<String>,
    pub completion_report_id: Option<String>,
    pub completion_transcription_posture: CompletionReportTranscriptionPosture,
    pub dead_letter_posture: MicroTaskLoopDeadLetterPosture,
    pub compact_summary: String,
    pub replay_ref: String,
    pub message_payload_kinds: Vec<MicroTaskMailboxPayloadKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskLoopControlV1 {
    pub schema_id: String,
    pub control_id: String,
    pub folded_stub_id: String,
    pub checkpoints: Vec<MicroTaskLoopCheckpointV1>,
    pub verifier_outcomes: Vec<MicroTaskVerifierOutcomeV1>,
    pub compact_summary_first: bool,
    pub transcript_replay_required: bool,
    pub work_packet_projection_authoritative: bool,
    pub task_board_projection_authoritative: bool,
    pub local_small_model_fields: Vec<String>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicroTaskLoopSummaryProjectionV1 {
    pub schema_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub role_mailbox_thread_id: String,
    pub loop_state: MicroTaskLoopState,
    pub remaining_retries: u32,
    pub verifier_outcome_kind: Option<MicroTaskVerifierOutcomeKind>,
    pub escalation_target: Option<String>,
    pub completion_report_id: Option<String>,
    pub completion_transcription_posture: CompletionReportTranscriptionPosture,
    pub dead_letter_posture: MicroTaskLoopDeadLetterPosture,
    pub compact_summary: String,
    pub replayable_from_checkpoint: bool,
    pub mutates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicroTaskLoopActionPreviewV1 {
    pub schema_id: String,
    pub mt_id: String,
    pub action_kind: LoopActionKind,
    pub kernel_action_id: Option<String>,
    pub authority_effect: AuthorityEffect,
    pub approval_posture: ApprovalPosture,
    pub preview_required: bool,
    pub mutates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicroTaskLoopControlValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_role_mailbox_micro_task_loop_control(
    control: &MicroTaskLoopControlV1,
) -> Result<(), Vec<MicroTaskLoopControlValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &control.schema_id);
    require_non_empty(&mut errors, "control_id", &control.control_id);
    require_non_empty(&mut errors, "folded_stub_id", &control.folded_stub_id);
    require_vec(&mut errors, "checkpoints", &control.checkpoints);
    require_vec(&mut errors, "verifier_outcomes", &control.verifier_outcomes);
    require_vec(
        &mut errors,
        "local_small_model_fields",
        &control.local_small_model_fields,
    );
    require_vec(
        &mut errors,
        "product_authority_refs",
        &control.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &control.folded_source_refs,
    );

    if control.folded_stub_id != FOLDED_ROLE_MAILBOX_LOOP_CONTROL_STUB_ID {
        errors.push(MicroTaskLoopControlValidationError {
            field: "folded_stub_id",
            message: "loop control must bind the folded Role Mailbox Micro-Task loop-control stub",
        });
    }

    if !contains_text(
        &control.folded_source_refs,
        FOLDED_ROLE_MAILBOX_LOOP_CONTROL_STUB_ID,
    ) {
        errors.push(MicroTaskLoopControlValidationError {
            field: "folded_source_refs",
            message: "folded Role Mailbox Micro-Task loop-control source must be preserved",
        });
    }

    if !control.compact_summary_first {
        errors.push(MicroTaskLoopControlValidationError {
            field: "compact_summary_first",
            message: "mailbox-linked loop inspection must be compact-summary-first",
        });
    }
    if control.transcript_replay_required {
        errors.push(MicroTaskLoopControlValidationError {
            field: "transcript_replay_required",
            message: "loop state must be replayable from checkpoints without transcript replay",
        });
    }
    if control.work_packet_projection_authoritative {
        errors.push(MicroTaskLoopControlValidationError {
            field: "work_packet_projection_authoritative",
            message:
                "Work Packet projections may explain loop posture but not become loop authority",
        });
    }
    if control.task_board_projection_authoritative {
        errors.push(MicroTaskLoopControlValidationError {
            field: "task_board_projection_authoritative",
            message:
                "Task Board projections may explain loop posture but not become loop authority",
        });
    }

    validate_refs(&mut errors, control);
    validate_local_small_model_fields(&mut errors, &control.local_small_model_fields);
    validate_verifier_outcomes(&mut errors, &control.verifier_outcomes);
    validate_checkpoints(control, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn latest_micro_task_loop_checkpoint<'a>(
    control: &'a MicroTaskLoopControlV1,
    mt_id: &str,
) -> Result<&'a MicroTaskLoopCheckpointV1, Vec<MicroTaskLoopControlValidationError>> {
    validate_role_mailbox_micro_task_loop_control(control)?;
    control
        .checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.mt_id == mt_id)
        .max_by_key(|checkpoint| checkpoint.sequence)
        .ok_or_else(|| {
            vec![MicroTaskLoopControlValidationError {
                field: "mt_id",
                message: "no checkpoint exists for requested Micro-Task",
            }]
        })
}

pub fn project_micro_task_loop_summary(
    control: &MicroTaskLoopControlV1,
    mt_id: &str,
) -> Result<MicroTaskLoopSummaryProjectionV1, Vec<MicroTaskLoopControlValidationError>> {
    let latest = latest_micro_task_loop_checkpoint(control, mt_id)?;
    let verifier_outcomes = verifier_outcome_by_id(&control.verifier_outcomes);
    let verifier_outcome_kind = latest
        .latest_verifier_outcome_id
        .as_deref()
        .and_then(|outcome_id| verifier_outcomes.get(outcome_id))
        .map(|outcome| outcome.kind);

    Ok(MicroTaskLoopSummaryProjectionV1 {
        schema_id: "hsk.kernel.micro_task_loop_summary_projection@1".to_string(),
        wp_id: latest.wp_id.clone(),
        mt_id: latest.mt_id.clone(),
        role_mailbox_thread_id: latest.role_mailbox_thread_id.clone(),
        loop_state: latest.loop_state,
        remaining_retries: latest.retry_budget.remaining_attempts,
        verifier_outcome_kind,
        escalation_target: latest.escalation_target.clone(),
        completion_report_id: latest.completion_report_id.clone(),
        completion_transcription_posture: latest.completion_transcription_posture,
        dead_letter_posture: latest.dead_letter_posture,
        compact_summary: latest.compact_summary.clone(),
        replayable_from_checkpoint: !latest.replay_ref.trim().is_empty(),
        mutates_authority: false,
    })
}

pub fn preview_micro_task_loop_action(
    control: &MicroTaskLoopControlV1,
    mt_id: &str,
    action_kind: LoopActionKind,
) -> Result<MicroTaskLoopActionPreviewV1, Vec<MicroTaskLoopControlValidationError>> {
    let latest = latest_micro_task_loop_checkpoint(control, mt_id)?;

    let kernel_action_id = match action_kind {
        LoopActionKind::Retry
        | LoopActionKind::Escalate
        | LoopActionKind::Complete
        | LoopActionKind::RemediateDeadLetter => Some("kernel.workflow_transition.preview"),
    };

    Ok(MicroTaskLoopActionPreviewV1 {
        schema_id: "hsk.kernel.micro_task_loop_action_preview@1".to_string(),
        mt_id: latest.mt_id.clone(),
        action_kind,
        kernel_action_id: kernel_action_id.map(str::to_string),
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        preview_required: true,
        mutates_authority: false,
    })
}

fn validate_refs(
    errors: &mut Vec<MicroTaskLoopControlValidationError>,
    control: &MicroTaskLoopControlV1,
) {
    for required_ref in [
        "kernel.role_mailbox_contract",
        "kernel.locus_work_tracking",
        "kernel.workflow_transition_registry",
        "kernel.dcc_layout_projection_registry",
    ] {
        if !contains_exact(&control.product_authority_refs, required_ref) {
            errors.push(MicroTaskLoopControlValidationError {
                field: "product_authority_refs",
                message: "loop control must cite Role Mailbox, Locus, workflow transition, and DCC projection authority refs",
            });
        }
    }
}

fn validate_local_small_model_fields(
    errors: &mut Vec<MicroTaskLoopControlValidationError>,
    fields: &[String],
) {
    for required_field in REQUIRED_LOCAL_SMALL_MODEL_FIELDS {
        if !contains_exact(fields, required_field) {
            errors.push(MicroTaskLoopControlValidationError {
                field: "local_small_model_fields",
                message: "local-small-model loop projection must expose compact summary, retry, verifier, escalation, completion, and dead-letter fields",
            });
        }
    }
}

fn validate_verifier_outcomes(
    errors: &mut Vec<MicroTaskLoopControlValidationError>,
    outcomes: &[MicroTaskVerifierOutcomeV1],
) {
    let mut outcome_ids = HashSet::new();
    for outcome in outcomes {
        if !outcome_ids.insert(outcome.outcome_id.as_str()) {
            errors.push(MicroTaskLoopControlValidationError {
                field: "verifier_outcomes.outcome_id",
                message: "verifier outcome ids must be unique",
            });
        }
        require_non_empty(errors, "verifier_outcomes.outcome_id", &outcome.outcome_id);
        require_non_empty(
            errors,
            "verifier_outcomes.verifier_session_id",
            &outcome.verifier_session_id,
        );
        require_non_empty(errors, "verifier_outcomes.summary", &outcome.summary);
        require_non_empty(
            errors,
            "verifier_outcomes.next_action",
            &outcome.next_action,
        );
        require_vec(
            errors,
            "verifier_outcomes.evidence_refs",
            &outcome.evidence_refs,
        );
    }
}

fn validate_checkpoints(
    control: &MicroTaskLoopControlV1,
    errors: &mut Vec<MicroTaskLoopControlValidationError>,
) {
    let verifier_outcomes = verifier_outcome_by_id(&control.verifier_outcomes);
    let mut checkpoint_ids = HashSet::new();
    let mut sequence_by_mt: HashMap<&str, HashSet<u64>> = HashMap::new();

    for checkpoint in &control.checkpoints {
        if !checkpoint_ids.insert(checkpoint.checkpoint_id.as_str()) {
            errors.push(MicroTaskLoopControlValidationError {
                field: "checkpoints.checkpoint_id",
                message: "checkpoint ids must be unique",
            });
        }
        if !sequence_by_mt
            .entry(checkpoint.mt_id.as_str())
            .or_default()
            .insert(checkpoint.sequence)
        {
            errors.push(MicroTaskLoopControlValidationError {
                field: "checkpoints.sequence",
                message: "checkpoint sequence must be unique per Micro-Task",
            });
        }
        require_non_empty(
            errors,
            "checkpoints.checkpoint_id",
            &checkpoint.checkpoint_id,
        );
        require_non_empty(errors, "checkpoints.wp_id", &checkpoint.wp_id);
        require_non_empty(errors, "checkpoints.mt_id", &checkpoint.mt_id);
        require_non_empty(
            errors,
            "checkpoints.role_mailbox_thread_id",
            &checkpoint.role_mailbox_thread_id,
        );
        require_non_empty(
            errors,
            "checkpoints.compact_summary",
            &checkpoint.compact_summary,
        );
        require_non_empty(errors, "checkpoints.replay_ref", &checkpoint.replay_ref);
        require_vec(
            errors,
            "checkpoints.message_payload_kinds",
            &checkpoint.message_payload_kinds,
        );

        if checkpoint.sequence == 0 {
            errors.push(MicroTaskLoopControlValidationError {
                field: "checkpoints.sequence",
                message: "checkpoint sequence must be greater than zero",
            });
        }
        validate_retry_budget(errors, checkpoint);
        validate_payload_kinds(errors, &checkpoint.message_payload_kinds);
        validate_checkpoint_state(errors, checkpoint, &verifier_outcomes);
    }
}

fn validate_retry_budget(
    errors: &mut Vec<MicroTaskLoopControlValidationError>,
    checkpoint: &MicroTaskLoopCheckpointV1,
) {
    if checkpoint.retry_budget.max_attempts == 0 {
        errors.push(MicroTaskLoopControlValidationError {
            field: "retry_budget.max_attempts",
            message: "retry budget must declare at least one attempt",
        });
    }
    if checkpoint.retry_budget.attempts_used > checkpoint.retry_budget.max_attempts {
        errors.push(MicroTaskLoopControlValidationError {
            field: "retry_budget.attempts_used",
            message: "attempts used must not exceed max attempts",
        });
    }
    if checkpoint.retry_budget.remaining_attempts
        != checkpoint
            .retry_budget
            .max_attempts
            .saturating_sub(checkpoint.retry_budget.attempts_used)
    {
        errors.push(MicroTaskLoopControlValidationError {
            field: "retry_budget.remaining_attempts",
            message: "remaining attempts must equal max attempts minus attempts used",
        });
    }
    if checkpoint.loop_state == MicroTaskLoopState::Retrying
        && checkpoint.retry_budget.remaining_attempts == 0
    {
        errors.push(MicroTaskLoopControlValidationError {
            field: "retry_budget.remaining_attempts",
            message: "retrying state requires remaining retry budget",
        });
    }
}

fn validate_payload_kinds(
    errors: &mut Vec<MicroTaskLoopControlValidationError>,
    payload_kinds: &[MicroTaskMailboxPayloadKind],
) {
    let observed: HashSet<MicroTaskMailboxPayloadKind> = payload_kinds.iter().copied().collect();
    for required_kind in REQUIRED_PAYLOAD_KINDS {
        if !observed.contains(&required_kind) {
            errors.push(MicroTaskLoopControlValidationError {
                field: "message_payload_kinds",
                message: "feedback, verification-needed, escalation, and completion-report payload kinds are required",
            });
        }
    }
}

fn validate_checkpoint_state(
    errors: &mut Vec<MicroTaskLoopControlValidationError>,
    checkpoint: &MicroTaskLoopCheckpointV1,
    verifier_outcomes: &HashMap<&str, &MicroTaskVerifierOutcomeV1>,
) {
    if requires_verifier_outcome(checkpoint.loop_state) {
        match checkpoint.latest_verifier_outcome_id.as_deref() {
            Some(outcome_id) if verifier_outcomes.contains_key(outcome_id) => {}
            Some(_) => errors.push(MicroTaskLoopControlValidationError {
                field: "latest_verifier_outcome_id",
                message: "checkpoint references an unknown verifier outcome",
            }),
            None => errors.push(MicroTaskLoopControlValidationError {
                field: "latest_verifier_outcome_id",
                message: "checkpoint state requires a verifier outcome",
            }),
        }
    }

    if matches!(
        checkpoint.loop_state,
        MicroTaskLoopState::Escalated | MicroTaskLoopState::DeadLettered
    ) && checkpoint
        .escalation_target
        .as_deref()
        .is_none_or(str::is_empty)
    {
        errors.push(MicroTaskLoopControlValidationError {
            field: "escalation_target",
            message: "escalated and dead-lettered loops require an escalation target",
        });
    }

    if checkpoint.loop_state == MicroTaskLoopState::Completed
        && checkpoint
            .completion_report_id
            .as_deref()
            .is_none_or(str::is_empty)
    {
        errors.push(MicroTaskLoopControlValidationError {
            field: "completion_report_id",
            message: "completed loops require a completion report id",
        });
    }

    if checkpoint.dead_letter_posture != MicroTaskLoopDeadLetterPosture::None
        && !matches!(
            checkpoint.loop_state,
            MicroTaskLoopState::Escalated | MicroTaskLoopState::DeadLettered
        )
    {
        errors.push(MicroTaskLoopControlValidationError {
            field: "dead_letter_posture",
            message: "dead-letter posture must align with escalated or dead-lettered loop state",
        });
    }
}

fn requires_verifier_outcome(loop_state: MicroTaskLoopState) -> bool {
    matches!(
        loop_state,
        MicroTaskLoopState::VerificationNeeded
            | MicroTaskLoopState::Retrying
            | MicroTaskLoopState::Escalated
            | MicroTaskLoopState::Completed
            | MicroTaskLoopState::DeadLettered
    )
}

fn verifier_outcome_by_id(
    outcomes: &[MicroTaskVerifierOutcomeV1],
) -> HashMap<&str, &MicroTaskVerifierOutcomeV1> {
    outcomes
        .iter()
        .map(|outcome| (outcome.outcome_id.as_str(), outcome))
        .collect()
}

fn require_non_empty(
    errors: &mut Vec<MicroTaskLoopControlValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(MicroTaskLoopControlValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<MicroTaskLoopControlValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(MicroTaskLoopControlValidationError {
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

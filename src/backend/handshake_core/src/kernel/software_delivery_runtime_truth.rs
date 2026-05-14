use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::action_envelope::AuthorityEffect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeTruthAuthoritySourceKind {
    ProductRuntimeRecord,
    WorkflowStateRecord,
    TaskBoardStateRecord,
    RoleMailboxStableRecord,
    GateStateRecord,
    EventLedgerProjection,
    GovernedActionReceipt,
    PacketProse,
    MailboxChronology,
    MarkdownFreshness,
}

impl RuntimeTruthAuthoritySourceKind {
    pub fn is_authoritative(self) -> bool {
        matches!(
            self,
            RuntimeTruthAuthoritySourceKind::ProductRuntimeRecord
                | RuntimeTruthAuthoritySourceKind::WorkflowStateRecord
                | RuntimeTruthAuthoritySourceKind::TaskBoardStateRecord
                | RuntimeTruthAuthoritySourceKind::RoleMailboxStableRecord
                | RuntimeTruthAuthoritySourceKind::GateStateRecord
                | RuntimeTruthAuthoritySourceKind::EventLedgerProjection
                | RuntimeTruthAuthoritySourceKind::GovernedActionReceipt
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoftwareDeliveryPhase {
    Startup,
    Planning,
    Implementation,
    Validation,
    Closeout,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoftwareDeliveryStatus {
    Ready,
    InProgress,
    Blocked,
    Submitted,
    Validated,
    Failed,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftwareDeliveryRuntimeJoinsV1 {
    pub work_packet_id: String,
    pub task_board_item_id: String,
    pub role_mailbox_thread_id: String,
    pub workflow_state_id: String,
    pub gate_state_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftwareDeliveryRuntimeTruthRecordV1 {
    pub schema_id: String,
    pub record_id: String,
    pub wp_id: String,
    pub mt_id: Option<String>,
    pub worktree_id: String,
    pub branch: String,
    pub phase: SoftwareDeliveryPhase,
    pub status: SoftwareDeliveryStatus,
    pub next_actor: String,
    pub waiting_on: Option<String>,
    pub record_seq: u64,
    pub source_kind: RuntimeTruthAuthoritySourceKind,
    pub joins: SoftwareDeliveryRuntimeJoinsV1,
    pub governed_action_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareDeliveryGovernedActionV1 {
    pub action_id: String,
    pub title: String,
    pub input_schema_id: String,
    pub result_schema_id: String,
    pub authority_effect: AuthorityEffect,
    pub allowed_source_kinds: Vec<RuntimeTruthAuthoritySourceKind>,
    pub output_record_schema_id: String,
    pub validation_hooks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareDeliveryRuntimePostureProjectionV1 {
    pub schema_id: String,
    pub wp_id: String,
    pub current_record: SoftwareDeliveryRuntimeTruthRecordV1,
    pub governed_actions: Vec<SoftwareDeliveryGovernedActionV1>,
    pub source_lineage_refs: Vec<String>,
    pub ignored_non_authority_source_kinds: Vec<RuntimeTruthAuthoritySourceKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareDeliveryRuntimeTruthError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn kernel002_software_delivery_governed_actions() -> Vec<SoftwareDeliveryGovernedActionV1> {
    vec![
        software_delivery_action(
            "kernel.software_delivery_runtime_truth.record",
            "Record Software Delivery Runtime Truth",
            "hsk.kernel.software_delivery_runtime_truth_record_input@1",
            "hsk.kernel.software_delivery_runtime_truth_record_result@1",
            AuthorityEffect::EventLedgerAuthorityWrite,
            &[
                RuntimeTruthAuthoritySourceKind::ProductRuntimeRecord,
                RuntimeTruthAuthoritySourceKind::WorkflowStateRecord,
                RuntimeTruthAuthoritySourceKind::TaskBoardStateRecord,
                RuntimeTruthAuthoritySourceKind::RoleMailboxStableRecord,
                RuntimeTruthAuthoritySourceKind::GateStateRecord,
                RuntimeTruthAuthoritySourceKind::GovernedActionReceipt,
            ],
            &["stable_id_join", "runtime_truth_source_kind"],
        ),
        software_delivery_action(
            "kernel.software_delivery_runtime_truth.transition",
            "Transition Software Delivery Runtime Truth",
            "hsk.kernel.software_delivery_runtime_truth_transition_input@1",
            "hsk.kernel.software_delivery_runtime_truth_transition_result@1",
            AuthorityEffect::EventLedgerAuthorityWrite,
            &[
                RuntimeTruthAuthoritySourceKind::ProductRuntimeRecord,
                RuntimeTruthAuthoritySourceKind::EventLedgerProjection,
                RuntimeTruthAuthoritySourceKind::GovernedActionReceipt,
            ],
            &[
                "stable_id_join",
                "runtime_truth_source_kind",
                "transition_authority",
            ],
        ),
        software_delivery_action(
            "kernel.software_delivery_runtime_truth.project",
            "Project Software Delivery Runtime Truth",
            "hsk.kernel.software_delivery_runtime_posture_query@1",
            "hsk.kernel.software_delivery_runtime_posture_projection@1",
            AuthorityEffect::ProjectionOnly,
            &[
                RuntimeTruthAuthoritySourceKind::ProductRuntimeRecord,
                RuntimeTruthAuthoritySourceKind::EventLedgerProjection,
                RuntimeTruthAuthoritySourceKind::GovernedActionReceipt,
            ],
            &[
                "stable_id_join",
                "runtime_truth_source_kind",
                "latest_record_seq",
            ],
        ),
    ]
}

pub fn validate_software_delivery_runtime_truth_record(
    record: &SoftwareDeliveryRuntimeTruthRecordV1,
) -> Result<(), Vec<SoftwareDeliveryRuntimeTruthError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &record.schema_id);
    require_non_empty(&mut errors, "record_id", &record.record_id);
    require_non_empty(&mut errors, "wp_id", &record.wp_id);
    require_non_empty(&mut errors, "worktree_id", &record.worktree_id);
    require_non_empty(&mut errors, "branch", &record.branch);
    require_non_empty(&mut errors, "next_actor", &record.next_actor);
    require_non_empty(
        &mut errors,
        "joins.work_packet_id",
        &record.joins.work_packet_id,
    );
    require_non_empty(
        &mut errors,
        "joins.task_board_item_id",
        &record.joins.task_board_item_id,
    );
    require_non_empty(
        &mut errors,
        "joins.role_mailbox_thread_id",
        &record.joins.role_mailbox_thread_id,
    );
    require_non_empty(
        &mut errors,
        "joins.workflow_state_id",
        &record.joins.workflow_state_id,
    );
    require_non_empty(
        &mut errors,
        "joins.gate_state_id",
        &record.joins.gate_state_id,
    );

    if record.record_seq == 0 {
        errors.push(SoftwareDeliveryRuntimeTruthError {
            field: "record_seq",
            message: "record sequence must be greater than zero",
        });
    }

    if !record.source_kind.is_authoritative() {
        errors.push(SoftwareDeliveryRuntimeTruthError {
            field: "source_kind",
            message: "source is not an authoritative runtime truth source",
        });
    }

    if record.joins.work_packet_id != record.wp_id {
        errors.push(SoftwareDeliveryRuntimeTruthError {
            field: "joins.work_packet_id",
            message: "work packet join must match runtime truth wp_id",
        });
    }

    if record.governed_action_ids.is_empty() {
        errors.push(SoftwareDeliveryRuntimeTruthError {
            field: "governed_action_ids",
            message: "at least one governed action id is required",
        });
    }

    if record.evidence_refs.is_empty() {
        errors.push(SoftwareDeliveryRuntimeTruthError {
            field: "evidence_refs",
            message: "at least one proof or receipt evidence reference is required",
        });
    }

    if !record
        .folded_source_refs
        .iter()
        .any(|source| source.contains("WP-1-Software-Delivery-Runtime-Truth-v1"))
    {
        errors.push(SoftwareDeliveryRuntimeTruthError {
            field: "folded_source_refs",
            message: "folded legacy runtime truth source must be preserved",
        });
    }

    if has_duplicates(&record.governed_action_ids) {
        errors.push(SoftwareDeliveryRuntimeTruthError {
            field: "governed_action_ids",
            message: "governed action ids must be unique",
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn query_software_delivery_runtime_posture(
    records: &[SoftwareDeliveryRuntimeTruthRecordV1],
    actions: &[SoftwareDeliveryGovernedActionV1],
    wp_id: &str,
) -> Result<SoftwareDeliveryRuntimePostureProjectionV1, Vec<SoftwareDeliveryRuntimeTruthError>> {
    let mut valid_records = Vec::new();
    let mut ignored_non_authority_source_kinds = Vec::new();
    let mut errors = Vec::new();

    for record in records.iter().filter(|record| record.wp_id == wp_id) {
        match validate_software_delivery_runtime_truth_record(record) {
            Ok(()) => valid_records.push(record.clone()),
            Err(record_errors) if is_only_non_authority_source_error(&record_errors) => {
                push_unique_source_kind(
                    &mut ignored_non_authority_source_kinds,
                    record.source_kind,
                );
            }
            Err(record_errors) => errors.extend(record_errors),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let Some(current_record) = valid_records
        .into_iter()
        .max_by_key(|record| record.record_seq)
    else {
        return Err(vec![SoftwareDeliveryRuntimeTruthError {
            field: "records",
            message: "no authoritative runtime truth record exists for requested work packet",
        }]);
    };

    let governed_actions = select_record_actions(actions, &current_record)?;
    let source_lineage_refs = current_record.folded_source_refs.clone();

    Ok(SoftwareDeliveryRuntimePostureProjectionV1 {
        schema_id: "hsk.kernel.software_delivery_runtime_posture_projection@1".to_string(),
        wp_id: wp_id.to_string(),
        current_record,
        governed_actions,
        source_lineage_refs,
        ignored_non_authority_source_kinds,
    })
}

fn select_record_actions(
    actions: &[SoftwareDeliveryGovernedActionV1],
    record: &SoftwareDeliveryRuntimeTruthRecordV1,
) -> Result<Vec<SoftwareDeliveryGovernedActionV1>, Vec<SoftwareDeliveryRuntimeTruthError>> {
    let mut selected = Vec::new();
    let mut missing_action_ids = Vec::new();

    for action_id in &record.governed_action_ids {
        match actions.iter().find(|action| &action.action_id == action_id) {
            Some(action) => selected.push(action.clone()),
            None => missing_action_ids.push(action_id.clone()),
        }
    }

    if missing_action_ids.is_empty() {
        Ok(selected)
    } else {
        Err(vec![SoftwareDeliveryRuntimeTruthError {
            field: "governed_action_ids",
            message: "runtime truth record references an unknown governed action id",
        }])
    }
}

fn software_delivery_action(
    action_id: &str,
    title: &str,
    input_schema_id: &str,
    result_schema_id: &str,
    authority_effect: AuthorityEffect,
    allowed_source_kinds: &[RuntimeTruthAuthoritySourceKind],
    validation_hooks: &[&str],
) -> SoftwareDeliveryGovernedActionV1 {
    SoftwareDeliveryGovernedActionV1 {
        action_id: action_id.to_string(),
        title: title.to_string(),
        input_schema_id: input_schema_id.to_string(),
        result_schema_id: result_schema_id.to_string(),
        authority_effect,
        allowed_source_kinds: allowed_source_kinds.to_vec(),
        output_record_schema_id: "hsk.kernel.software_delivery_runtime_truth_record@1".to_string(),
        validation_hooks: validation_hooks
            .iter()
            .map(|hook| (*hook).to_string())
            .collect(),
    }
}

fn require_non_empty(
    errors: &mut Vec<SoftwareDeliveryRuntimeTruthError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(SoftwareDeliveryRuntimeTruthError {
            field,
            message: "value must not be empty",
        });
    }
}

fn is_only_non_authority_source_error(errors: &[SoftwareDeliveryRuntimeTruthError]) -> bool {
    errors.len() == 1 && errors[0].field == "source_kind"
}

fn push_unique_source_kind(
    source_kinds: &mut Vec<RuntimeTruthAuthoritySourceKind>,
    source_kind: RuntimeTruthAuthoritySourceKind,
) {
    if !source_kinds.contains(&source_kind) {
        source_kinds.push(source_kind);
    }
}

fn has_duplicates(values: &[String]) -> bool {
    let mut seen = HashSet::new();
    values.iter().any(|value| !seen.insert(value))
}

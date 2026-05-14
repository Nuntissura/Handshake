use serde::{Deserialize, Serialize};

const FOLDED_OVERLAY_LIFECYCLE_STUB: &str =
    "WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifecycleState {
    NotStarted,
    Running,
    Paused,
    Steering,
    Canceling,
    Canceled,
    Recovered,
    CloseoutReady,
    Closed,
    PartialFailure,
    Restarting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlActionKind {
    Start,
    Steer,
    Cancel,
    Close,
    Recover,
    CheckpointReplay,
    Restart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryPosture {
    NotNeeded,
    Restartable,
    ReplayReady,
    ReplayBlocked,
    Recovered,
    PartialFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CheckpointStatus {
    Available,
    ReplayValidated,
    ReplayFailed,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartialFailureKind {
    None,
    ActionTimeout,
    ActorCrash,
    ValidationFailed,
    CheckpointGap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifecycleRecoverySourceKind {
    ProductLifecycleRecord,
    WorkflowStateRecord,
    CheckpointLineageRecord,
    GovernedActionReceipt,
    TranscriptHistory,
    PacketEdit,
    UiLocalState,
}

impl LifecycleRecoverySourceKind {
    pub fn is_authoritative(self) -> bool {
        matches!(
            self,
            LifecycleRecoverySourceKind::ProductLifecycleRecord
                | LifecycleRecoverySourceKind::WorkflowStateRecord
                | LifecycleRecoverySourceKind::CheckpointLineageRecord
                | LifecycleRecoverySourceKind::GovernedActionReceipt
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointRefV1 {
    pub checkpoint_id: String,
    pub sequence: u64,
    pub state_hash: String,
    pub event_ledger_ref: String,
    pub status: CheckpointStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernedActionLineageV1 {
    pub action_id: String,
    pub trace_id: String,
    pub result_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayLifecycleRecoveryRecordV1 {
    pub schema_id: String,
    pub lifecycle_id: String,
    pub work_item_id: String,
    pub record_seq: u64,
    pub lifecycle_state: LifecycleState,
    pub recovery_posture: RecoveryPosture,
    pub available_control_actions: Vec<ControlActionKind>,
    pub checkpoints: Vec<CheckpointRefV1>,
    pub partial_failure: PartialFailureKind,
    pub restart_safe: bool,
    pub projection_safe: bool,
    pub governed_action_lineage: Vec<GovernedActionLineageV1>,
    pub source_kind: LifecycleRecoverySourceKind,
    pub evidence_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayLifecycleRecoveryPostureProjectionV1 {
    pub schema_id: String,
    pub work_item_id: String,
    pub current_record: OverlayLifecycleRecoveryRecordV1,
    pub lifecycle_state: LifecycleState,
    pub recovery_posture: RecoveryPosture,
    pub partial_failure: PartialFailureKind,
    pub replay_checkpoint_ids: Vec<String>,
    pub ignored_non_authority_source_kinds: Vec<LifecycleRecoverySourceKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayLifecycleRecoveryValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_overlay_lifecycle_recovery_record(
    record: &OverlayLifecycleRecoveryRecordV1,
) -> Result<(), Vec<OverlayLifecycleRecoveryValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &record.schema_id);
    require_non_empty(&mut errors, "lifecycle_id", &record.lifecycle_id);
    require_non_empty(&mut errors, "work_item_id", &record.work_item_id);
    require_vec(
        &mut errors,
        "available_control_actions",
        &record.available_control_actions,
    );
    require_vec(&mut errors, "checkpoints", &record.checkpoints);
    require_vec(
        &mut errors,
        "governed_action_lineage",
        &record.governed_action_lineage,
    );
    require_vec(&mut errors, "evidence_refs", &record.evidence_refs);

    if record.record_seq == 0 {
        errors.push(OverlayLifecycleRecoveryValidationError {
            field: "record_seq",
            message: "record sequence must be greater than zero",
        });
    }

    if !record.source_kind.is_authoritative() {
        errors.push(OverlayLifecycleRecoveryValidationError {
            field: "source_kind",
            message: "lifecycle recovery source is not authoritative",
        });
    }

    if !record.restart_safe {
        errors.push(OverlayLifecycleRecoveryValidationError {
            field: "restart_safe",
            message: "lifecycle recovery posture must be restart safe",
        });
    }

    if !record.projection_safe {
        errors.push(OverlayLifecycleRecoveryValidationError {
            field: "projection_safe",
            message: "lifecycle recovery posture must be projection safe",
        });
    }

    if matches!(
        record.recovery_posture,
        RecoveryPosture::Restartable | RecoveryPosture::ReplayReady
    ) && !record
        .checkpoints
        .iter()
        .any(|checkpoint| checkpoint.status == CheckpointStatus::ReplayValidated)
    {
        errors.push(OverlayLifecycleRecoveryValidationError {
            field: "checkpoints",
            message: "restartable or replay-ready posture requires a replay-validated checkpoint",
        });
    }

    for checkpoint in &record.checkpoints {
        require_non_empty(
            &mut errors,
            "checkpoints.checkpoint_id",
            &checkpoint.checkpoint_id,
        );
        require_non_empty(
            &mut errors,
            "checkpoints.state_hash",
            &checkpoint.state_hash,
        );
        require_non_empty(
            &mut errors,
            "checkpoints.event_ledger_ref",
            &checkpoint.event_ledger_ref,
        );
        if checkpoint.sequence == 0 {
            errors.push(OverlayLifecycleRecoveryValidationError {
                field: "checkpoints.sequence",
                message: "checkpoint sequence must be greater than zero",
            });
        }
    }

    for action in &record.governed_action_lineage {
        require_non_empty(
            &mut errors,
            "governed_action_lineage.action_id",
            &action.action_id,
        );
        require_non_empty(
            &mut errors,
            "governed_action_lineage.trace_id",
            &action.trace_id,
        );
        require_non_empty(
            &mut errors,
            "governed_action_lineage.result_id",
            &action.result_id,
        );
    }

    if !record
        .folded_source_refs
        .iter()
        .any(|source| source.contains(FOLDED_OVERLAY_LIFECYCLE_STUB))
    {
        errors.push(OverlayLifecycleRecoveryValidationError {
            field: "folded_source_refs",
            message: "folded lifecycle recovery source must be preserved",
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn query_overlay_lifecycle_recovery_posture(
    records: &[OverlayLifecycleRecoveryRecordV1],
    work_item_id: &str,
) -> Result<OverlayLifecycleRecoveryPostureProjectionV1, Vec<OverlayLifecycleRecoveryValidationError>>
{
    let mut valid_records = Vec::new();
    let mut ignored_non_authority_source_kinds = Vec::new();
    let mut errors = Vec::new();

    for record in records
        .iter()
        .filter(|record| record.work_item_id == work_item_id)
    {
        match validate_overlay_lifecycle_recovery_record(record) {
            Ok(()) => valid_records.push(record.clone()),
            Err(record_errors) if is_only_non_authority_source_error(&record_errors) => {
                if !ignored_non_authority_source_kinds.contains(&record.source_kind) {
                    ignored_non_authority_source_kinds.push(record.source_kind);
                }
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
        return Err(vec![OverlayLifecycleRecoveryValidationError {
            field: "records",
            message: "no authoritative lifecycle recovery record exists for work item",
        }]);
    };

    let replay_checkpoint_ids = current_record
        .checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.status == CheckpointStatus::ReplayValidated)
        .map(|checkpoint| checkpoint.checkpoint_id.clone())
        .collect();

    Ok(OverlayLifecycleRecoveryPostureProjectionV1 {
        schema_id: "hsk.kernel.overlay_lifecycle_recovery_posture_projection@1".to_string(),
        work_item_id: work_item_id.to_string(),
        lifecycle_state: current_record.lifecycle_state,
        recovery_posture: current_record.recovery_posture,
        partial_failure: current_record.partial_failure,
        current_record,
        replay_checkpoint_ids,
        ignored_non_authority_source_kinds,
    })
}

fn require_non_empty(
    errors: &mut Vec<OverlayLifecycleRecoveryValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(OverlayLifecycleRecoveryValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<OverlayLifecycleRecoveryValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(OverlayLifecycleRecoveryValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn is_only_non_authority_source_error(errors: &[OverlayLifecycleRecoveryValidationError]) -> bool {
    errors.len() == 1 && errors[0].field == "source_kind"
}

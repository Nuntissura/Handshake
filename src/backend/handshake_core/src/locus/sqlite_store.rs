use crate::storage::{StorageError, StorageResult};
use serde_json::Value;

use super::types::LocusOperation;

pub fn parse_locus_operation(
    protocol_id: &str,
    raw_inputs: &Value,
) -> StorageResult<LocusOperation> {
    match protocol_id {
        "locus_create_wp_v1" => Ok(LocusOperation::CreateWp(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_update_wp_v1" => Ok(LocusOperation::UpdateWp(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_gate_wp_v1" => Ok(LocusOperation::GateWp(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_close_wp_v1" => Ok(LocusOperation::CloseWp(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_delete_wp_v1" => Ok(LocusOperation::DeleteWp(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_register_mts_v1" => Ok(LocusOperation::RegisterMts(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_start_mt_v1" => Ok(LocusOperation::StartMt(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_record_iteration_v1" => Ok(LocusOperation::RecordIteration(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_complete_mt_v1" => Ok(LocusOperation::CompleteMt(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_add_dependency_v1" => Ok(LocusOperation::AddDependency(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_remove_dependency_v1" => Ok(LocusOperation::RemoveDependency(
            serde_json::from_value(raw_inputs.clone())?,
        )),
        "locus_query_ready_v1" => Ok(LocusOperation::QueryReady(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_get_wp_status_v1" => Ok(LocusOperation::GetWpStatus(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_get_mt_progress_v1" => Ok(LocusOperation::GetMtProgress(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        "locus_sync_task_board_v1" => Ok(LocusOperation::SyncTaskBoard(serde_json::from_value(
            raw_inputs.clone(),
        )?)),
        other => Err(StorageError::Validation(match other {
            "" => "missing locus protocol_id",
            _ => "unknown locus protocol_id",
        })),
    }
}

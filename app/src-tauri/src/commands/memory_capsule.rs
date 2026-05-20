use std::{collections::BTreeMap, sync::Mutex};

use handshake_core::memory::{
    CapsuleFlightRecorderEvent, CapsuleRecord, CapsuleSummary, FemsFlightRecorder,
    FemsFlightRecorderError, KernelActionRejection, KernelActionSubmission, KernelActionSubmitter,
    ListRecentCapsulesRequest, MemoryCapsuleIpcStore, MemoryIpcError, MemoryIpcService,
    SuppressCapsuleRequest, SuppressItemRequest, SuppressionReceipt, MEMORY_CAPSULE_GET_COMMAND,
    MEMORY_CAPSULE_LIST_RECENT_COMMAND, MEMORY_CAPSULE_SUPPRESS_CAPSULE_COMMAND,
    MEMORY_CAPSULE_SUPPRESS_ITEM_COMMAND,
};
use tauri::State;
use uuid::Uuid;

#[derive(Default)]
pub struct MemoryCapsuleIpcState {
    records: Mutex<BTreeMap<Uuid, CapsuleRecord>>,
    submissions: Mutex<Vec<KernelActionSubmission>>,
    flight_recorder_events: Mutex<Vec<CapsuleFlightRecorderEvent>>,
}

impl MemoryCapsuleIpcState {
    fn service(&self) -> MemoryIpcService<'_> {
        MemoryIpcService::new(self, self, self)
    }
}

impl MemoryCapsuleIpcStore for MemoryCapsuleIpcState {
    fn all_capsule_records(&self) -> Result<Vec<CapsuleRecord>, MemoryIpcError> {
        let records = self.records.lock().map_err(|_| MemoryIpcError::Store {
            message: "memory capsule record store mutex poisoned".to_string(),
        })?;
        Ok(records.values().cloned().collect())
    }

    fn get_capsule_record(
        &self,
        capsule_id: Uuid,
    ) -> Result<Option<CapsuleRecord>, MemoryIpcError> {
        let records = self.records.lock().map_err(|_| MemoryIpcError::Store {
            message: "memory capsule record store mutex poisoned".to_string(),
        })?;
        Ok(records.get(&capsule_id).cloned())
    }

    fn save_capsule_record(&self, record: CapsuleRecord) -> Result<(), MemoryIpcError> {
        let mut records = self.records.lock().map_err(|_| MemoryIpcError::Store {
            message: "memory capsule record store mutex poisoned".to_string(),
        })?;
        records.insert(record.capsule_id, record);
        Ok(())
    }
}

impl KernelActionSubmitter for MemoryCapsuleIpcState {
    fn submit(&self, submission: KernelActionSubmission) -> Result<(), KernelActionRejection> {
        let Ok(mut submissions) = self.submissions.lock() else {
            return Err(KernelActionRejection {
                code: "memory_capsule_ipc_state_poisoned".to_string(),
                reason: "memory capsule IPC submission queue mutex poisoned".to_string(),
            });
        };
        submissions.push(submission);
        Ok(())
    }
}

impl FemsFlightRecorder for MemoryCapsuleIpcState {
    fn record_event(
        &self,
        event: CapsuleFlightRecorderEvent,
    ) -> Result<(), FemsFlightRecorderError> {
        let Ok(mut events) = self.flight_recorder_events.lock() else {
            return Err(FemsFlightRecorderError::new(
                "memory capsule IPC flight recorder mutex poisoned",
            ));
        };
        events.push(event);
        Ok(())
    }
}

#[tauri::command]
pub async fn kernel_memory_capsule_list_recent(
    limit: u32,
    state: State<'_, MemoryCapsuleIpcState>,
) -> Result<Vec<CapsuleSummary>, String> {
    let _ = MEMORY_CAPSULE_LIST_RECENT_COMMAND;
    let response = state
        .service()
        .list_recent(ListRecentCapsulesRequest { limit })
        .map_err(|error| error.to_string())?;
    Ok(response.capsules)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_memory_capsule_get(
    capsule_id: Uuid,
    state: State<'_, MemoryCapsuleIpcState>,
) -> Result<CapsuleRecord, String> {
    let _ = MEMORY_CAPSULE_GET_COMMAND;
    let response = state
        .service()
        .get(handshake_core::memory::GetCapsuleRequest { capsule_id })
        .map_err(|error| error.to_string())?;
    Ok(response.record)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_memory_capsule_suppress_item(
    capsule_id: Uuid,
    item_id: String,
    reason: String,
    actor_id: String,
    session_id: String,
    state: State<'_, MemoryCapsuleIpcState>,
) -> Result<SuppressionReceipt, String> {
    let _ = MEMORY_CAPSULE_SUPPRESS_ITEM_COMMAND;
    state
        .service()
        .suppress_item(SuppressItemRequest {
            capsule_id,
            item_id,
            reason,
            actor_id,
            session_id,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_memory_capsule_suppress_capsule(
    capsule_id: Uuid,
    reason: String,
    actor_id: String,
    session_id: String,
    state: State<'_, MemoryCapsuleIpcState>,
) -> Result<SuppressionReceipt, String> {
    let _ = MEMORY_CAPSULE_SUPPRESS_CAPSULE_COMMAND;
    state
        .service()
        .suppress_capsule(SuppressCapsuleRequest {
            capsule_id,
            reason,
            actor_id,
            session_id,
        })
        .map_err(|error| error.to_string())
}

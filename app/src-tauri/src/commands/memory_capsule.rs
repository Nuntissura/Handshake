//! MT-146 Memory capsule inspection & suppression IPC.
//!
//! Production wiring is exclusively backed by the shared embedded
//! SurrealDB/EventLedger authority. The default state is unavailable and every
//! operation fails closed; there is no process-local durable-success fallback.

use std::sync::{Arc, Mutex};

use handshake_core::{
    memory::{
        CapsuleFlightRecorderEvent, CapsuleRecord, CapsuleSummary, FemsFlightRecorder,
        FemsFlightRecorderError, KernelActionRejection, KernelActionSubmission,
        KernelActionSubmitter, ListRecentCapsulesRequest, MemoryCapsuleIpcStore, MemoryIpcError,
        MemoryIpcService, SuppressCapsuleRequest, SuppressItemRequest, SuppressionReceipt,
        SurrealKernelActionSubmitter, SurrealMemoryCapsuleStore, MEMORY_CAPSULE_GET_COMMAND,
        MEMORY_CAPSULE_LIST_RECENT_COMMAND, MEMORY_CAPSULE_SUPPRESS_CAPSULE_COMMAND,
        MEMORY_CAPSULE_SUPPRESS_ITEM_COMMAND,
    },
    storage::Database,
};
use tauri::State;
use uuid::Uuid;

enum MemoryCapsuleStoreBackend {
    Unavailable,
    Surreal {
        store: Arc<SurrealMemoryCapsuleStore>,
        submitter: Arc<SurrealKernelActionSubmitter>,
        flight_recorder_events: Mutex<Vec<CapsuleFlightRecorderEvent>>,
    },
}

impl Default for MemoryCapsuleStoreBackend {
    fn default() -> Self {
        Self::Unavailable
    }
}

#[derive(Default)]
pub struct MemoryCapsuleIpcState {
    backend: MemoryCapsuleStoreBackend,
}

impl MemoryCapsuleIpcState {
    pub fn with_surreal(db: Arc<dyn Database>) -> Self {
        Self {
            backend: MemoryCapsuleStoreBackend::Surreal {
                store: Arc::new(SurrealMemoryCapsuleStore::with_db(Arc::clone(&db))),
                submitter: Arc::new(SurrealKernelActionSubmitter::with_db(db)),
                flight_recorder_events: Mutex::new(Vec::new()),
            },
        }
    }

    fn service(&self) -> MemoryIpcService<'_> {
        MemoryIpcService::new(self, self, self)
    }

    fn unavailable_store_error() -> MemoryIpcError {
        MemoryIpcError::Store {
            message: "embedded SurrealDB authority is unavailable".to_owned(),
        }
    }
}

impl MemoryCapsuleIpcStore for MemoryCapsuleIpcState {
    fn all_capsule_records(&self) -> Result<Vec<CapsuleRecord>, MemoryIpcError> {
        match &self.backend {
            MemoryCapsuleStoreBackend::Unavailable => Err(Self::unavailable_store_error()),
            MemoryCapsuleStoreBackend::Surreal { store, .. } => store.all_capsule_records(),
        }
    }

    fn get_capsule_record(
        &self,
        capsule_id: Uuid,
    ) -> Result<Option<CapsuleRecord>, MemoryIpcError> {
        match &self.backend {
            MemoryCapsuleStoreBackend::Unavailable => Err(Self::unavailable_store_error()),
            MemoryCapsuleStoreBackend::Surreal { store, .. } => {
                store.get_capsule_record(capsule_id)
            }
        }
    }

    fn save_capsule_record(&self, record: CapsuleRecord) -> Result<(), MemoryIpcError> {
        match &self.backend {
            MemoryCapsuleStoreBackend::Unavailable => Err(Self::unavailable_store_error()),
            MemoryCapsuleStoreBackend::Surreal { store, .. } => store.save_capsule_record(record),
        }
    }
}

impl KernelActionSubmitter for MemoryCapsuleIpcState {
    fn submit(&self, submission: KernelActionSubmission) -> Result<(), KernelActionRejection> {
        match &self.backend {
            MemoryCapsuleStoreBackend::Unavailable => Err(KernelActionRejection {
                code: "memory_capsule_surreal_unavailable".to_owned(),
                reason: "embedded SurrealDB authority is unavailable".to_owned(),
            }),
            MemoryCapsuleStoreBackend::Surreal { submitter, .. } => submitter.submit(submission),
        }
    }
}

impl FemsFlightRecorder for MemoryCapsuleIpcState {
    fn record_event(
        &self,
        event: CapsuleFlightRecorderEvent,
    ) -> Result<(), FemsFlightRecorderError> {
        let MemoryCapsuleStoreBackend::Surreal {
            flight_recorder_events,
            ..
        } = &self.backend
        else {
            return Err(FemsFlightRecorderError::new(
                "embedded SurrealDB authority is unavailable",
            ));
        };
        let Ok(mut events) = flight_recorder_events.lock() else {
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

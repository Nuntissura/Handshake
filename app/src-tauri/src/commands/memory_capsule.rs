//! MT-146 Memory capsule inspection & suppression IPC.
//!
//! The IPC state holds two seams: a durable [`MemoryCapsuleIpcStore`] and a
//! durable [`KernelActionSubmitter`]. When the orchestrator wires a real
//! Postgres `Database` through [`MemoryCapsuleIpcState::with_postgres`], the
//! state binds to [`PostgresMemoryCapsuleStore`] + [`PostgresKernelActionSubmitter`]
//! so list/get/suppress IPC operations are durable across process restarts.
//!
//! When the Postgres binding has not been wired (e.g. early-boot or test
//! runs without `DATABASE_URL`), the state falls back to an in-memory store.
//! That fallback is explicitly *not* the validator's required surface; the
//! production wiring lives behind `with_postgres`.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use handshake_core::{
    memory::{
        CapsuleFlightRecorderEvent, CapsuleRecord, CapsuleSummary, FemsFlightRecorder,
        FemsFlightRecorderError, KernelActionRejection, KernelActionSubmission,
        KernelActionSubmitter, ListRecentCapsulesRequest, MemoryCapsuleIpcStore, MemoryIpcError,
        MemoryIpcService, PostgresKernelActionSubmitter, PostgresMemoryCapsuleStore,
        SuppressCapsuleRequest, SuppressItemRequest, SuppressionReceipt, MEMORY_CAPSULE_GET_COMMAND,
        MEMORY_CAPSULE_LIST_RECENT_COMMAND, MEMORY_CAPSULE_SUPPRESS_CAPSULE_COMMAND,
        MEMORY_CAPSULE_SUPPRESS_ITEM_COMMAND,
    },
    storage::Database,
};
use tauri::State;
use uuid::Uuid;

/// MT-146: routing target for capsule storage + kernel action persistence.
///
/// Variants intentionally hold trait objects so the state can be swapped at
/// boot time without changing IPC command signatures.
enum MemoryCapsuleStoreBackend {
    InMemory {
        records: Mutex<BTreeMap<Uuid, CapsuleRecord>>,
        submissions: Mutex<Vec<KernelActionSubmission>>,
        flight_recorder_events: Mutex<Vec<CapsuleFlightRecorderEvent>>,
    },
    Postgres {
        store: Arc<PostgresMemoryCapsuleStore>,
        submitter: Arc<PostgresKernelActionSubmitter>,
        flight_recorder_events: Mutex<Vec<CapsuleFlightRecorderEvent>>,
    },
}

impl Default for MemoryCapsuleStoreBackend {
    fn default() -> Self {
        Self::InMemory {
            records: Mutex::new(BTreeMap::new()),
            submissions: Mutex::new(Vec::new()),
            flight_recorder_events: Mutex::new(Vec::new()),
        }
    }
}

#[derive(Default)]
pub struct MemoryCapsuleIpcState {
    backend: MemoryCapsuleStoreBackend,
}

impl MemoryCapsuleIpcState {
    /// MT-146 production wiring: bind the IPC state to a real Postgres database
    /// so list/get/suppress operations are durable across process restarts.
    pub fn with_postgres(db: Arc<dyn Database>) -> Self {
        Self {
            backend: MemoryCapsuleStoreBackend::Postgres {
                store: Arc::new(PostgresMemoryCapsuleStore::with_db(Arc::clone(&db))),
                submitter: Arc::new(PostgresKernelActionSubmitter::with_db(db)),
                flight_recorder_events: Mutex::new(Vec::new()),
            },
        }
    }

    fn service(&self) -> MemoryIpcService<'_> {
        MemoryIpcService::new(self, self, self)
    }
}

impl MemoryCapsuleIpcStore for MemoryCapsuleIpcState {
    fn all_capsule_records(&self) -> Result<Vec<CapsuleRecord>, MemoryIpcError> {
        match &self.backend {
            MemoryCapsuleStoreBackend::InMemory { records, .. } => {
                let records = records.lock().map_err(|_| MemoryIpcError::Store {
                    message: "memory capsule record store mutex poisoned".to_string(),
                })?;
                Ok(records.values().cloned().collect())
            }
            MemoryCapsuleStoreBackend::Postgres { store, .. } => store.all_capsule_records(),
        }
    }

    fn get_capsule_record(
        &self,
        capsule_id: Uuid,
    ) -> Result<Option<CapsuleRecord>, MemoryIpcError> {
        match &self.backend {
            MemoryCapsuleStoreBackend::InMemory { records, .. } => {
                let records = records.lock().map_err(|_| MemoryIpcError::Store {
                    message: "memory capsule record store mutex poisoned".to_string(),
                })?;
                Ok(records.get(&capsule_id).cloned())
            }
            MemoryCapsuleStoreBackend::Postgres { store, .. } => {
                store.get_capsule_record(capsule_id)
            }
        }
    }

    fn save_capsule_record(&self, record: CapsuleRecord) -> Result<(), MemoryIpcError> {
        match &self.backend {
            MemoryCapsuleStoreBackend::InMemory { records, .. } => {
                let mut records = records.lock().map_err(|_| MemoryIpcError::Store {
                    message: "memory capsule record store mutex poisoned".to_string(),
                })?;
                records.insert(record.capsule_id, record);
                Ok(())
            }
            MemoryCapsuleStoreBackend::Postgres { store, .. } => store.save_capsule_record(record),
        }
    }
}

impl KernelActionSubmitter for MemoryCapsuleIpcState {
    fn submit(&self, submission: KernelActionSubmission) -> Result<(), KernelActionRejection> {
        match &self.backend {
            MemoryCapsuleStoreBackend::InMemory { submissions, .. } => {
                let Ok(mut submissions) = submissions.lock() else {
                    return Err(KernelActionRejection {
                        code: "memory_capsule_ipc_state_poisoned".to_string(),
                        reason: "memory capsule IPC submission queue mutex poisoned".to_string(),
                    });
                };
                submissions.push(submission);
                Ok(())
            }
            MemoryCapsuleStoreBackend::Postgres { submitter, .. } => submitter.submit(submission),
        }
    }
}

impl FemsFlightRecorder for MemoryCapsuleIpcState {
    fn record_event(
        &self,
        event: CapsuleFlightRecorderEvent,
    ) -> Result<(), FemsFlightRecorderError> {
        let events_mutex = match &self.backend {
            MemoryCapsuleStoreBackend::InMemory {
                flight_recorder_events,
                ..
            } => flight_recorder_events,
            MemoryCapsuleStoreBackend::Postgres {
                flight_recorder_events,
                ..
            } => flight_recorder_events,
        };
        let Ok(mut events) = events_mutex.lock() else {
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

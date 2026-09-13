//! MT-159 pinned core memory IPC.
//!
//! Pin/unpin/list commands are backed by the shared embedded SurrealDB kernel
//! EventLedger. There is no in-memory success path: if the authority has not
//! initialized, commands return a typed error instead of
//! pretending that a pin was durable.

use std::sync::Arc;

use handshake_core::{
    memory::{
        PinError, PinIpcService, PinReceipt, PinSubmitter, PinnedItem, SetPinRequest,
        SurrealKernelActionSubmitter, PIN_MEMORY_ACTION_ID, UNPIN_MEMORY_ACTION_ID,
    },
    storage::Database,
};
use tauri::State;
use uuid::Uuid;

enum MemoryPinBackend {
    Unavailable {
        reason: String,
    },
    Surreal {
        submitter: Arc<SurrealKernelActionSubmitter>,
    },
}

pub struct MemoryPinIpcState {
    backend: MemoryPinBackend,
}

impl Default for MemoryPinIpcState {
    fn default() -> Self {
        Self {
            backend: MemoryPinBackend::Unavailable {
                reason: "embedded SurrealDB memory pin authority has not initialized".to_string(),
            },
        }
    }
}

impl MemoryPinIpcState {
    pub fn with_surreal(db: Arc<dyn Database>) -> Self {
        Self {
            backend: MemoryPinBackend::Surreal {
                submitter: Arc::new(SurrealKernelActionSubmitter::with_db(db)),
            },
        }
    }

    fn service(&self) -> PinIpcService<'_> {
        PinIpcService::new(self)
    }
}

impl PinSubmitter for MemoryPinIpcState {
    fn set_pin(&self, item: PinnedItem) -> Result<PinReceipt, PinError> {
        match &self.backend {
            MemoryPinBackend::Unavailable { reason } => Err(PinError::Rejected {
                code: "memory_pin_surreal_unavailable".to_string(),
                reason: reason.clone(),
            }),
            MemoryPinBackend::Surreal { submitter } => submitter.set_pin(item),
        }
    }

    fn list_pinned(&self) -> Result<Vec<PinnedItem>, PinError> {
        match &self.backend {
            MemoryPinBackend::Unavailable { reason } => Err(PinError::Rejected {
                code: "memory_pin_surreal_unavailable".to_string(),
                reason: reason.clone(),
            }),
            MemoryPinBackend::Surreal { submitter } => submitter.list_pinned(),
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_memory_pin_set(
    item_id: Uuid,
    reason: String,
    actor_id: String,
    session_id: String,
    state: State<'_, MemoryPinIpcState>,
) -> Result<PinReceipt, String> {
    let _ = PIN_MEMORY_ACTION_ID;
    state
        .service()
        .set(SetPinRequest {
            item_id,
            pinned: true,
            reason,
            actor_id,
            session_id,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_memory_pin_unset(
    item_id: Uuid,
    reason: String,
    actor_id: String,
    session_id: String,
    state: State<'_, MemoryPinIpcState>,
) -> Result<PinReceipt, String> {
    let _ = UNPIN_MEMORY_ACTION_ID;
    state
        .service()
        .set(SetPinRequest {
            item_id,
            pinned: false,
            reason,
            actor_id,
            session_id,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn kernel_memory_pin_list(
    state: State<'_, MemoryPinIpcState>,
) -> Result<Vec<PinnedItem>, String> {
    state.service().list().map_err(|error| error.to_string())
}

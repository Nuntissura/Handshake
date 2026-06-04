//! MT-159 pinned core memory IPC.
//!
//! Pin/unpin/list commands are backed by the Postgres kernel event ledger via
//! `PostgresKernelActionSubmitter`. There is no in-memory success path: if the
//! app has not connected to Postgres, commands return a typed error instead of
//! pretending that a pin was durable.

use std::sync::Arc;

use handshake_core::{
    memory::{
        PinError, PinIpcService, PinReceipt, PinSubmitter, PinnedItem,
        PostgresKernelActionSubmitter, SetPinRequest, PIN_MEMORY_ACTION_ID, UNPIN_MEMORY_ACTION_ID,
    },
    storage::Database,
};
use tauri::State;
use uuid::Uuid;

enum MemoryPinBackend {
    Unavailable {
        reason: String,
    },
    Postgres {
        submitter: Arc<PostgresKernelActionSubmitter>,
    },
}

pub struct MemoryPinIpcState {
    backend: MemoryPinBackend,
}

impl Default for MemoryPinIpcState {
    fn default() -> Self {
        Self {
            backend: MemoryPinBackend::Unavailable {
                reason: "Postgres memory pin state has not been initialized".to_string(),
            },
        }
    }
}

impl MemoryPinIpcState {
    pub fn with_postgres(db: Arc<dyn Database>) -> Self {
        Self {
            backend: MemoryPinBackend::Postgres {
                submitter: Arc::new(PostgresKernelActionSubmitter::with_db(db)),
            },
        }
    }

    pub fn from_env_or_unavailable() -> Self {
        match tauri::async_runtime::block_on(handshake_core::storage::init_storage()) {
            Ok(db) => Self::with_postgres(db),
            Err(error) => Self {
                backend: MemoryPinBackend::Unavailable {
                    reason: format!("Postgres memory pin state unavailable: {error}"),
                },
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
                code: "memory_pin_postgres_unavailable".to_string(),
                reason: reason.clone(),
            }),
            MemoryPinBackend::Postgres { submitter } => submitter.set_pin(item),
        }
    }

    fn list_pinned(&self) -> Result<Vec<PinnedItem>, PinError> {
        match &self.backend {
            MemoryPinBackend::Unavailable { reason } => Err(PinError::Rejected {
                code: "memory_pin_postgres_unavailable".to_string(),
                reason: reason.clone(),
            }),
            MemoryPinBackend::Postgres { submitter } => submitter.list_pinned(),
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

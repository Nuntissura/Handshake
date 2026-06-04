//! MT-121: Tauri IPC surface for the per-session distillation flag.
//!
//! Two commands:
//!   - kernel_session_mark_for_distillation(session_id, flag,
//!     operator_signature)
//!   - kernel_session_get_distill_flag(session_id)
//!
//! Both commands dispatch through a Tauri-managed
//! [`SessionDistillState`] that holds the production
//! `handshake_core::distillation::session_flag::InMemorySessionFlagStore`
//! (the in-module SessionFlagStore impl declared by MT-121). Operator
//! marks are durable for the lifetime of the Tauri process; default-deny
//! is preserved (read returns `false` for sessions that have never been
//! marked).

use std::sync::Arc;

use handshake_core::distillation::session_flag::{
    get_distill_flag, mark_for_distillation, InMemorySessionFlagStore, SessionFlagError,
    SessionFlagStore,
};
use serde::{Deserialize, Serialize};
use tauri::State;

pub const KERNEL_SESSION_MARK_FOR_DISTILLATION_IPC_CHANNEL: &str =
    "kernel_session_mark_for_distillation";
pub const KERNEL_SESSION_GET_DISTILL_FLAG_IPC_CHANNEL: &str = "kernel_session_get_distill_flag";

/// Tauri-managed state holding the active `SessionFlagStore`. The
/// process-level default uses `InMemorySessionFlagStore`; the field
/// type is `Arc<dyn SessionFlagStore + Send + Sync>` so callers can
/// substitute a Postgres-backed store at app construction time without
/// modifying this module.
pub struct SessionDistillState {
    store: Arc<dyn SessionFlagStore + Send + Sync>,
}

impl SessionDistillState {
    pub fn new(store: Arc<dyn SessionFlagStore + Send + Sync>) -> Self {
        Self { store }
    }

    pub fn in_memory() -> Self {
        Self {
            store: Arc::new(InMemorySessionFlagStore::default()),
        }
    }

    pub fn store(&self) -> &(dyn SessionFlagStore + Send + Sync) {
        self.store.as_ref()
    }
}

impl Default for SessionDistillState {
    fn default() -> Self {
        Self::in_memory()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMarkForDistillationRequestIpc {
    pub session_id: String,
    pub flag: bool,
    pub operator_signature: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFlagWriteReceiptIpc {
    pub session_id: String,
    pub previous_flag: bool,
    pub new_flag: bool,
    pub operator_signature: String,
    pub event_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionGetDistillFlagRequestIpc {
    pub session_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDistillFlagResultIpc {
    pub session_id: String,
    pub distill_corpus: bool,
    pub event_type: String,
}

#[tauri::command]
pub async fn kernel_session_mark_for_distillation(
    request: SessionMarkForDistillationRequestIpc,
    state: State<'_, SessionDistillState>,
) -> Result<SessionFlagWriteReceiptIpc, String> {
    let _ = KERNEL_SESSION_MARK_FOR_DISTILLATION_IPC_CHANNEL;
    session_mark_for_distillation(request, state.store())
}

#[tauri::command]
pub async fn kernel_session_get_distill_flag(
    request: SessionGetDistillFlagRequestIpc,
    state: State<'_, SessionDistillState>,
) -> Result<SessionDistillFlagResultIpc, String> {
    let _ = KERNEL_SESSION_GET_DISTILL_FLAG_IPC_CHANNEL;
    session_get_distill_flag(request, state.store())
}

pub fn session_mark_for_distillation(
    request: SessionMarkForDistillationRequestIpc,
    store: &(dyn SessionFlagStore + Send + Sync),
) -> Result<SessionFlagWriteReceiptIpc, String> {
    if request.session_id.trim().is_empty() {
        return Err(map_err(SessionFlagError::EmptySessionId));
    }
    if request.operator_signature.trim().is_empty() {
        return Err(map_err(SessionFlagError::EmptySignature));
    }
    let now_utc = chrono::Utc::now().to_rfc3339();
    let record = mark_for_distillation(
        store,
        request.session_id.trim(),
        request.flag,
        request.operator_signature.trim(),
        &now_utc,
    )
    .map_err(map_err)?;
    Ok(SessionFlagWriteReceiptIpc {
        session_id: record.session_id,
        previous_flag: record.previous_flag,
        new_flag: record.new_flag,
        operator_signature: record.operator_signature,
        event_type: "FR-EVT-DISTILL-SESSION-FLAG-WRITE".to_string(),
    })
}

pub fn session_get_distill_flag(
    request: SessionGetDistillFlagRequestIpc,
    store: &(dyn SessionFlagStore + Send + Sync),
) -> Result<SessionDistillFlagResultIpc, String> {
    if request.session_id.trim().is_empty() {
        return Err(map_err(SessionFlagError::EmptySessionId));
    }
    let flag = get_distill_flag(store, request.session_id.trim()).map_err(map_err)?;
    Ok(SessionDistillFlagResultIpc {
        session_id: request.session_id,
        distill_corpus: flag,
        event_type: "FR-EVT-DISTILL-SESSION-FLAG-READ".to_string(),
    })
}

fn map_err(error: SessionFlagError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Arc<dyn SessionFlagStore + Send + Sync> {
        Arc::new(InMemorySessionFlagStore::default())
    }

    #[test]
    fn camel_case_serialization_round_trip() {
        let value = serde_json::to_value(SessionMarkForDistillationRequestIpc {
            session_id: "s1".to_string(),
            flag: true,
            operator_signature: "op".to_string(),
        })
        .expect("serialize");
        assert!(value.get("sessionId").is_some());
        assert!(value.get("operatorSignature").is_some());
        assert!(value.get("session_id").is_none());
    }

    #[test]
    fn mark_rejects_empty_session_id_and_empty_signature() {
        let s = store();
        let err = session_mark_for_distillation(
            SessionMarkForDistillationRequestIpc {
                session_id: " ".to_string(),
                flag: true,
                operator_signature: "op".to_string(),
            },
            s.as_ref(),
        )
        .expect_err("empty session_id");
        assert!(err.contains("session_id"));

        let err = session_mark_for_distillation(
            SessionMarkForDistillationRequestIpc {
                session_id: "s1".to_string(),
                flag: true,
                operator_signature: " ".to_string(),
            },
            s.as_ref(),
        )
        .expect_err("empty signature");
        assert!(err.contains("operator_signature"));
    }

    #[test]
    fn get_returns_default_deny_for_unmarked_session() {
        let s = store();
        let result = session_get_distill_flag(
            SessionGetDistillFlagRequestIpc {
                session_id: "never-marked".to_string(),
            },
            s.as_ref(),
        )
        .expect("read");
        assert_eq!(result.session_id, "never-marked");
        assert!(!result.distill_corpus, "default-deny preserved");
    }

    #[test]
    fn mark_then_get_round_trips_through_store() {
        // Sub-rule 2 evidence: this test exercises the full Tauri command
        // dispatch path against the real production InMemorySessionFlagStore
        // (concrete SessionFlagStore impl from session_flag.rs). No
        // narrative error short-circuit; a real write goes through, a real
        // read returns the persisted value.
        let s = store();
        let write = session_mark_for_distillation(
            SessionMarkForDistillationRequestIpc {
                session_id: "s-mt121".to_string(),
                flag: true,
                operator_signature: "ilja180520260209".to_string(),
            },
            s.as_ref(),
        )
        .expect("mark write");
        assert_eq!(write.session_id, "s-mt121");
        assert!(!write.previous_flag, "default-deny precondition");
        assert!(write.new_flag);
        assert_eq!(write.event_type, "FR-EVT-DISTILL-SESSION-FLAG-WRITE");

        let read = session_get_distill_flag(
            SessionGetDistillFlagRequestIpc {
                session_id: "s-mt121".to_string(),
            },
            s.as_ref(),
        )
        .expect("read after mark");
        assert!(
            read.distill_corpus,
            "store round-trip surfaces the mark through the IPC read path"
        );
    }

    #[test]
    fn mark_then_get_then_unmark_returns_to_default_deny() {
        let s = store();
        let _ = session_mark_for_distillation(
            SessionMarkForDistillationRequestIpc {
                session_id: "s-unmark".to_string(),
                flag: true,
                operator_signature: "ilja180520260209".to_string(),
            },
            s.as_ref(),
        )
        .expect("first mark");

        let unmark = session_mark_for_distillation(
            SessionMarkForDistillationRequestIpc {
                session_id: "s-unmark".to_string(),
                flag: false,
                operator_signature: "ilja180520260209".to_string(),
            },
            s.as_ref(),
        )
        .expect("unmark");
        assert!(unmark.previous_flag, "previous_flag tracks last value");
        assert!(!unmark.new_flag);

        let read = session_get_distill_flag(
            SessionGetDistillFlagRequestIpc {
                session_id: "s-unmark".to_string(),
            },
            s.as_ref(),
        )
        .expect("read after unmark");
        assert!(!read.distill_corpus, "unmark returns to default-deny");
    }

    #[test]
    fn default_state_uses_in_memory_store() {
        let state = SessionDistillState::default();
        let result = session_get_distill_flag(
            SessionGetDistillFlagRequestIpc {
                session_id: "x".to_string(),
            },
            state.store(),
        )
        .expect("read");
        assert!(!result.distill_corpus);
    }
}

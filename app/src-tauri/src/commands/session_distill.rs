//! MT-121: Tauri IPC surface for the per-session distillation flag.
//!
//! Two commands:
//!   - kernel_session_mark_for_distillation(session_id, flag,
//!     operator_signature)
//!   - kernel_session_get_distill_flag(session_id)
//!
//! Live wiring to the Postgres `governed_sessions.distill_corpus`
//! column is deferred to the same MT that lands the schema migration;
//! this surface validates inputs (default-deny + HBR-INT-006 signature
//! gate) and returns `live_runtime_unavailable` from the write path
//! until the storage adapter is attached. The read path returns the
//! default-deny value (`false`) until then so MT-119
//! `CorpusExtractor::extract` can already be wired into the UI without
//! risking accidental opt-in.

use handshake_core::distillation::session_flag::SessionFlagError;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::model_runtime::ModelRuntimeState;

pub const KERNEL_SESSION_MARK_FOR_DISTILLATION_IPC_CHANNEL: &str =
    "kernel_session_mark_for_distillation";
pub const KERNEL_SESSION_GET_DISTILL_FLAG_IPC_CHANNEL: &str = "kernel_session_get_distill_flag";

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
    _state: State<'_, ModelRuntimeState>,
) -> Result<SessionFlagWriteReceiptIpc, String> {
    let _ = KERNEL_SESSION_MARK_FOR_DISTILLATION_IPC_CHANNEL;
    session_mark_for_distillation(request)
}

#[tauri::command]
pub async fn kernel_session_get_distill_flag(
    request: SessionGetDistillFlagRequestIpc,
    _state: State<'_, ModelRuntimeState>,
) -> Result<SessionDistillFlagResultIpc, String> {
    let _ = KERNEL_SESSION_GET_DISTILL_FLAG_IPC_CHANNEL;
    session_get_distill_flag(request)
}

pub fn session_mark_for_distillation(
    request: SessionMarkForDistillationRequestIpc,
) -> Result<SessionFlagWriteReceiptIpc, String> {
    if request.session_id.trim().is_empty() {
        return Err(map_err(SessionFlagError::EmptySessionId));
    }
    if request.operator_signature.trim().is_empty() {
        return Err(map_err(SessionFlagError::EmptySignature));
    }
    // Live wiring to the SessionFlagStore (Postgres
    // governed_sessions.distill_corpus column) is deferred to the
    // schema-migration follow-on. The validation gates here mirror
    // distillation::session_flag::mark_for_distillation so callers see
    // the same error shape now and against the live store later.
    Err(format!(
        "session_distill live store is not attached for session {}; \
         validation passes but the governed_sessions.distill_corpus \
         column migration is pending. Default-deny is preserved.",
        request.session_id
    ))
}

pub fn session_get_distill_flag(
    request: SessionGetDistillFlagRequestIpc,
) -> Result<SessionDistillFlagResultIpc, String> {
    if request.session_id.trim().is_empty() {
        return Err(map_err(SessionFlagError::EmptySessionId));
    }
    // Default-deny is the right answer until the live store is wired;
    // returning `false` lets MT-119 `CorpusExtractor::extract` be
    // exercised through the UI without risking accidental opt-in.
    Ok(SessionDistillFlagResultIpc {
        session_id: request.session_id,
        distill_corpus: false,
        event_type: "FR-EVT-DISTILL-SESSION-FLAG-READ".to_string(),
    })
}

fn map_err(error: SessionFlagError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let err = session_mark_for_distillation(SessionMarkForDistillationRequestIpc {
            session_id: " ".to_string(),
            flag: true,
            operator_signature: "op".to_string(),
        })
        .expect_err("empty session_id");
        assert!(err.contains("session_id"));

        let err = session_mark_for_distillation(SessionMarkForDistillationRequestIpc {
            session_id: "s1".to_string(),
            flag: true,
            operator_signature: " ".to_string(),
        })
        .expect_err("empty signature");
        assert!(err.contains("operator_signature"));
    }

    #[test]
    fn get_returns_default_deny_until_store_wired() {
        let result = session_get_distill_flag(SessionGetDistillFlagRequestIpc {
            session_id: "s1".to_string(),
        })
        .expect("read");
        assert_eq!(result.session_id, "s1");
        assert!(!result.distill_corpus, "default-deny");
    }
}

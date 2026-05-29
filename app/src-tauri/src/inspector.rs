use std::sync::{Arc, Mutex};

use handshake_core::inspector_read::{
    EventLedgerRow, InspectorReadV1, ModelLoadedRow, ProcessRow, SessionId, SessionStateRead,
    SessionSummary, TraceProjection,
};
use tauri::State;

pub const KERNEL_INSPECTOR_PORT_IPC_CHANNEL: &str = "kernel.inspector.port";
pub const KERNEL_INSPECTOR_LIST_SESSIONS_IPC_CHANNEL: &str = "kernel.inspector.list_sessions";
pub const KERNEL_INSPECTOR_SESSION_STATE_IPC_CHANNEL: &str = "kernel.inspector.session_state";
pub const KERNEL_INSPECTOR_EVENT_LEDGER_TAIL_IPC_CHANNEL: &str =
    "kernel.inspector.event_ledger_tail";
pub const KERNEL_INSPECTOR_PROCESS_LEDGER_ACTIVE_IPC_CHANNEL: &str =
    "kernel.inspector.process_ledger_active";
pub const KERNEL_INSPECTOR_TRACE_PROJECTION_IPC_CHANNEL: &str = "kernel.inspector.trace_projection";
pub const KERNEL_INSPECTOR_LOADED_MODELS_IPC_CHANNEL: &str = "kernel.inspector.loaded_models";

#[derive(Debug, Default)]
pub struct InspectorPortState {
    port: Mutex<Option<u16>>,
}

impl InspectorPortState {
    pub fn new(port: Option<u16>) -> Self {
        Self {
            port: Mutex::new(port),
        }
    }

    pub fn port(&self) -> Result<Option<u16>, String> {
        self.port
            .lock()
            .map(|guard| *guard)
            .map_err(|_| "inspector port state mutex poisoned".to_string())
    }
}

#[tauri::command]
pub fn kernel_inspector_port(state: State<'_, InspectorPortState>) -> Result<Option<u16>, String> {
    let _ = KERNEL_INSPECTOR_PORT_IPC_CHANNEL;
    state.port()
}

#[tauri::command]
pub fn kernel_inspector_list_sessions(
    reader: State<'_, Arc<dyn InspectorReadV1>>,
) -> Vec<SessionSummary> {
    let _ = KERNEL_INSPECTOR_LIST_SESSIONS_IPC_CHANNEL;
    reader.list_sessions()
}

#[tauri::command]
pub fn kernel_inspector_session_state(
    reader: State<'_, Arc<dyn InspectorReadV1>>,
    session_id: SessionId,
) -> Option<SessionStateRead> {
    let _ = KERNEL_INSPECTOR_SESSION_STATE_IPC_CHANNEL;
    reader.session_state(session_id)
}

#[tauri::command]
pub fn kernel_inspector_event_ledger_tail(
    reader: State<'_, Arc<dyn InspectorReadV1>>,
    n: usize,
) -> Vec<EventLedgerRow> {
    let _ = KERNEL_INSPECTOR_EVENT_LEDGER_TAIL_IPC_CHANNEL;
    reader.event_ledger_tail(n)
}

#[tauri::command]
pub fn kernel_inspector_process_ledger_active(
    reader: State<'_, Arc<dyn InspectorReadV1>>,
) -> Vec<ProcessRow> {
    let _ = KERNEL_INSPECTOR_PROCESS_LEDGER_ACTIVE_IPC_CHANNEL;
    reader.process_ledger_active()
}

#[tauri::command]
pub fn kernel_inspector_trace_projection(
    reader: State<'_, Arc<dyn InspectorReadV1>>,
    session_id: SessionId,
) -> Option<TraceProjection> {
    let _ = KERNEL_INSPECTOR_TRACE_PROJECTION_IPC_CHANNEL;
    reader.trace_projection(session_id)
}

#[tauri::command]
pub fn kernel_inspector_loaded_models(
    reader: State<'_, Arc<dyn InspectorReadV1>>,
) -> LoadedModelsRead {
    let _ = KERNEL_INSPECTOR_LOADED_MODELS_IPC_CHANNEL;
    reader.loaded_models()
}

type LoadedModelsRead = Vec<ModelLoadedRow>;

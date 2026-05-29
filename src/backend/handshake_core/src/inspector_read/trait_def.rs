use super::trace_projection::TraceProjection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WorkspaceId(pub String);

impl WorkspaceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: SessionId,
    pub state: String,
    pub model_id: Option<String>,
    pub active_process_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateRead {
    pub id: SessionId,
    pub state: String,
    pub latest_event_id: Option<String>,
    pub active_process_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EventLedgerRow {
    pub event_id: String,
    pub event_type: String,
    pub event_sequence: i64,
    pub created_at_utc: String,
    #[serde(default)]
    pub event_version: String,
    #[serde(default)]
    pub kernel_task_run_id: String,
    #[serde(default)]
    pub session_run_id: String,
    #[serde(default)]
    pub aggregate_type: String,
    #[serde(default)]
    pub aggregate_id: String,
    #[serde(default)]
    pub idempotency_key: String,
    #[serde(default)]
    pub actor_kind: String,
    #[serde(default)]
    pub actor_id: String,
    #[serde(default)]
    pub causation_id: Option<String>,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub payload_hash: String,
    #[serde(default)]
    pub source_component: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRow {
    pub process_uuid: String,
    pub session_id: SessionId,
    pub engine_kind: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceStateRead {
    pub workspace_id: WorkspaceId,
    pub state_vector: String,
    pub last_update_id: Option<String>,
    pub readable_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLoadedRow {
    pub model_id: String,
    pub adapter_id: String,
    pub process_uuid: Option<String>,
    pub loaded_at_utc: Option<String>,
}

pub trait InspectorReadV1: Send + Sync {
    fn list_sessions(&self) -> Vec<SessionSummary>;
    fn session_state(&self, id: SessionId) -> Option<SessionStateRead>;
    fn event_ledger_tail(&self, n: usize) -> Vec<EventLedgerRow>;
    fn process_ledger_active(&self) -> Vec<ProcessRow>;
    fn workspace_state_read(&self, ws_id: WorkspaceId) -> Option<WorkspaceStateRead>;
    fn trace_projection(&self, session_id: SessionId) -> Option<TraceProjection>;
    fn loaded_models(&self) -> Vec<ModelLoadedRow>;
}

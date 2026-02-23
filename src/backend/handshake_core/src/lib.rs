pub mod ace;
pub mod ai_ready_data;
pub mod api;
pub mod bundles;
pub mod capabilities;
pub mod capability_registry_workflow;
pub mod diagnostics;
pub mod flight_recorder;
pub mod governance_pack;
pub mod jobs;
pub mod llm;
pub mod logging;
pub mod loom_fs;
pub mod mcp;
pub mod mex;
pub mod models;
pub mod role_mailbox;
pub mod runtime_governance;
pub mod storage;
pub mod terminal;
pub mod tokenization;
pub mod workflows;

use std::sync::Arc;

use crate::diagnostics::DiagnosticsStore;
use crate::flight_recorder::FlightRecorder;
use crate::llm::LlmClient;
use crate::storage::Database;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Database>,
    pub flight_recorder: Arc<dyn FlightRecorder>,
    pub diagnostics: Arc<dyn DiagnosticsStore>,
    pub llm_client: Arc<dyn LlmClient>,
    pub capability_registry: Arc<capabilities::CapabilityRegistry>,
}

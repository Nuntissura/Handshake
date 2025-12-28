pub mod ace;
pub mod api;
pub mod capabilities;
pub mod flight_recorder;
pub mod jobs;
pub mod llm;
pub mod logging;
pub mod mex;
pub mod models;
pub mod storage;
pub mod terminal;
pub mod tokenization;
pub mod workflows;

use std::sync::Arc;

use crate::flight_recorder::FlightRecorder;
use crate::llm::LlmClient;
use crate::storage::Database;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Database>,
    pub flight_recorder: Arc<dyn FlightRecorder>,
    pub llm_client: Arc<dyn LlmClient>,
    pub capability_registry: Arc<capabilities::CapabilityRegistry>,
}

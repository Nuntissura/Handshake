pub mod api;
pub mod capabilities;
pub mod flight_recorder;
pub mod jobs;
pub mod llm;
pub mod logging;
pub mod models;
pub mod storage;
pub mod terminal;
pub mod tokenization;
pub mod workflows;

use duckdb::Connection as DuckDbConnection;
use std::sync::{Arc, Mutex};

use crate::llm::LLMClient;
use crate::storage::Database;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Database>,
    pub fr_pool: Arc<Mutex<DuckDbConnection>>,
    pub llm_client: Arc<dyn LLMClient>,
    pub capability_registry: Arc<capabilities::CapabilityRegistry>,
}

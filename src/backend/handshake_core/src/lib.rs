#[cfg(all(feature = "runtime-full", not(feature = "duckdb-flight-recorder")))]
extern crate self as duckdb;

#[cfg(all(feature = "runtime-full", not(feature = "duckdb-flight-recorder")))]
mod duckdb_build_stub {
    use std::{fmt, marker::PhantomData, path::Path};

    #[derive(Debug, Clone)]
    pub enum Error {
        QueryReturnedNoRows,
        InvalidParameterName(String),
        Unavailable(String),
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::QueryReturnedNoRows => write!(f, "duckdb row not found"),
                Self::InvalidParameterName(value) => {
                    write!(f, "invalid duckdb parameter: {value}")
                }
                Self::Unavailable(value) => write!(f, "duckdb feature unavailable: {value}"),
            }
        }
    }

    impl std::error::Error for Error {}

    pub type Result<T> = std::result::Result<T, Error>;

    pub trait ToSql {}

    impl<T> ToSql for T {}

    #[derive(Debug, Default)]
    pub struct Connection;

    impl Connection {
        pub fn open<P: AsRef<Path>>(_path: P) -> Result<Self> {
            Err(Error::Unavailable(
                "enable duckdb-flight-recorder to open DuckDB".to_string(),
            ))
        }

        pub fn open_in_memory() -> Result<Self> {
            Err(Error::Unavailable(
                "enable duckdb-flight-recorder to open DuckDB".to_string(),
            ))
        }

        pub fn execute<P>(&self, _sql: &str, _params: P) -> Result<usize> {
            Err(Error::Unavailable(
                "enable duckdb-flight-recorder to execute DuckDB SQL".to_string(),
            ))
        }

        pub fn execute_batch(&self, _sql: &str) -> Result<()> {
            Err(Error::Unavailable(
                "enable duckdb-flight-recorder to execute DuckDB SQL".to_string(),
            ))
        }

        pub fn prepare(&self, _sql: &str) -> Result<Statement> {
            Err(Error::Unavailable(
                "enable duckdb-flight-recorder to prepare DuckDB SQL".to_string(),
            ))
        }
    }

    pub struct Statement;

    impl Statement {
        pub fn execute<P>(&mut self, _params: P) -> Result<usize> {
            Err(Error::Unavailable(
                "enable duckdb-flight-recorder to execute DuckDB SQL".to_string(),
            ))
        }

        pub fn query<P>(&mut self, _params: P) -> Result<Rows<'static>> {
            Err(Error::Unavailable(
                "enable duckdb-flight-recorder to query DuckDB".to_string(),
            ))
        }

        pub fn query_row<P, F, T>(&mut self, _params: P, _f: F) -> Result<T>
        where
            F: FnOnce(&Row<'_>) -> Result<T>,
        {
            Err(Error::QueryReturnedNoRows)
        }

        pub fn query_map<P, F, T>(&mut self, _params: P, _f: F) -> Result<Vec<Result<T>>>
        where
            F: FnMut(&Row<'_>) -> Result<T>,
        {
            Ok(Vec::new())
        }
    }

    pub struct Rows<'stmt> {
        _marker: PhantomData<&'stmt ()>,
    }

    impl Rows<'_> {
        pub fn next(&mut self) -> Result<Option<&Row<'_>>> {
            Ok(None)
        }
    }

    pub struct Row<'stmt> {
        _marker: PhantomData<&'stmt ()>,
    }

    impl Row<'_> {
        pub fn get<I, T: Default>(&self, _idx: I) -> Result<T> {
            Ok(T::default())
        }
    }

    pub fn params_from_iter<I>(_params: I) {}
}

#[cfg(all(feature = "runtime-full", not(feature = "duckdb-flight-recorder")))]
pub use duckdb_build_stub::{params_from_iter, Connection, Error, Result, Row, ToSql};

#[cfg(all(feature = "runtime-full", not(feature = "duckdb-flight-recorder")))]
#[macro_export]
macro_rules! params {
    ($($value:expr),* $(,)?) => {{
        let _ = ($( &$value ),*);
    }};
}

#[cfg(feature = "runtime-full")]
pub mod ace;
#[cfg(feature = "runtime-full")]
pub mod ai_ready_data;
#[cfg(feature = "runtime-full")]
pub mod api;
#[cfg(feature = "runtime-full")]
pub mod bundles;
#[cfg(feature = "runtime-full")]
pub mod capabilities;
#[cfg(feature = "runtime-full")]
pub mod capability_registry_workflow;
#[cfg(feature = "runtime-full")]
pub mod diagnostics;
#[cfg(feature = "runtime-full")]
pub mod distillation;
#[cfg(feature = "runtime-full")]
pub mod flight_recorder;
#[cfg(feature = "runtime-full")]
pub mod governance_artifact_registry;
#[cfg(feature = "runtime-full")]
pub mod governance_check_runner;
#[cfg(feature = "runtime-full")]
pub mod governance_pack;
#[cfg(feature = "runtime-full")]
pub mod inspector_read;
#[cfg(feature = "runtime-full")]
pub mod jobs;
pub mod kernel;
#[cfg(feature = "runtime-full")]
pub mod llm;
#[cfg(feature = "runtime-full")]
pub mod logging;
#[cfg(feature = "runtime-full")]
pub mod loom_fs;
#[cfg(feature = "runtime-full")]
pub mod mcp;
#[cfg(feature = "runtime-full")]
pub mod memory;
#[cfg(feature = "runtime-full")]
pub mod mex;
#[cfg(feature = "runtime-full")]
pub mod model_manual;
#[cfg(feature = "runtime-full")]
pub mod model_runtime;
#[cfg(feature = "runtime-full")]
pub mod models;
#[cfg(feature = "runtime-full")]
pub mod process_ledger;
#[cfg(feature = "runtime-full")]
pub mod sandbox;
#[cfg(feature = "runtime-full")]
pub mod role_mailbox;
#[cfg(feature = "runtime-full")]
pub mod runtime_governance;
#[cfg(feature = "runtime-full")]
pub mod storage;
#[cfg(feature = "runtime-full")]
pub mod terminal;
#[cfg(feature = "runtime-full")]
pub mod test_harness;
#[cfg(feature = "tokenization")]
pub mod tokenization;
#[cfg(feature = "runtime-full")]
pub mod workflows;
#[cfg(feature = "runtime-full")]
pub mod workspace_safety;

#[cfg(feature = "runtime-full")]
use std::sync::Arc;

#[cfg(feature = "runtime-full")]
use crate::diagnostics::DiagnosticsStore;
#[cfg(feature = "runtime-full")]
use crate::flight_recorder::FlightRecorder;
#[cfg(feature = "runtime-full")]
use crate::llm::LlmClient;
#[cfg(feature = "runtime-full")]
use crate::storage::Database;

#[cfg(feature = "runtime-full")]
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Database>,
    pub flight_recorder: Arc<dyn FlightRecorder>,
    pub diagnostics: Arc<dyn DiagnosticsStore>,
    pub llm_client: Arc<dyn LlmClient>,
    pub capability_registry: Arc<capabilities::CapabilityRegistry>,
    pub session_registry: Arc<workflows::SessionRegistry>,
}

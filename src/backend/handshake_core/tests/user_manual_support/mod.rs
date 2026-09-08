//! Shared support for the WP-KERNEL-009 UserManual integration tests
//! (MT-193..MT-208): a real embedded-SurrealDB AppState + loopback server over
//! the actual Axum routers. No server, fallback database, or mock store is
//! involved: every fixture owns an isolated embedded store and the AppState
//! shares that same handle.

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::tests::{embedded_test_backend, EmbeddedTestBackend};
use handshake_core::storage::{Database, NewWorkspace, StorageResult, WriteContext};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use uuid::Uuid;

#[derive(Default)]
pub struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }
    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }
    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diag: Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }
    async fn list_problems(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
    async fn get_diagnostic(
        &self,
        _id: Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
    }
    async fn list_diagnostics(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
}

pub struct NoopLlmClient {
    profile: ModelProfile,
}

#[async_trait]
impl LlmClient for NoopLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }
    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

/// A concrete database handle over one isolated embedded test store.
///
/// The retained backend owns the store directory and lifecycle handle while
/// `db` exposes the concrete Surreal type required by UserManual and knowledge
/// consumers.
pub struct ManualTestBackend {
    pub db: SurrealDatabase,
    _backend: EmbeddedTestBackend,
}

impl ManualTestBackend {
    pub async fn create_workspace(&self) -> String {
        self.db
            .create_workspace(
                &WriteContext::system(Some("user-manual-test".to_owned())),
                NewWorkspace {
                    name: format!("user-manual-test-{}", Uuid::now_v7()),
                },
            )
            .await
            .expect("create UserManual test workspace")
            .id
    }

    /// Close the embedded engine on the test runtime before that runtime is
    /// torn down, then prove that its isolated RocksDB directory is removable.
    /// The backend's synchronous `Drop` cleanup remains a failure-path fallback;
    /// successful tests must use this explicit path so engine tasks can drain.
    pub async fn close_and_remove(self) -> StorageResult<()> {
        let ManualTestBackend { db, _backend } = self;
        drop(db);
        _backend.close_and_remove().await
    }
}

/// Open a mandatory real embedded store. Failure is a test failure, never a
/// reason to skip the proof or fall back to another database.
pub async fn manual_test_backend() -> StorageResult<ManualTestBackend> {
    let backend = embedded_test_backend().await?;
    let db = SurrealDatabase::new(backend.storage.clone());
    Ok(ManualTestBackend {
        db,
        _backend: backend,
    })
}

/// Build a real AppState sharing the exact embedded store already used by the
/// test's concrete database handle.
pub async fn app_state_for(db: &SurrealDatabase) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: Arc::new(db.clone()),
        surreal: db.storage().clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("user-manual-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

/// Serve a router on a loopback listener (quiet: no foreground window).
pub async fn start_server(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("test api server");
    });
    (format!("http://{addr}"), server)
}

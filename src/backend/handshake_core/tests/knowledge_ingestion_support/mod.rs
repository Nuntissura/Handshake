//! Shared embedded-SurrealDB support for knowledge and Loom integration tests.
//!
//! Every fixture owns one real on-disk embedded store created by the canonical
//! `embedded_test_backend` helper. The fixture names describe the embedded
//! authority directly and expose no server URL or raw query connection.
#![allow(dead_code)]

use std::sync::Arc;

use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_ingestion::engine::{IngestionContext, IngestionEngine};
use handshake_core::storage::surreal::SurrealStorageConfig;
use handshake_core::storage::surreal::{SurrealDatabase, SurrealStorage};
use handshake_core::storage::tests::{embedded_test_backend, EmbeddedTestBackend};
use handshake_core::storage::{Database, NewWorkspace, StorageError, StorageResult, WriteContext};
use uuid::Uuid;

/// Embedded fixture for knowledge/Loom tests.
///
/// The store is real and on disk for the lifetime of this value. Its cleanup
/// guard is retained in `backend`, so dropping the fixture closes and removes
/// the isolated data directory after all derived handles have been dropped.
pub struct EmbeddedKnowledgeStore {
    pub db: SurrealDatabase,
    pub storage: SurrealStorage,
    pub data_dir: std::path::PathBuf,
    backend: EmbeddedTestBackend,
}

impl EmbeddedKnowledgeStore {
    pub async fn create_workspace(&self) -> String {
        self.db
            .create_workspace(
                &WriteContext::human(None),
                NewWorkspace {
                    name: format!("knowledge-ws-{}", Uuid::now_v7()),
                },
            )
            .await
            .expect("create workspace for embedded knowledge test")
            .id
    }

    pub fn database(&self) -> Arc<dyn Database> {
        Arc::new(self.db.clone())
    }

    /// Close the shared embedded handle before a durability/restart proof.
    pub async fn shutdown(&self) -> StorageResult<()> {
        self.storage
            .shutdown()
            .await
            .map_err(|error| StorageError::Database(error.to_string()))
    }

    /// Reopen the same on-disk store after `shutdown`.
    pub async fn reopen_database(&self) -> StorageResult<SurrealDatabase> {
        let config = SurrealStorageConfig::for_data_dir(&self.data_dir)
            .map_err(|error| StorageError::Database(error.to_string()))?;
        let storage = handshake_core::storage::surreal::SurrealStorage::open(config)
            .await
            .map_err(|error| StorageError::Database(error.to_string()))?;
        Ok(SurrealDatabase::new(storage))
    }

    pub async fn close_and_remove(self) -> StorageResult<()> {
        let EmbeddedKnowledgeStore {
            db,
            storage,
            backend,
            ..
        } = self;
        drop(db);
        drop(storage);
        backend.close_and_remove().await
    }
}

/// Open a mandatory real embedded store. The `None` shape is retained for
/// callers whose old helper used loud skip branches; the embedded path either
/// returns `Some` or panics with the setup failure.
pub async fn open_embedded_store() -> Option<EmbeddedKnowledgeStore> {
    let backend = embedded_test_backend()
        .await
        .expect("open isolated embedded knowledge test store");
    let storage = backend.storage.clone();
    let data_dir = backend.data_dir.clone();
    let db = SurrealDatabase::new(storage.clone());
    Some(EmbeddedKnowledgeStore {
        db,
        storage,
        data_dir,
        backend,
    })
}

/// Shared ingestion test environment over the same embedded authority handle.
pub struct EmbeddedIngestionFixture {
    pub store: EmbeddedKnowledgeStore,
    pub engine: IngestionEngine,
}

pub async fn open_embedded_ingestion_fixture() -> Option<EmbeddedIngestionFixture> {
    let store = open_embedded_store().await?;
    let engine = IngestionEngine::from_database(Arc::new(store.db.clone()));
    Some(EmbeddedIngestionFixture { store, engine })
}

/// Backend-navigation context for tests (actor/session/correlation metadata).
pub fn test_ctx(label: &str) -> IngestionContext {
    let suffix = Uuid::now_v7();
    IngestionContext {
        actor: KernelActor::System(format!("ingestion-test-{label}")),
        kernel_task_run_id: format!("KTR-INGEST-{suffix}"),
        session_run_id: format!("SR-INGEST-{suffix}"),
        correlation_id: Some(format!("CORR-INGEST-{suffix}")),
    }
}

/// Register a root under the default allowlist policy.
pub async fn register_root(
    env: &EmbeddedIngestionFixture,
    ctx: &IngestionContext,
    workspace_id: &str,
    repo_relative_path: &str,
    root_kind: handshake_core::storage::knowledge::KnowledgeRootKind,
) -> handshake_core::storage::knowledge::KnowledgeSourceRoot {
    use handshake_core::knowledge_ingestion::engine::RootRegistrationRequest;
    let (root, _decision) = env
        .engine
        .register_root(
            ctx,
            RootRegistrationRequest {
                workspace_id: workspace_id.to_owned(),
                display_name: if repo_relative_path.is_empty() {
                    "test root (repo)".to_owned()
                } else {
                    format!("test root {repo_relative_path}")
                },
                root_kind,
                repo_relative_path: repo_relative_path.to_owned(),
                file_allowlist_policy: serde_json::json!({"include": ["**/*"], "exclude": []}),
                operator_approved: true,
            },
        )
        .await
        .expect("register embedded test root");
    root
}

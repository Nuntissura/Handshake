use std::sync::Arc;

use handshake_core::atelier::AtelierStore;
use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
};
use handshake_core::storage::Database;

/// One isolated, fully bootstrapped embedded-SurrealDB authority for Atelier
/// integration tests. Keeping the temporary directory in this value guarantees
/// that the on-disk engine outlives every handle derived from it.
pub struct AtelierSurrealHarness {
    pub atelier: AtelierStore,
    pub database: Arc<dyn Database>,
    pub storage: SurrealStorage,
    _directory: tempfile::TempDir,
}

impl AtelierSurrealHarness {
    pub async fn create() -> Self {
        let directory = tempfile::tempdir().expect("create isolated Atelier SurrealDB root");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::for_data_dir(directory.path())
                .expect("configure isolated Atelier SurrealDB"),
        )
        .await
        .expect("open isolated Atelier SurrealDB");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap isolated Atelier SurrealDB schema");
        let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(storage.clone()));
        let atelier = AtelierStore::with_event_ledger(storage.clone(), database.clone());
        atelier
            .ensure_schema()
            .await
            .expect("verify isolated Atelier SurrealDB schema");
        Self {
            atelier,
            database,
            storage,
            _directory: directory,
        }
    }

    pub async fn shutdown(self) {
        self.storage
            .shutdown()
            .await
            .expect("close isolated Atelier SurrealDB");
    }
}

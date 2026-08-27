use std::{
    path::PathBuf,
    sync::{Arc, Mutex as StdMutex},
};

use uuid::Uuid;

use super::{SurrealDatabase, SurrealStorage, SurrealStorageConfig};
use crate::storage::{Database, StorageError, StorageResult};

fn checked_subdirectory(
    parent: &std::path::Path,
    name: &str,
    artifacts_root: &std::path::Path,
) -> StorageResult<PathBuf> {
    let candidate = parent.join(name);
    if !candidate.exists() {
        std::fs::create_dir(&candidate).map_err(|error| {
            StorageError::Database(format!(
                "could not create MT-136 proof directory {}: {error}",
                candidate.display()
            ))
        })?;
    }
    let resolved = dunce::canonicalize(&candidate).map_err(|error| {
        StorageError::Database(format!(
            "could not resolve MT-136 proof directory {}: {error}",
            candidate.display()
        ))
    })?;
    if !resolved.starts_with(artifacts_root) {
        return Err(StorageError::Database(format!(
            "MT-136 proof directory escaped HANDSHAKE_ARTIFACTS_ROOT: {}",
            resolved.display()
        )));
    }
    Ok(resolved)
}

#[derive(Clone)]
pub(crate) struct EmbeddedProofBackend {
    pub(crate) database: Arc<dyn Database>,
    pub(crate) storage: SurrealStorage,
    pub(crate) data_dir: PathBuf,
    cleanup: Arc<ProofCleanupGuard>,
}

struct ProofCleanupGuard {
    storage: StdMutex<Option<SurrealStorage>>,
    data_dir: PathBuf,
}

impl ProofCleanupGuard {
    fn take_storage(&self) -> Option<SurrealStorage> {
        match self.storage.lock() {
            Ok(mut storage) => storage.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        }
    }

    async fn cleanup(&self) -> StorageResult<()> {
        let Some(storage) = self.take_storage() else {
            return Ok(());
        };
        verified_shutdown_and_remove(storage, self.data_dir.clone()).await
    }

    fn replace_storage(&self, storage: SurrealStorage) {
        match self.storage.lock() {
            Ok(mut current) => *current = Some(storage),
            Err(poisoned) => *poisoned.into_inner() = Some(storage),
        }
    }
}

impl Drop for ProofCleanupGuard {
    fn drop(&mut self) {
        let Some(storage) = self.take_storage() else {
            return;
        };
        let data_dir = self.data_dir.clone();
        let cleanup = std::thread::Builder::new()
            .name("mt136-proof-cleanup".to_owned())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| {
                        StorageError::Database(format!(
                            "could not build MT-136 cleanup runtime: {error}"
                        ))
                    })?;
                runtime.block_on(verified_shutdown_and_remove(storage, data_dir))
            });
        let result = match cleanup {
            Ok(thread) => match thread.join() {
                Ok(result) => result,
                Err(_) => Err(StorageError::Database(
                    "MT-136 proof cleanup thread panicked".to_owned(),
                )),
            },
            Err(error) => Err(StorageError::Database(format!(
                "could not start MT-136 proof cleanup thread: {error}"
            ))),
        };
        if let Err(error) = result {
            eprintln!("MT136_PROOF_CLEANUP_FAILURE {error}");
        }
    }
}

impl EmbeddedProofBackend {
    pub(crate) async fn reopen(self) -> StorageResult<Self> {
        let Self {
            database,
            storage,
            data_dir,
            cleanup,
        } = self;
        drop(database);
        storage
            .shutdown()
            .await
            .map_err(|error| StorageError::Database(error.to_string()))?;
        drop(storage);

        let config = SurrealStorageConfig::for_data_dir(&data_dir)
            .map_err(|error| StorageError::Database(error.to_string()))?;
        let storage = SurrealStorage::open(config)
            .await
            .map_err(|error| StorageError::Database(error.to_string()))?;
        cleanup.replace_storage(storage.clone());
        let database = SurrealDatabase::new(storage.clone());
        database.run_migrations().await?;
        Ok(Self {
            database: Arc::new(database),
            storage,
            data_dir,
            cleanup,
        })
    }

    pub(crate) async fn close_and_remove(self) -> StorageResult<()> {
        let Self {
            database,
            storage,
            data_dir: _,
            cleanup,
        } = self;
        drop(database);
        drop(storage);
        cleanup.cleanup().await
    }
}

pub(crate) async fn verified_shutdown_and_remove(
    storage: SurrealStorage,
    data_dir: PathBuf,
) -> StorageResult<()> {
    let shutdown = storage.shutdown().await;
    drop(storage);
    let cleanup = std::fs::remove_dir_all(&data_dir);

    let mut failures = Vec::new();
    if let Err(error) = shutdown {
        failures.push(format!("shutdown failed: {error}"));
    }
    if let Err(error) = cleanup {
        if error.kind() != std::io::ErrorKind::NotFound {
            failures.push(format!(
                "cleanup failed for {}: {error}",
                data_dir.display()
            ));
        }
    }
    match data_dir.try_exists() {
        Ok(false) => {}
        Ok(true) => failures.push(format!(
            "cleanup reported success but proof store still exists: {}",
            data_dir.display()
        )),
        Err(error) => failures.push(format!(
            "could not verify proof-store removal for {}: {error}",
            data_dir.display()
        )),
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(StorageError::Database(failures.join("; ")))
    }
}

pub(crate) async fn embedded_proof_backend() -> StorageResult<EmbeddedProofBackend> {
    let configured_root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT").ok_or_else(|| {
        StorageError::Database(
            "HANDSHAKE_ARTIFACTS_ROOT must name the absolute _Artifacts root for MT-136 proofs"
                .to_owned(),
        )
    })?;
    let configured_root = PathBuf::from(configured_root);
    if !configured_root.is_absolute() {
        return Err(StorageError::Database(format!(
            "HANDSHAKE_ARTIFACTS_ROOT must be absolute, got {}",
            configured_root.display()
        )));
    }
    std::fs::create_dir_all(&configured_root).map_err(|error| {
        StorageError::Database(format!(
            "could not create MT-136 artifacts root {}: {error}",
            configured_root.display()
        ))
    })?;
    let artifacts_root = dunce::canonicalize(&configured_root).map_err(|error| {
        StorageError::Database(format!(
            "could not resolve MT-136 artifacts root {}: {error}",
            configured_root.display()
        ))
    })?;
    let test_root = checked_subdirectory(&artifacts_root, "handshake-test", &artifacts_root)?;
    let stores_root = checked_subdirectory(&test_root, "mt136-surface-proofs", &artifacts_root)?;
    let data_dir = stores_root.join(format!("store-{}", Uuid::now_v7().simple()));
    std::fs::create_dir_all(&data_dir).map_err(|error| {
        StorageError::Database(format!(
            "could not create MT-136 embedded proof store {}: {error}",
            data_dir.display()
        ))
    })?;

    let config = match SurrealStorageConfig::for_data_dir(&data_dir) {
        Ok(config) => config,
        Err(error) => {
            let cleanup = std::fs::remove_dir_all(&data_dir);
            return match cleanup {
                Ok(()) => Err(StorageError::Database(error.to_string())),
                Err(cleanup_error) => Err(StorageError::Database(format!(
                    "{error}; additionally could not remove failed MT-136 proof store {}: {cleanup_error}",
                    data_dir.display()
                ))),
            };
        }
    };
    let storage = match SurrealStorage::open(config).await {
        Ok(storage) => storage,
        Err(error) => {
            let cleanup = std::fs::remove_dir_all(&data_dir);
            return match cleanup {
                Ok(()) => Err(StorageError::Database(error.to_string())),
                Err(cleanup_error) => Err(StorageError::Database(format!(
                    "{error}; additionally could not remove failed MT-136 proof store {}: {cleanup_error}",
                    data_dir.display()
                ))),
            };
        }
    };
    let database = SurrealDatabase::new(storage.clone());
    if let Err(error) = database.run_migrations().await {
        drop(database);
        let shutdown = storage.shutdown().await;
        drop(storage);
        let cleanup = std::fs::remove_dir_all(&data_dir);
        let mut message = error.to_string();
        if let Err(shutdown_error) = shutdown {
            message.push_str(&format!("; shutdown failed: {shutdown_error}"));
        }
        if let Err(cleanup_error) = cleanup {
            message.push_str(&format!(
                "; cleanup failed for {}: {cleanup_error}",
                data_dir.display()
            ));
        }
        return Err(StorageError::Database(message));
    }

    let cleanup = Arc::new(ProofCleanupGuard {
        storage: StdMutex::new(Some(storage.clone())),
        data_dir: data_dir.clone(),
    });
    Ok(EmbeddedProofBackend {
        database: Arc::new(database),
        storage,
        data_dir,
        cleanup,
    })
}

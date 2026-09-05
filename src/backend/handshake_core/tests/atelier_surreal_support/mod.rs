use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::Utc;
use handshake_core::atelier::AtelierStore;
use handshake_core::storage::artifacts::{
    artifact_root_dir, write_file_artifact, ArtifactClassification, ArtifactLayer,
    ArtifactManifest, ArtifactPayloadKind,
};
use handshake_core::storage::surreal::{
    bootstrap_schema, RowFilter, ScalarValue, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
};
use handshake_core::storage::Database;
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    hex::encode(digest.finalize())
}

/// Native file-artifact fixture shared by Atelier integration tests.
#[derive(Debug, Clone)]
pub struct NativeMediaArtifact {
    pub artifact_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub workspace_root: PathBuf,
    pub payload_path: PathBuf,
    pub manifest_path: PathBuf,
    pub stored_payload: Vec<u8>,
}

pub fn write_native_media_artifact(payload: &[u8]) -> NativeMediaArtifact {
    let workspace_root = tempfile::tempdir()
        .expect("create isolated native artifact workspace")
        .keep();
    write_native_media_artifact_in_workspace(&workspace_root, payload)
}

pub fn write_native_media_artifact_in_workspace(
    workspace_root: &Path,
    payload: &[u8],
) -> NativeMediaArtifact {
    fs::create_dir_all(workspace_root).expect("create native artifact workspace");
    let artifact_id = Uuid::now_v7();
    let content_hash = sha256_hex(payload);
    let manifest = ArtifactManifest {
        artifact_id,
        layer: ArtifactLayer::L1,
        kind: ArtifactPayloadKind::File,
        mime: "image/png".to_string(),
        filename_hint: Some("fixture.png".to_string()),
        created_at: Utc::now(),
        created_by_job_id: None,
        source_entity_refs: Vec::new(),
        source_artifact_refs: Vec::new(),
        content_hash: content_hash.clone(),
        size_bytes: payload.len() as u64,
        classification: ArtifactClassification::Low,
        exportable: true,
        retention_ttl_days: None,
        pinned: None,
        hash_basis: None,
        hash_exclude_paths: Vec::new(),
    };
    write_file_artifact(workspace_root, &manifest, payload)
        .expect("write native artifact payload and manifest");
    let root = artifact_root_dir(workspace_root, ArtifactLayer::L1, artifact_id);
    NativeMediaArtifact {
        artifact_ref: format!("artifact://.handshake/artifacts/L1/{artifact_id}/payload"),
        content_hash,
        byte_len: payload.len() as i64,
        workspace_root: workspace_root.to_path_buf(),
        payload_path: root.join("payload"),
        manifest_path: root.join("artifact.json"),
        stored_payload: payload.to_vec(),
    }
}

pub fn write_native_media_artifact_from_stored_payload(payload: &[u8]) -> NativeMediaArtifact {
    write_native_media_artifact(payload)
}

/// One isolated, fully bootstrapped embedded-SurrealDB authority for Atelier
/// integration tests. Keeping the temporary directory in this value guarantees
/// that the on-disk engine outlives every handle derived from it.
pub struct AtelierSurrealHarness {
    pub atelier: AtelierStore,
    pub database: Arc<dyn Database>,
    pub storage: SurrealStorage,
    /// `Some` while this harness owns a fresh temp dir (dropped with the harness); `None` when it
    /// re-opened a directory kept alive across a close/reopen proof, in which case the caller owns
    /// the path. tempfile 3.27 has no `TempDir::from_path`, so re-adoption is tracked by path.
    _directory: Option<tempfile::TempDir>,
    data_dir: PathBuf,
}

/// A test-owned store plus its backing harness. The wrapper keeps the
/// temporary RocksDB directory alive for the whole test while still allowing
/// existing Atelier tests to call the store methods directly through `Deref`.
pub struct ConnectedAtelier {
    pub store: AtelierStore,
    pub harness: AtelierSurrealHarness,
}

impl std::ops::Deref for ConnectedAtelier {
    type Target = AtelierStore;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl AtelierSurrealHarness {
    pub async fn create() -> Self {
        let directory = tempfile::tempdir().expect("create isolated Atelier SurrealDB root");
        let data_dir = directory.path().to_path_buf();
        Self::open_directory(Some(directory), data_dir).await
    }

    /// Re-open an existing on-disk store for a real close/reopen durability
    /// proof. The directory was kept alive by `close_for_reopen`; the caller
    /// owns it and cleans it up after the final harness is shut down.
    pub async fn open_existing(data_dir: PathBuf) -> Self {
        Self::open_directory(None, data_dir).await
    }

    async fn open_directory(directory: Option<tempfile::TempDir>, data_dir: PathBuf) -> Self {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::for_data_dir(&data_dir)
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
            data_dir,
        }
    }

    /// Close the embedded store while preserving its directory for a later
    /// `open_existing` call. The caller owns the returned path and should
    /// close one final harness to clean it up.
    pub async fn close_for_reopen(self) -> PathBuf {
        self.storage
            .shutdown()
            .await
            .expect("close isolated Atelier SurrealDB");
        match self._directory {
            Some(directory) => directory.keep(),
            None => self.data_dir,
        }
    }

    pub async fn shutdown(self) {
        self.storage
            .shutdown()
            .await
            .expect("close isolated Atelier SurrealDB");
    }

    /// Read-only catalog-validated row count for focused integration proofs.
    pub async fn row_count_by_field(&self, table: &str, field: &str, value: &str) -> u64 {
        let inspector = self.storage.test_inspector();
        let table_selector = inspector
            .table_selector(table)
            .await
            .expect("inspect embedded table");
        let field_selector = table_selector.field(field).expect("inspect embedded field");
        inspector
            .row_count(
                &table_selector,
                RowFilter::FieldEquals {
                    field: field_selector,
                    value: ScalarValue::String(value.to_owned()),
                },
            )
            .await
            .expect("count embedded rows")
    }
}

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

/// Owns a temporary directory for the lifetime of a harness and removes it when
/// dropped.
///
/// `tempfile::TempDir` cannot serve this role: it has no constructor that
/// adopts an already-existing directory, which `open_existing` needs in order to
/// re-take ownership of the path handed back by `close_for_reopen`. Its
/// `from_path` sibling belongs to `TempPath`, which owns a single file rather
/// than a directory tree.
struct OwnedTempRoot {
    path: Option<PathBuf>,
}

impl OwnedTempRoot {
    /// Take ownership of an existing directory.
    fn adopt(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn path(&self) -> &Path {
        self.path
            .as_deref()
            .expect("temporary root is still owned by this guard")
    }

    /// Give up ownership without removing the directory so that a later harness
    /// can adopt the same path.
    fn release(mut self) -> PathBuf {
        self.path
            .take()
            .expect("temporary root is still owned by this guard")
    }
}

impl Drop for OwnedTempRoot {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = fs::remove_dir_all(path);
        }
    }
}

/// One isolated, fully bootstrapped embedded-SurrealDB authority for Atelier
/// integration tests. Keeping the temporary directory in this value guarantees
/// that the on-disk engine outlives every handle derived from it.
pub struct AtelierSurrealHarness {
    pub atelier: AtelierStore,
    pub database: Arc<dyn Database>,
    pub storage: SurrealStorage,
    _directory: OwnedTempRoot,
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
        Self::open_directory(OwnedTempRoot::adopt(directory.keep())).await
    }

    /// Re-open an existing on-disk store for a real close/reopen durability
    /// proof. Ownership of the directory remains with the returned harness.
    pub async fn open_existing(data_dir: PathBuf) -> Self {
        Self::open_directory(OwnedTempRoot::adopt(data_dir)).await
    }

    async fn open_directory(directory: OwnedTempRoot) -> Self {
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

    /// Close the embedded store while preserving its directory for a later
    /// `open_existing` call. The caller owns the returned path and should
    /// close one final harness to clean it up.
    pub async fn close_for_reopen(self) -> PathBuf {
        self.storage
            .shutdown()
            .await
            .expect("close isolated Atelier SurrealDB");
        self._directory.release()
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

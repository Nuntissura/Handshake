use handshake_core::managed_postgres::{
    ManagedPostgres, ManagedPostgresConfig, ManagedPostgresError,
};
use handshake_core::storage::artifacts::{
    artifact_root_rel, resolve_workspace_root, validate_artifact_content_hash, write_file_artifact,
    ArtifactClassification, ArtifactLayer, ArtifactManifest, ArtifactPayloadKind,
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::sync::OnceCell;
use uuid::Uuid;

static MANAGED_POSTGRES: OnceCell<Option<ManagedPostgres>> = OnceCell::const_new();
static TEST_ARTIFACT_WORKSPACE: OnceLock<tempfile::TempDir> = OnceLock::new();

pub async fn database_url() -> Option<String> {
    if let Some(url) = std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Some(url);
    }

    let managed = MANAGED_POSTGRES
        .get_or_init(|| async {
            match ManagedPostgres::ensure_running(ManagedPostgresConfig::from_env()).await {
                Ok(managed) => Some(managed),
                Err(ManagedPostgresError::BinariesNotFound(detail)) => {
                    eprintln!(
                        "SKIP atelier PostgreSQL proof: PostgreSQL binaries not found ({detail})"
                    );
                    None
                }
                Err(err) => panic!("Handshake-managed PostgreSQL startup failed: {err}"),
            }
        })
        .await;

    managed.as_ref().map(ManagedPostgres::database_url)
}

pub struct NativeMediaArtifact {
    pub workspace_root: PathBuf,
    pub artifact_id: Uuid,
    pub artifact_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub stored_payload: Vec<u8>,
}

pub fn write_native_media_artifact(payload: &[u8]) -> NativeMediaArtifact {
    let mut stored_payload = payload.to_vec();
    stored_payload.extend_from_slice(Uuid::new_v4().as_bytes());
    write_native_media_artifact_from_stored_payload(&stored_payload)
}

pub fn test_artifact_workspace_root() -> PathBuf {
    if let Ok(value) = std::env::var("HANDSHAKE_WORKSPACE_ROOT") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    let workspace = TEST_ARTIFACT_WORKSPACE.get_or_init(|| {
        tempfile::tempdir().expect("create process-local test ArtifactStore workspace")
    });
    std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", workspace.path());
    resolve_workspace_root().expect("resolve process-local test ArtifactStore workspace")
}

pub fn write_native_media_artifact_from_stored_payload(
    stored_payload: &[u8],
) -> NativeMediaArtifact {
    let workspace_root = test_artifact_workspace_root();
    write_native_media_artifact_from_stored_payload_in_workspace(&workspace_root, stored_payload)
}

pub fn write_native_media_artifact_in_workspace(
    workspace_root: &Path,
    payload: &[u8],
) -> NativeMediaArtifact {
    let mut stored_payload = payload.to_vec();
    stored_payload.extend_from_slice(Uuid::new_v4().as_bytes());
    write_native_media_artifact_from_stored_payload_in_workspace(workspace_root, &stored_payload)
}

pub fn write_native_media_artifact_from_stored_payload_in_workspace(
    workspace_root: &Path,
    stored_payload: &[u8],
) -> NativeMediaArtifact {
    let artifact_id = Uuid::now_v7();
    let content_hash = sha256_hex(stored_payload);
    let manifest = ArtifactManifest {
        artifact_id,
        layer: ArtifactLayer::L1,
        kind: ArtifactPayloadKind::File,
        mime: "image/png".to_string(),
        filename_hint: Some("atelier-media.png".to_string()),
        created_at: chrono::Utc::now(),
        created_by_job_id: None,
        source_entity_refs: Vec::new(),
        source_artifact_refs: Vec::new(),
        content_hash: content_hash.clone(),
        size_bytes: stored_payload.len() as u64,
        classification: ArtifactClassification::Low,
        exportable: true,
        retention_ttl_days: None,
        pinned: Some(true),
        hash_basis: None,
        hash_exclude_paths: Vec::new(),
    };
    write_file_artifact(&workspace_root, &manifest, &stored_payload)
        .expect("write native ArtifactStore");
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, artifact_id)
        .expect("validate native ArtifactStore payload hash");
    let artifact_ref = format!(
        "artifact://{}/payload",
        artifact_root_rel(ArtifactLayer::L1, artifact_id)
    );
    NativeMediaArtifact {
        workspace_root: workspace_root.to_path_buf(),
        artifact_id,
        artifact_ref,
        content_hash,
        byte_len: stored_payload.len() as i64,
        stored_payload: stored_payload.to_vec(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

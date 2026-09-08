use std::path::{Path, PathBuf};

use crate::storage::artifacts::{self, ArtifactError};
use crate::storage::{
    Asset, Database, LoomArtifactBinding, LoomArtifactBindingState, NewAsset, StorageError,
    StorageResult, VerifiedLoomArtifact, WriteContext,
};
use artifacts::{ArtifactClassification, ArtifactLayer, ArtifactManifest, ArtifactPayloadKind};

// Serializes filesystem publication/recovery in this embedded process. Database
// compare-and-set checks remain authoritative for the stored reservation.
static LOOM_ARTIFACT_IO: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn artifact_error(error: impl std::fmt::Display) -> StorageError {
    StorageError::Migration(error.to_string())
}

fn classification(value: &str, ttl: Option<u32>) -> StorageResult<ArtifactClassification> {
    if ttl == Some(0) {
        return Err(StorageError::Validation(
            "Loom retention TTL must be positive",
        ));
    }
    match value {
        "low" => Ok(ArtifactClassification::Low),
        "medium" => Ok(ArtifactClassification::Medium),
        "high" if ttl.is_some() => Ok(ArtifactClassification::High),
        _ => Err(StorageError::Validation(
            "Loom classification requires an explicit retention policy",
        )),
    }
}

fn verify_bytes(hash: &str, length: i64, bytes: &[u8]) -> StorageResult<()> {
    let (hash, _) = artifacts::normalize_sha256_hex_ref(hash)
        .ok_or(StorageError::Validation("invalid Loom asset hash"))?;
    if length < 0 || length as u64 != bytes.len() as u64 || hash != artifacts::sha256_hex(bytes) {
        return Err(StorageError::Validation(
            "Loom asset byte integrity mismatch",
        ));
    }
    Ok(())
}

fn legacy_path(root: &Path, asset: &Asset) -> StorageResult<PathBuf> {
    if asset.workspace_id.is_empty()
        || asset.workspace_id.contains(['/', '\\', ':'])
        || matches!(asset.workspace_id.as_str(), "." | "..")
    {
        return Err(StorageError::Validation("invalid Loom workspace path"));
    }
    let (hash, _) = artifacts::normalize_sha256_hex_ref(&asset.content_hash)
        .ok_or(StorageError::Validation("invalid Loom asset hash"))?;
    let path = loom_asset_blob_path(root, &asset.workspace_id, &asset.kind, &hash);
    if path.exists() {
        let canonical = std::fs::canonicalize(&path).map_err(artifact_error)?;
        let workspace = std::fs::canonicalize(
            root.join("data")
                .join("workspaces")
                .join(&asset.workspace_id),
        )
        .map_err(artifact_error)?;
        let canonical_root = std::fs::canonicalize(root).map_err(artifact_error)?;
        if !workspace.starts_with(canonical_root) || !canonical.starts_with(workspace) {
            return Err(StorageError::Validation(
                "Loom legacy path escapes workspace",
            ));
        }
    }
    Ok(path)
}

fn verified_artifact(
    root: &Path,
    asset: &Asset,
    binding: &LoomArtifactBinding,
) -> StorageResult<Vec<u8>> {
    if binding.asset_id != asset.asset_id || binding.workspace_id != asset.workspace_id {
        return Err(StorageError::Validation("Loom binding identity mismatch"));
    }
    let manifest = artifacts::read_artifact_manifest(root, ArtifactLayer::L1, binding.artifact_id)
        .map_err(artifact_error)?;
    if manifest.artifact_id != binding.artifact_id
        || manifest.layer != ArtifactLayer::L1
        || manifest.kind != ArtifactPayloadKind::File
        || manifest.mime != asset.mime
        || manifest.filename_hint != asset.original_filename
        || manifest.size_bytes != asset.size_bytes as u64
        || !artifacts::sha256_refs_match(&manifest.content_hash, &asset.content_hash)
        || manifest.classification
            != classification(&asset.classification, binding.retention_ttl_days)?
        || manifest.exportable != asset.exportable
        || manifest.retention_ttl_days != binding.retention_ttl_days
    {
        return Err(StorageError::Validation(
            "Loom artifact manifest differs from asset",
        ));
    }
    let bytes =
        artifacts::read_file_artifact_with_manifest(root, &manifest).map_err(artifact_error)?;
    verify_bytes(&asset.content_hash, asset.size_bytes, &bytes)?;
    Ok(bytes)
}

fn materialize_reserved(
    root: &Path,
    asset: &Asset,
    binding: &LoomArtifactBinding,
    bytes: &[u8],
    ctx: &WriteContext,
) -> StorageResult<()> {
    verify_bytes(&asset.content_hash, asset.size_bytes, bytes)?;
    if binding.state == LoomArtifactBindingState::Ready {
        verified_artifact(root, asset, binding)?;
        return Ok(());
    }
    let class = classification(&asset.classification, binding.retention_ttl_days)?;
    let destination = artifacts::artifact_root_dir(root, ArtifactLayer::L1, binding.artifact_id);
    std::fs::create_dir_all(&destination).map_err(artifact_error)?;
    if destination
        .join(artifacts::ARTIFACT_MANIFEST_FILENAME)
        .exists()
    {
        verified_artifact(root, asset, binding)?;
        return Ok(());
    }
    let payload = destination.join("payload");
    if payload.exists() {
        // Crash after publication but before manifest: verify, then finish; never replace.
        verify_bytes(
            &asset.content_hash,
            asset.size_bytes,
            &std::fs::read(&payload).map_err(artifact_error)?,
        )?;
    } else {
        artifacts::write_file_atomic(&destination, &payload, bytes, false)
            .map_err(artifact_error)?;
    }
    let manifest = ArtifactManifest {
        artifact_id: binding.artifact_id,
        layer: ArtifactLayer::L1,
        kind: ArtifactPayloadKind::File,
        mime: asset.mime.clone(),
        filename_hint: asset.original_filename.clone(),
        created_at: asset.created_at,
        created_by_job_id: ctx.job_id,
        source_entity_refs: Vec::new(),
        source_artifact_refs: Vec::new(),
        content_hash: artifacts::sha256_hex(bytes),
        size_bytes: bytes.len() as u64,
        classification: class,
        exportable: asset.exportable,
        retention_ttl_days: binding.retention_ttl_days,
        pinned: None,
        hash_basis: None,
        hash_exclude_paths: Vec::new(),
    };
    artifacts::write_artifact_manifest_atomic(&destination, &manifest).map_err(artifact_error)?;
    verified_artifact(root, asset, binding)?;
    Ok(())
}

async fn complete_binding(
    storage: &dyn Database,
    ctx: &WriteContext,
    root: &Path,
    asset: &Asset,
    binding: LoomArtifactBinding,
    bytes: &[u8],
) -> StorageResult<Vec<u8>> {
    materialize_reserved(root, asset, &binding, bytes, ctx)?;
    let artifact_id = binding.artifact_id;
    storage
        .publish_loom_artifact_binding(
            ctx,
            VerifiedLoomArtifact {
                asset: asset.clone(),
                binding,
            },
        )
        .await?;
    let current_asset = storage
        .get_asset(&asset.workspace_id, &asset.asset_id)
        .await?;
    let current = storage
        .get_loom_artifact_binding(&asset.workspace_id, &asset.asset_id)
        .await?
        .ok_or(StorageError::Validation("Loom published binding missing"))?;
    if current.state != LoomArtifactBindingState::Ready || current.artifact_id != artifact_id {
        return Err(StorageError::Validation(
            "Loom binding not ready or changed",
        ));
    }
    let bytes = verified_artifact(root, &current_asset, &current)?;
    // Canonical binding and destination were reread. Only a verified legacy input
    // may be retired; damaged legacy bytes remain available for diagnosis.
    let legacy = legacy_path(root, &current_asset)?;
    if legacy.exists() {
        let old = std::fs::read(&legacy).map_err(artifact_error)?;
        verify_bytes(&current_asset.content_hash, current_asset.size_bytes, &old)?;
        std::fs::remove_file(&legacy).map_err(artifact_error)?;
    }
    Ok(bytes)
}

/// Validate input before creating a catalog identity, then publish its verified L1 binding.
pub async fn materialize_loom_asset(
    storage: &dyn Database,
    ctx: &WriteContext,
    root: &Path,
    new_asset: NewAsset,
    bytes: &[u8],
    retention_ttl_days: Option<u32>,
) -> StorageResult<Asset> {
    let _guard = LOOM_ARTIFACT_IO.lock().await;
    verify_bytes(&new_asset.content_hash, new_asset.size_bytes, bytes)?;
    classification(&new_asset.classification, retention_ttl_days)?;
    let asset = match storage
        .find_asset_by_content_hash(&new_asset.workspace_id, &new_asset.content_hash)
        .await?
    {
        Some(asset) => asset,
        None => match storage.create_asset(ctx, new_asset.clone()).await {
            Ok(asset) => asset,
            Err(error) => storage
                .find_asset_by_content_hash(&new_asset.workspace_id, &new_asset.content_hash)
                .await?
                .ok_or(error)?,
        },
    };
    verify_bytes(&asset.content_hash, asset.size_bytes, bytes)?;
    let binding = storage
        .reserve_loom_artifact_binding(ctx, &asset, retention_ttl_days)
        .await?;
    complete_binding(storage, ctx, root, &asset, binding, bytes).await?;
    storage
        .get_asset(&asset.workspace_id, &asset.asset_id)
        .await
}

/// Resume any interrupted migration boundary; legacy bytes are input only.
pub async fn migrate_loom_asset(
    storage: &dyn Database,
    ctx: &WriteContext,
    root: &Path,
    asset: &Asset,
    retention_ttl_days: Option<u32>,
) -> StorageResult<Vec<u8>> {
    let _guard = LOOM_ARTIFACT_IO.lock().await;
    let prior = storage
        .get_loom_artifact_binding(&asset.workspace_id, &asset.asset_id)
        .await?;
    if let Some(binding) = &prior {
        if retention_ttl_days.is_some() && retention_ttl_days != binding.retention_ttl_days {
            return Err(StorageError::Conflict("loom_artifact_retention_policy"));
        }
    }
    let ttl = prior
        .as_ref()
        .and_then(|b| b.retention_ttl_days)
        .or(retention_ttl_days);
    classification(&asset.classification, ttl)?;
    let binding = storage
        .reserve_loom_artifact_binding(ctx, asset, ttl)
        .await?;
    let destination = artifacts::artifact_root_dir(root, ArtifactLayer::L1, binding.artifact_id);
    let bytes = if binding.state == LoomArtifactBindingState::Ready
        || destination
            .join(artifacts::ARTIFACT_MANIFEST_FILENAME)
            .exists()
    {
        verified_artifact(root, asset, &binding)?
    } else if destination.join("payload").exists() {
        let bytes = std::fs::read(destination.join("payload")).map_err(artifact_error)?;
        verify_bytes(&asset.content_hash, asset.size_bytes, &bytes)?;
        bytes
    } else {
        let bytes = std::fs::read(legacy_path(root, asset)?).map_err(artifact_error)?;
        verify_bytes(&asset.content_hash, asset.size_bytes, &bytes)?;
        bytes
    };
    complete_binding(storage, ctx, root, asset, binding, &bytes).await
}

/// Read all verified bytes before the HTTP boundary selects a Range slice.
pub async fn read_loom_asset_bytes(
    storage: &dyn Database,
    ctx: &WriteContext,
    root: &Path,
    asset: &Asset,
) -> StorageResult<Vec<u8>> {
    if let Some(binding) = storage
        .get_loom_artifact_binding(&asset.workspace_id, &asset.asset_id)
        .await?
    {
        if binding.state == LoomArtifactBindingState::Ready {
            // Resume a crash between the Ready publication and legacy retirement.
            if legacy_path(root, asset)?.exists() {
                return migrate_loom_asset(storage, ctx, root, asset, None).await;
            }
            return verified_artifact(root, asset, &binding);
        }
    }
    migrate_loom_asset(storage, ctx, root, asset, None).await
}

pub const LOOM_ASSET_DIR: &str = "assets";
pub const LOOM_ASSET_ORIGINAL_DIR: &str = "original";
pub const LOOM_ASSET_PREVIEW_DIR: &str = "preview";
pub const LOOM_ASSET_PROXY_DIR: &str = "proxy";

pub fn resolve_handshake_root() -> Result<PathBuf, ArtifactError> {
    artifacts::resolve_workspace_root()
}

pub fn loom_asset_blob_path(
    handshake_root: &Path,
    workspace_id: &str,
    asset_kind: &str,
    content_hash: &str,
) -> PathBuf {
    let tier_dir = match asset_kind {
        "original" => LOOM_ASSET_ORIGINAL_DIR,
        "thumbnail" => LOOM_ASSET_PREVIEW_DIR,
        "proxy" => LOOM_ASSET_PROXY_DIR,
        _ => "blobs",
    };

    handshake_root
        .join("data")
        .join("workspaces")
        .join(workspace_id)
        .join(LOOM_ASSET_DIR)
        .join(tier_dir)
        .join(content_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loom_asset_blob_path_uses_portable_workspace_layout_for_originals() {
        let root = Path::new("C:/handshake-root");

        let path = loom_asset_blob_path(root, "ws-123", "original", "abc123");

        assert_eq!(
            path,
            root.join("data")
                .join("workspaces")
                .join("ws-123")
                .join("assets")
                .join("original")
                .join("abc123")
        );
    }

    #[test]
    fn loom_asset_blob_path_routes_preview_proxy_and_fallback_kinds() {
        let root = Path::new("C:/handshake-root");

        assert_eq!(
            loom_asset_blob_path(root, "ws-123", "thumbnail", "thumb"),
            root.join("data")
                .join("workspaces")
                .join("ws-123")
                .join("assets")
                .join("preview")
                .join("thumb")
        );
        assert_eq!(
            loom_asset_blob_path(root, "ws-123", "proxy", "proxy-hash"),
            root.join("data")
                .join("workspaces")
                .join("ws-123")
                .join("assets")
                .join("proxy")
                .join("proxy-hash")
        );
        assert_eq!(
            loom_asset_blob_path(root, "ws-123", "unknown", "blob-hash"),
            root.join("data")
                .join("workspaces")
                .join("ws-123")
                .join("assets")
                .join("blobs")
                .join("blob-hash")
        );
    }
}

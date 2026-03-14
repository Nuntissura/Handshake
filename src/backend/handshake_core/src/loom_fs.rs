use std::path::{Path, PathBuf};

use crate::storage::artifacts::{self, ArtifactError};

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

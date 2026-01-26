use std::path::{Path, PathBuf};

/// Shadow Workspace Root (Phase 1, normative): `data/workspaces/{workspace_id}/workspace/`
/// (Relative to the Handshake root directory.)
///
/// Layer directories are relative to that root:
/// - Bronze: `raw/`
/// - Silver: `derived/`
/// - Gold: `indexes/`, `graph/`
#[derive(Debug, Clone)]
pub struct ShadowWorkspacePaths {
    handshake_root: PathBuf,
    workspace_id: String,
}

impl ShadowWorkspacePaths {
    pub fn new(handshake_root: PathBuf, workspace_id: impl Into<String>) -> Self {
        Self {
            handshake_root,
            workspace_id: workspace_id.into(),
        }
    }

    pub fn handshake_root(&self) -> &Path {
        &self.handshake_root
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn shadow_root_dir(&self) -> PathBuf {
        self.handshake_root
            .join("data")
            .join("workspaces")
            .join(&self.workspace_id)
            .join("workspace")
    }

    pub fn bronze_dir(&self) -> PathBuf {
        self.shadow_root_dir().join("raw")
    }

    pub fn silver_dir(&self) -> PathBuf {
        self.shadow_root_dir().join("derived")
    }

    pub fn gold_indexes_dir(&self) -> PathBuf {
        self.shadow_root_dir().join("indexes")
    }

    pub fn gold_graph_dir(&self) -> PathBuf {
        self.shadow_root_dir().join("graph")
    }

    pub fn bronze_artifact_path(&self, bronze_id: &str) -> PathBuf {
        self.bronze_dir().join(format!("{bronze_id}.bin"))
    }

    pub fn silver_chunk_artifact_path(&self, silver_id: &str) -> PathBuf {
        self.silver_dir().join(format!("{silver_id}.txt"))
    }

    pub fn silver_embedding_artifact_path(
        &self,
        silver_id: &str,
        model_id: &str,
        model_version: &str,
    ) -> PathBuf {
        self.silver_dir()
            .join(format!("{silver_id}__{model_id}__{model_version}.json"))
    }

    pub fn keyword_index_path(&self) -> PathBuf {
        self.gold_indexes_dir().join("keyword_index_bm25_v1.json")
    }

    pub fn vector_index_path(&self, model_id: &str, model_version: &str) -> PathBuf {
        self.gold_indexes_dir().join(format!(
            "vector_index__{model_id}__{model_version}__v1.json"
        ))
    }

    pub fn graph_index_path(&self) -> PathBuf {
        self.gold_graph_dir().join("graph_v1.json")
    }

    pub fn to_root_relative(&self, path: &Path) -> String {
        let relative = path.strip_prefix(&self.handshake_root).unwrap_or(path);
        relative
            .to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches("./")
            .to_string()
    }
}

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

use crate::ai_ready_data::records::{
    BronzeRecord, EmbeddingModelRecord, EmbeddingRegistry, NewBronzeRecord, NewSilverRecord,
    SilverRecord,
};

pub(crate) mod locus_sqlite;
pub mod postgres;
pub mod retention;
pub mod sqlite;

// Test utilities - exposed for integration tests.
// The helper function `run_storage_conformance` uses Result-based error handling.
pub mod tests;

pub type StorageResult<T> = Result<T, StorageError>;

/// Unified storage error type so callers don't leak provider-specific details.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("conflict: {0}")]
    Conflict(&'static str),
    #[error("validation failed: {0}")]
    Validation(&'static str),
    #[error("mutation guard blocked: {0}")]
    Guard(&'static str),
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Opaque database error - hides provider-specific types [§2.3.12.3 Trait Purity]
    #[error("database error: {0}")]
    Database(String),
    /// Opaque migration error - hides provider-specific types [§2.3.12.3 Trait Purity]
    #[error("migration error: {0}")]
    Migration(String),
}

// [§2.3.12.3] Manual From impl to convert sqlx::Error -> StorageError::Database
// This preserves the error message while hiding the sqlx type from public API.
impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        StorageError::Database(err.to_string())
    }
}

// [§2.3.12.3] Manual From impl to convert MigrateError -> StorageError::Migration
// This preserves the error message while hiding the sqlx::migrate type from public API.
impl From<sqlx::migrate::MigrateError> for StorageError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        StorageError::Migration(err.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        StorageError::Serialization(value.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewWorkspace {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewDocument {
    pub workspace_id: String,
    pub title: String,
}

/// Document block with content and classification metadata.
///
/// Classification fields support ACE runtime validators [HSK-ACE-VAL-100]:
/// - `sensitivity`: Content sensitivity level ("low"|"medium"|"high"|"unknown")
/// - `exportable`: Whether content can be sent to cloud models
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub document_id: String,
    pub kind: String,
    pub sequence: i64,
    pub raw_content: String,
    pub display_content: String,
    pub derived_content: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Content sensitivity level [HSK-ACE-VAL-100]
    /// Values: "low", "medium", "high", "unknown" (NULL treated as "unknown")
    pub sensitivity: Option<String>,
    /// Whether content can be exported to cloud models [HSK-ACE-VAL-100]
    /// NULL or true = exportable, false = local-only
    pub exportable: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct NewBlock {
    pub id: Option<String>,
    pub document_id: String,
    pub kind: String,
    pub sequence: i64,
    pub raw_content: String,
    pub display_content: Option<String>,
    pub derived_content: Option<Value>,
    /// Content sensitivity level [HSK-ACE-VAL-100]
    pub sensitivity: Option<String>,
    /// Whether content can be exported to cloud models [HSK-ACE-VAL-100]
    pub exportable: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct BlockUpdate {
    pub kind: Option<String>,
    pub sequence: Option<i64>,
    pub raw_content: Option<String>,
    pub display_content: Option<String>,
    pub derived_content: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Canvas {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasNode {
    pub id: String,
    pub canvas_id: String,
    pub kind: String,
    pub position_x: f64,
    pub position_y: f64,
    pub data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasEdge {
    pub id: String,
    pub canvas_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewCanvas {
    pub workspace_id: String,
    pub title: String,
}

#[derive(Clone, Debug)]
pub struct NewCanvasNode {
    pub id: Option<String>,
    pub kind: String,
    pub position_x: f64,
    pub position_y: f64,
    pub data: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct NewCanvasEdge {
    pub id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasGraph {
    pub canvas: Canvas,
    pub nodes: Vec<CanvasNode>,
    pub edges: Vec<CanvasEdge>,
}

/// [HSK-GC-001] Artifact classification for retention policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// Flight Recorder traces (.jsonl)
    Log,
    /// AI Job outputs / EngineResults
    Result,
    /// Context snapshots (ACE-RAG)
    Evidence,
    /// Web/Model cache
    Cache,
    /// Durable workflow snapshots
    Checkpoint,
}

impl std::fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactKind::Log => write!(f, "log"),
            ArtifactKind::Result => write!(f, "result"),
            ArtifactKind::Evidence => write!(f, "evidence"),
            ArtifactKind::Cache => write!(f, "cache"),
            ArtifactKind::Checkpoint => write!(f, "checkpoint"),
        }
    }
}

/// [HSK-GC-001] Report produced after a prune operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneReport {
    pub timestamp: DateTime<Utc>,
    pub items_scanned: u32,
    pub items_pruned: u32,
    pub items_spared_pinned: u32,
    pub items_spared_window: u32,
    pub total_bytes_freed: u64,
}

impl PruneReport {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            items_scanned: 0,
            items_pruned: 0,
            items_spared_pinned: 0,
            items_spared_window: 0,
            total_bytes_freed: 0,
        }
    }
}

impl Default for PruneReport {
    fn default() -> Self {
        Self::new()
    }
}

/// [HSK-GC-001] Retention policy configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub kind: ArtifactKind,
    /// Number of days to retain items. Default: 30 for Logs, 7 for Cache.
    pub window_days: u32,
    /// Minimum versions to keep even if expired. Default: 3.
    pub min_versions: u32,
}

impl RetentionPolicy {
    /// Default policy for logs: 30 days, keep min 3 versions.
    pub fn default_log() -> Self {
        Self {
            kind: ArtifactKind::Log,
            window_days: 30,
            min_versions: 3,
        }
    }

    /// Default policy for AI job results: 30 days, keep min 3 versions.
    pub fn default_result() -> Self {
        Self {
            kind: ArtifactKind::Result,
            window_days: 30,
            min_versions: 3,
        }
    }

    /// Default policy for cache: 7 days, keep min 3 versions.
    pub fn default_cache() -> Self {
        Self {
            kind: ArtifactKind::Cache,
            window_days: 7,
            min_versions: 3,
        }
    }
}

/// Artifact system foundations (Phase 1): on-disk artifact store + manifests + hashing + LocalFile
/// materialize helper.
///
/// Spec anchors:
/// - 2.3.10.6 Artifact manifests + on-disk layout (normative)
/// - 2.3.10.7 Bundles + canonical hashing (normative)
/// - 2.3.11.2 Materialize rules (atomic + traversal-safe + no bypass)
pub mod artifacts {
    use std::fs;
    use std::io::{self, Write};
    use std::path::{Component, Path, PathBuf};

    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use sha2::{Digest, Sha256};
    use thiserror::Error;
    use uuid::Uuid;

    use crate::ace::ArtifactHandle;

    use super::EntityRef;

    const HANDSHAKE_DIR: &str = ".handshake";
    const ARTIFACTS_DIR: &str = "artifacts";
    const ARTIFACT_MANIFEST_FILENAME: &str = "artifact.json";

    #[derive(Debug, Error)]
    pub enum ArtifactError {
        #[error("workspace root resolve failed: {0}")]
        WorkspaceRootResolve(String),
        #[error("invalid relative path: {path}")]
        InvalidRelPath { path: String },
        #[error("path traversal blocked: {path}")]
        PathTraversalBlocked { path: String },
        #[error("write blocked: target escapes root: {path}")]
        RootEscape { path: String },
        #[error("missing retention_ttl_days for high-sensitivity artifact: artifact_id={artifact_id} kind={kind:?}")]
        MissingRetentionTtlDays {
            artifact_id: Uuid,
            kind: ArtifactPayloadKind,
        },
        #[error("invalid sha256 hex: {field}")]
        InvalidSha256Hex { field: String },
        #[error("content hash mismatch")]
        ContentHashMismatch,
        #[error("io error: {0}")]
        Io(#[from] io::Error),
        #[error("serialization error: {0}")]
        Serialization(#[from] serde_json::Error),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum ArtifactLayer {
        #[serde(rename = "L1")]
        L1,
        #[serde(rename = "L2")]
        L2,
        #[serde(rename = "L3")]
        L3,
        #[serde(rename = "L4")]
        L4,
    }

    impl ArtifactLayer {
        pub fn as_str(&self) -> &'static str {
            match self {
                ArtifactLayer::L1 => "L1",
                ArtifactLayer::L2 => "L2",
                ArtifactLayer::L3 => "L3",
                ArtifactLayer::L4 => "L4",
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ArtifactPayloadKind {
        File,
        ToolOutput,
        Transcript,
        DatasetSlice,
        PromptPayload,
        Report,
        Bundle,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ArtifactClassification {
        Low,
        Medium,
        High,
    }

    /// Spec 2.3.10.6 minimum schema (+ optional hash_basis fields for deterministic validation).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ArtifactManifest {
        pub artifact_id: Uuid,
        pub layer: ArtifactLayer,
        pub kind: ArtifactPayloadKind,
        pub mime: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub filename_hint: Option<String>,
        pub created_at: DateTime<Utc>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub created_by_job_id: Option<Uuid>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub source_entity_refs: Vec<EntityRef>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub source_artifact_refs: Vec<ArtifactHandle>,
        pub content_hash: String,
        pub size_bytes: u64,
        pub classification: ArtifactClassification,
        pub exportable: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub retention_ttl_days: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub pinned: Option<bool>,
        /// Optional but recommended for directory artifacts to enable deterministic content_hash
        /// validation (hash basis and explicit excludes).
        #[serde(skip_serializing_if = "Option::is_none")]
        pub hash_basis: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub hash_exclude_paths: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct BundleIndexEntry {
        pub path: String,
        pub content_hash: String,
        pub size_bytes: u64,
    }

    #[derive(Debug, Clone)]
    pub struct ArtifactWriteEntry {
        pub rel_path: String,
        pub bytes: Vec<u8>,
    }

    pub fn resolve_workspace_root() -> Result<PathBuf, ArtifactError> {
        if let Ok(value) = std::env::var("HANDSHAKE_WORKSPACE_ROOT") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }
        crate::capability_registry_workflow::repo_root_from_manifest_dir()
            .map_err(|e| ArtifactError::WorkspaceRootResolve(e.to_string()))
    }

    pub fn artifact_store_root(workspace_root: &Path) -> PathBuf {
        workspace_root.join(HANDSHAKE_DIR).join(ARTIFACTS_DIR)
    }

    pub fn artifact_root_dir(
        workspace_root: &Path,
        layer: ArtifactLayer,
        artifact_id: Uuid,
    ) -> PathBuf {
        artifact_store_root(workspace_root)
            .join(layer.as_str())
            .join(artifact_id.to_string())
    }

    pub fn artifact_root_rel(layer: ArtifactLayer, artifact_id: Uuid) -> String {
        format!(
            "{}/{}/{}/{}",
            HANDSHAKE_DIR,
            ARTIFACTS_DIR,
            layer.as_str(),
            artifact_id
        )
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        hex::encode(h.finalize())
    }

    fn normalize_rel_path(input: &str) -> String {
        input
            .replace('\\', "/")
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string()
    }

    fn ensure_safe_rel_path(rel_path: &str) -> Result<(), ArtifactError> {
        let path = Path::new(rel_path);
        if path.is_absolute() {
            return Err(ArtifactError::PathTraversalBlocked {
                path: rel_path.to_string(),
            });
        }
        for component in path.components() {
            match component {
                Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                    return Err(ArtifactError::PathTraversalBlocked {
                        path: rel_path.to_string(),
                    })
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn ensure_root_escape_blocked(root: &Path, target_path: &Path) -> Result<(), ArtifactError> {
        let root = fs::canonicalize(root)?;
        let parent = target_path
            .parent()
            .ok_or_else(|| ArtifactError::InvalidRelPath {
                path: target_path.to_string_lossy().to_string(),
            })?;
        fs::create_dir_all(parent)?;
        let parent_canon = fs::canonicalize(parent)?;
        if !parent_canon.starts_with(&root) {
            return Err(ArtifactError::RootEscape {
                path: target_path.to_string_lossy().to_string(),
            });
        }
        Ok(())
    }

    /// Atomic file write (temp + fsync + rename) with best-effort parent dir fsync.
    pub fn write_file_atomic(
        root: &Path,
        target_path: &Path,
        bytes: &[u8],
        overwrite: bool,
    ) -> Result<(), ArtifactError> {
        ensure_root_escape_blocked(root, target_path)?;

        let parent = target_path
            .parent()
            .ok_or_else(|| ArtifactError::InvalidRelPath {
                path: target_path.to_string_lossy().to_string(),
            })?;
        let parent_canon = fs::canonicalize(parent)?;

        let tmp_name = format!(".hsk_tmp_{}", Uuid::new_v4());
        let tmp_path = parent_canon.join(tmp_name);
        let mut tmp_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        tmp_file.write_all(bytes)?;
        tmp_file.sync_all()?;
        drop(tmp_file);

        if !overwrite && target_path.exists() {
            let _ = fs::remove_file(&tmp_path);
            return Err(ArtifactError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "target already exists (overwrite=false)",
            )));
        }
        if overwrite && target_path.exists() {
            if target_path.is_dir() {
                let _ = fs::remove_file(&tmp_path);
                return Err(ArtifactError::Io(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "target path exists and is a directory",
                )));
            }
            fs::remove_file(target_path)?;
        }

        if let Err(err) = fs::rename(&tmp_path, target_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(ArtifactError::Io(err));
        }

        if let Ok(dir_handle) = fs::File::open(&parent_canon) {
            let _ = dir_handle.sync_all();
        }
        Ok(())
    }

    /// Deterministic, canonical BundleIndex (sorted paths + per-item content_hash + size_bytes).
    pub fn canonical_bundle_index(mut entries: Vec<BundleIndexEntry>) -> Vec<BundleIndexEntry> {
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries
    }

    pub fn bundle_index_json(entries: &[BundleIndexEntry]) -> Result<Vec<u8>, ArtifactError> {
        let mut normalized: Vec<BundleIndexEntry> = Vec::with_capacity(entries.len());
        for entry in entries {
            if entry.path.trim().is_empty() {
                return Err(ArtifactError::InvalidRelPath {
                    path: entry.path.clone(),
                });
            }
            let rel = normalize_rel_path(&entry.path);
            ensure_safe_rel_path(&rel)?;
            if rel.contains(':') {
                return Err(ArtifactError::PathTraversalBlocked { path: rel });
            }
            normalized.push(BundleIndexEntry {
                path: rel,
                content_hash: entry.content_hash.clone(),
                size_bytes: entry.size_bytes,
            });
        }
        let normalized = canonical_bundle_index(normalized);
        Ok(serde_json::to_vec(&normalized)?)
    }

    pub fn bundle_index_content_hash(bundle_index_json: &[u8]) -> String {
        sha256_hex(bundle_index_json)
    }

    pub fn compute_entries_index(
        entries: &[ArtifactWriteEntry],
        exclude_paths: &[String],
    ) -> Result<(Vec<BundleIndexEntry>, u64), ArtifactError> {
        let mut index: Vec<BundleIndexEntry> = Vec::new();
        let mut total_size: u64 = 0;

        for entry in entries {
            let rel = normalize_rel_path(&entry.rel_path);
            if rel.trim().is_empty() {
                return Err(ArtifactError::InvalidRelPath { path: rel });
            }
            ensure_safe_rel_path(&rel)?;
            if rel.contains(':') {
                return Err(ArtifactError::PathTraversalBlocked { path: rel });
            }

            total_size = total_size.saturating_add(entry.bytes.len() as u64);
            if exclude_paths.iter().any(|p| normalize_rel_path(p) == rel) {
                continue;
            }
            let content_hash = sha256_hex(&entry.bytes);
            index.push(BundleIndexEntry {
                path: rel,
                content_hash,
                size_bytes: entry.bytes.len() as u64,
            });
        }

        Ok((canonical_bundle_index(index), total_size))
    }

    /// LocalFile materialize: writes a set of files into `export_root` (directory), enforcing
    /// traversal-safe relative paths and atomic per-file writes.
    pub fn materialize_local_dir(
        export_root: &Path,
        entries: &[ArtifactWriteEntry],
        overwrite: bool,
    ) -> Result<Vec<String>, ArtifactError> {
        if !export_root.is_absolute() {
            return Err(ArtifactError::InvalidRelPath {
                path: export_root.to_string_lossy().to_string(),
            });
        }
        fs::create_dir_all(export_root)?;
        if !export_root.is_dir() {
            return Err(ArtifactError::Io(io::Error::new(
                io::ErrorKind::NotADirectory,
                "export_root is not a directory",
            )));
        }
        let export_root_canon = fs::canonicalize(export_root)?;

        let mut materialized_paths: Vec<String> = Vec::with_capacity(entries.len());
        for entry in entries {
            let rel = normalize_rel_path(&entry.rel_path);
            ensure_safe_rel_path(&rel)?;
            if rel.contains(':') {
                return Err(ArtifactError::PathTraversalBlocked { path: rel });
            }

            let target_path = export_root_canon.join(Path::new(&rel));
            write_file_atomic(&export_root_canon, &target_path, &entry.bytes, overwrite)?;
            materialized_paths.push(rel);
        }

        materialized_paths.sort();
        Ok(materialized_paths)
    }

    pub fn write_artifact_manifest_atomic(
        artifact_root: &Path,
        manifest: &ArtifactManifest,
    ) -> Result<(), ArtifactError> {
        let manifest_path = artifact_root.join(ARTIFACT_MANIFEST_FILENAME);
        let bytes = serde_json::to_vec_pretty(manifest)?;
        write_file_atomic(artifact_root, &manifest_path, &bytes, false)
    }

    pub fn write_file_artifact(
        workspace_root: &Path,
        manifest: &ArtifactManifest,
        payload_bytes: &[u8],
    ) -> Result<(), ArtifactError> {
        if matches!(manifest.kind, ArtifactPayloadKind::PromptPayload)
            || matches!(manifest.classification, ArtifactClassification::High)
        {
            if manifest.retention_ttl_days.is_none() {
                return Err(ArtifactError::MissingRetentionTtlDays {
                    artifact_id: manifest.artifact_id,
                    kind: manifest.kind,
                });
            }
        }

        let artifact_root = artifact_root_dir(workspace_root, manifest.layer, manifest.artifact_id);
        fs::create_dir_all(&artifact_root)?;

        // Validate hash fields match payload.
        if manifest.content_hash != sha256_hex(payload_bytes) {
            return Err(ArtifactError::ContentHashMismatch);
        }
        if manifest.size_bytes != payload_bytes.len() as u64 {
            return Err(ArtifactError::ContentHashMismatch);
        }

        let payload_path = artifact_root.join("payload");
        write_file_atomic(&artifact_root, &payload_path, payload_bytes, false)?;
        write_artifact_manifest_atomic(&artifact_root, manifest)?;
        Ok(())
    }

    pub fn write_dir_artifact(
        workspace_root: &Path,
        manifest: &ArtifactManifest,
        entries: &[ArtifactWriteEntry],
    ) -> Result<(), ArtifactError> {
        if matches!(manifest.kind, ArtifactPayloadKind::PromptPayload)
            || matches!(manifest.classification, ArtifactClassification::High)
        {
            if manifest.retention_ttl_days.is_none() {
                return Err(ArtifactError::MissingRetentionTtlDays {
                    artifact_id: manifest.artifact_id,
                    kind: manifest.kind,
                });
            }
        }

        let artifact_root = artifact_root_dir(workspace_root, manifest.layer, manifest.artifact_id);
        let payload_root = artifact_root.join("payload");
        fs::create_dir_all(&payload_root)?;

        // Validate hash fields match entries (structural hashing).
        let (index, total_size) = compute_entries_index(entries, &manifest.hash_exclude_paths)?;
        let index_json = bundle_index_json(&index)?;
        let computed = bundle_index_content_hash(&index_json);
        if manifest.content_hash != computed || manifest.size_bytes != total_size {
            return Err(ArtifactError::ContentHashMismatch);
        }

        // Deterministic write order: lexicographic by normalized rel_path.
        let mut write_entries: Vec<ArtifactWriteEntry> = entries.to_vec();
        write_entries
            .sort_by(|a, b| normalize_rel_path(&a.rel_path).cmp(&normalize_rel_path(&b.rel_path)));

        for entry in &write_entries {
            let rel = normalize_rel_path(&entry.rel_path);
            ensure_safe_rel_path(&rel)?;
            let target_path = payload_root.join(Path::new(&rel));
            write_file_atomic(&payload_root, &target_path, &entry.bytes, false)?;
        }

        write_artifact_manifest_atomic(&artifact_root, manifest)?;
        Ok(())
    }

    pub fn read_artifact_manifest(
        workspace_root: &Path,
        layer: ArtifactLayer,
        artifact_id: Uuid,
    ) -> Result<ArtifactManifest, ArtifactError> {
        let artifact_root = artifact_root_dir(workspace_root, layer, artifact_id);
        let manifest_path = artifact_root.join(ARTIFACT_MANIFEST_FILENAME);
        let raw = fs::read_to_string(&manifest_path)?;
        let manifest: ArtifactManifest = serde_json::from_str(&raw)?;
        Ok(manifest)
    }

    pub fn validate_artifact_content_hash(
        workspace_root: &Path,
        layer: ArtifactLayer,
        artifact_id: Uuid,
    ) -> Result<(), ArtifactError> {
        let manifest = read_artifact_manifest(workspace_root, layer, artifact_id)?;
        let artifact_root = artifact_root_dir(workspace_root, layer, artifact_id);

        let payload_path = artifact_root.join("payload");
        let meta = fs::metadata(&payload_path)?;
        if meta.is_file() {
            let bytes = fs::read(payload_path)?;
            if sha256_hex(&bytes) != manifest.content_hash
                || bytes.len() as u64 != manifest.size_bytes
            {
                return Err(ArtifactError::ContentHashMismatch);
            }
            return Ok(());
        }

        if !meta.is_dir() {
            return Err(ArtifactError::InvalidRelPath {
                path: payload_path.to_string_lossy().to_string(),
            });
        }

        let payload_root = payload_path;
        let mut entries: Vec<ArtifactWriteEntry> = Vec::new();
        collect_dir_entries(&payload_root, &payload_root, &mut entries)?;
        let (index, total_size) = compute_entries_index(&entries, &manifest.hash_exclude_paths)?;
        let index_json = bundle_index_json(&index)?;
        let computed = bundle_index_content_hash(&index_json);
        if computed != manifest.content_hash || total_size != manifest.size_bytes {
            return Err(ArtifactError::ContentHashMismatch);
        }
        Ok(())
    }

    fn collect_dir_entries(
        root: &Path,
        dir: &Path,
        out: &mut Vec<ArtifactWriteEntry>,
    ) -> Result<(), ArtifactError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_symlink() {
                return Err(ArtifactError::InvalidRelPath {
                    path: entry.path().to_string_lossy().to_string(),
                });
            }
            let path = entry.path();
            if ty.is_dir() {
                collect_dir_entries(root, &path, out)?;
                continue;
            }
            if !ty.is_file() {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .map_err(|_| ArtifactError::InvalidRelPath {
                    path: path.to_string_lossy().to_string(),
                })?;
            let rel = normalize_rel_path(&rel.to_string_lossy());
            let bytes = fs::read(&path)?;
            out.push(ArtifactWriteEntry {
                rel_path: rel,
                bytes,
            });
        }
        Ok(())
    }

    pub fn normalize_materialized_path(path: &str) -> Result<String, ArtifactError> {
        let rel = normalize_rel_path(path);
        if rel.is_empty() || rel.contains(':') || rel.starts_with('/') {
            return Err(ArtifactError::InvalidRelPath { path: rel });
        }
        if rel.split('/').any(|c| c == "..") {
            return Err(ArtifactError::PathTraversalBlocked { path: rel });
        }
        Ok(rel)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityRef {
    pub entity_id: String,
    pub entity_kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    Read,
    Write,
    Plan,
    Execute,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannedOperation {
    pub op_type: OperationType,
    pub target: EntityRef,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Stalled,
    AwaitingValidation,
    AwaitingUser,
    Completed,
    CompletedWithIssues,
    Failed,
    Cancelled,
    Poisoned,
}

impl JobState {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobState::Queued => "queued",
            JobState::Running => "running",
            JobState::Stalled => "stalled",
            JobState::AwaitingValidation => "awaiting_validation",
            JobState::AwaitingUser => "awaiting_user",
            JobState::Completed => "completed",
            JobState::CompletedWithIssues => "completed_with_issues",
            JobState::Failed => "failed",
            JobState::Cancelled => "cancelled",
            JobState::Poisoned => "poisoned",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    DocEdit,
    DocRewrite,
    SheetTransform,
    CanvasCluster,
    AsrTranscribe,
    WorkflowRun,
    MicroTaskExecution,
    SpecRouter,
    LocusOperation,
    /// Backward-compatible terminal execution job kind.
    TerminalExec,
    /// Document summarization job kind.
    DocSummarize,
    /// Debug bundle export job [§10.5.6.8]
    DebugBundleExport,
    DocIngest,
    DistillationEval,
}

impl JobKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobKind::DocEdit => "doc_edit",
            JobKind::DocRewrite => "doc_rewrite",
            JobKind::SheetTransform => "sheet_transform",
            JobKind::CanvasCluster => "canvas_cluster",
            JobKind::AsrTranscribe => "asr_transcribe",
            JobKind::WorkflowRun => "workflow_run",
            JobKind::MicroTaskExecution => "micro_task_execution",
            JobKind::SpecRouter => "spec_router",
            JobKind::LocusOperation => "locus_operation",
            JobKind::TerminalExec => "terminal_exec",
            JobKind::DocSummarize => "doc_summarize",
            JobKind::DebugBundleExport => "debug_bundle_export",
            JobKind::DocIngest => "doc_ingest",
            JobKind::DistillationEval => "distillation_eval",
        }
    }
}

impl FromStr for JobKind {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "doc_edit" => Ok(JobKind::DocEdit),
            "doc_rewrite" => Ok(JobKind::DocRewrite),
            "sheet_transform" => Ok(JobKind::SheetTransform),
            "canvas_cluster" => Ok(JobKind::CanvasCluster),
            "asr_transcribe" => Ok(JobKind::AsrTranscribe),
            "workflow_run" => Ok(JobKind::WorkflowRun),
            "micro_task_execution" => Ok(JobKind::MicroTaskExecution),
            "spec_router" => Ok(JobKind::SpecRouter),
            "locus_operation" => Ok(JobKind::LocusOperation),
            "term_exec" | "terminal_exec" => Ok(JobKind::TerminalExec),
            "doc_summarize" => Ok(JobKind::DocSummarize),
            "debug_bundle_export" => Ok(JobKind::DebugBundleExport),
            "doc_ingest" => Ok(JobKind::DocIngest),
            "distillation_eval" => Ok(JobKind::DistillationEval),
            _ => Err(StorageError::Validation("invalid job kind")),
        }
    }
}

pub fn validate_job_contract(
    job_kind: &JobKind,
    profile_id: &str,
    protocol_id: &str,
) -> StorageResult<()> {
    const MICRO_TASK_EXECUTOR_V1_ID: &str = "micro_task_executor_v1";

    let is_mte_profile = profile_id == MICRO_TASK_EXECUTOR_V1_ID;
    let is_mte_protocol = protocol_id == MICRO_TASK_EXECUTOR_V1_ID;
    let is_mte_kind = matches!(job_kind, JobKind::MicroTaskExecution);

    if is_mte_kind && (!is_mte_profile || !is_mte_protocol) {
        return Err(StorageError::Validation(
            "invalid job contract: micro_task_execution requires micro_task_executor_v1 profile_id and protocol_id",
        ));
    }

    if (is_mte_profile || is_mte_protocol) && (!is_mte_profile || !is_mte_protocol) {
        return Err(StorageError::Validation(
            "invalid job contract: micro_task_executor_v1 requires profile_id and protocol_id to match",
        ));
    }

    if (is_mte_profile || is_mte_protocol) && !is_mte_kind {
        return Err(StorageError::Validation(
            "invalid job contract: micro_task_executor_v1 requires job_kind micro_task_execution",
        ));
    }

    Ok(())
}

impl TryFrom<&str> for JobState {
    type Error = StorageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "queued" => Ok(JobState::Queued),
            "running" => Ok(JobState::Running),
            "stalled" => Ok(JobState::Stalled),
            "awaiting_validation" => Ok(JobState::AwaitingValidation),
            "awaiting_user" => Ok(JobState::AwaitingUser),
            "completed" => Ok(JobState::Completed),
            "completed_with_issues" => Ok(JobState::CompletedWithIssues),
            "failed" => Ok(JobState::Failed),
            "cancelled" => Ok(JobState::Cancelled),
            "poisoned" => Ok(JobState::Poisoned),
            _ => Err(StorageError::Validation("invalid job state")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    AnalysisOnly,
    PreviewOnly,
    ApplyScoped,
}

impl AccessMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessMode::AnalysisOnly => "analysis_only",
            AccessMode::PreviewOnly => "preview_only",
            AccessMode::ApplyScoped => "apply_scoped",
        }
    }
}

impl TryFrom<&str> for AccessMode {
    type Error = StorageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "analysis_only" => Ok(AccessMode::AnalysisOnly),
            "preview_only" => Ok(AccessMode::PreviewOnly),
            "apply_scoped" => Ok(AccessMode::ApplyScoped),
            _ => Err(StorageError::Validation("invalid access mode")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SafetyMode {
    Strict,
    Normal,
    Experimental,
}

impl SafetyMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyMode::Strict => "strict",
            SafetyMode::Normal => "normal",
            SafetyMode::Experimental => "experimental",
        }
    }
}

impl TryFrom<&str> for SafetyMode {
    type Error = StorageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "strict" => Ok(SafetyMode::Strict),
            "normal" => Ok(SafetyMode::Normal),
            "experimental" => Ok(SafetyMode::Experimental),
            _ => Err(StorageError::Validation("invalid safety mode")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobMetrics {
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub total_tokens: u32,
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
    #[serde(default)]
    pub tokens_planner: u32,
    #[serde(default)]
    pub tokens_executor: u32,
    #[serde(default)]
    pub entities_read: u32,
    #[serde(default)]
    pub entities_written: u32,
    #[serde(default)]
    pub validators_run_count: u32,
}

impl JobMetrics {
    pub fn zero() -> Self {
        Self {
            duration_ms: 0,
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            tokens_planner: 0,
            tokens_executor: 0,
            entities_read: 0,
            entities_written: 0,
            validators_run_count: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiJob {
    pub job_id: Uuid,
    pub trace_id: Uuid,
    pub workflow_run_id: Option<Uuid>,
    pub job_kind: JobKind,
    pub state: JobState,
    pub error_message: Option<String>,
    pub protocol_id: String,
    pub profile_id: String,
    pub capability_profile_id: String,
    pub access_mode: AccessMode,
    pub safety_mode: SafetyMode,
    pub entity_refs: Vec<EntityRef>,
    pub planned_operations: Vec<PlannedOperation>,
    pub metrics: JobMetrics,
    pub status_reason: String,
    pub job_inputs: Option<Value>,
    pub job_outputs: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default)]
pub struct AiJobListFilter {
    pub status: Option<JobState>,
    pub job_kind: Option<JobKind>,
    pub wsid: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
pub struct NewAiJob {
    pub trace_id: Uuid,
    pub job_kind: JobKind,
    pub protocol_id: String,
    pub profile_id: String,
    pub capability_profile_id: String,
    pub access_mode: AccessMode,
    pub safety_mode: SafetyMode,
    pub entity_refs: Vec<EntityRef>,
    pub planned_operations: Vec<PlannedOperation>,
    pub status_reason: String,
    pub metrics: JobMetrics,
    pub job_inputs: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct JobStatusUpdate {
    pub job_id: Uuid,
    pub state: JobState,
    pub error_message: Option<String>,
    pub status_reason: String,
    pub metrics: Option<JobMetrics>,
    pub workflow_run_id: Option<Uuid>,
    pub trace_id: Option<Uuid>,
    pub job_outputs: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: Uuid,
    pub job_id: Uuid,
    pub status: JobState,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowNodeExecution {
    pub id: Uuid,
    pub workflow_run_id: Uuid,
    pub node_id: String,
    pub node_type: String,
    pub status: JobState,
    pub sequence: i64,
    pub input_payload: Option<Value>,
    pub output_payload: Option<Value>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewNodeExecution {
    pub workflow_run_id: Uuid,
    pub node_id: String,
    pub node_type: String,
    pub status: JobState,
    pub sequence: i64,
    pub input_payload: Option<Value>,
    pub started_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum WriteActorKind {
    Human,
    Ai,
    System,
}

pub type WriteActor = WriteActorKind;

impl WriteActorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            WriteActorKind::Human => "HUMAN",
            WriteActorKind::Ai => "AI",
            WriteActorKind::System => "SYSTEM",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MutationMetadata {
    pub actor_kind: WriteActorKind,
    pub actor_id: Option<String>,
    pub job_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub edit_event_id: Uuid,
    pub resource_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct WriteContext {
    pub actor_kind: WriteActorKind,
    pub actor_id: Option<String>,
    pub job_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
}

impl WriteContext {
    pub fn human(actor_id: Option<String>) -> Self {
        Self {
            actor_kind: WriteActorKind::Human,
            actor_id,
            job_id: None,
            workflow_id: None,
        }
    }

    pub fn system(actor_id: Option<String>) -> Self {
        Self {
            actor_kind: WriteActorKind::System,
            actor_id,
            job_id: None,
            workflow_id: None,
        }
    }

    pub fn ai(actor_id: Option<String>, job_id: Option<Uuid>, workflow_id: Option<Uuid>) -> Self {
        Self {
            actor_kind: WriteActorKind::Ai,
            actor_id,
            job_id,
            workflow_id,
        }
    }
}

#[derive(Debug, Error)]
pub enum GuardError {
    #[error("HSK-403-SILENT-EDIT")]
    SilentEdit,
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<GuardError> for StorageError {
    fn from(value: GuardError) -> Self {
        match value {
            GuardError::SilentEdit => StorageError::Guard("HSK-403-SILENT-EDIT"),
            GuardError::Storage(err) => err,
        }
    }
}

#[async_trait]
pub trait StorageGuard: Send + Sync {
    /// Validates the write request against the "No Silent Edits" policy.
    async fn validate_write(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> Result<MutationMetadata, GuardError>;
}

pub struct DefaultStorageGuard;

#[async_trait]
impl StorageGuard for DefaultStorageGuard {
    async fn validate_write(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> Result<MutationMetadata, GuardError> {
        if ctx.actor_kind == WriteActorKind::Ai
            && (ctx.job_id.is_none() || ctx.workflow_id.is_none())
        {
            return Err(GuardError::SilentEdit);
        }

        Ok(MutationMetadata {
            actor_kind: ctx.actor_kind,
            actor_id: ctx.actor_id.clone(),
            job_id: ctx.job_id,
            workflow_id: ctx.workflow_id,
            edit_event_id: Uuid::new_v4(),
            resource_id: resource_id.to_string(),
            timestamp: Utc::now(),
        })
    }
}

#[async_trait]
pub trait Database: Send + Sync + std::any::Any {
    // Health check
    async fn ping(&self) -> StorageResult<()>;

    // Workspace operations
    async fn list_workspaces(&self) -> StorageResult<Vec<Workspace>>;
    async fn create_workspace(
        &self,
        ctx: &WriteContext,
        workspace: NewWorkspace,
    ) -> StorageResult<Workspace>;
    async fn delete_workspace(&self, ctx: &WriteContext, id: &str) -> StorageResult<()>;
    async fn get_workspace(&self, id: &str) -> StorageResult<Option<Workspace>>;

    // Document operations
    async fn list_documents(&self, workspace_id: &str) -> StorageResult<Vec<Document>>;
    async fn get_document(&self, doc_id: &str) -> StorageResult<Document>;
    async fn create_document(
        &self,
        ctx: &WriteContext,
        doc: NewDocument,
    ) -> StorageResult<Document>;
    async fn delete_document(&self, ctx: &WriteContext, doc_id: &str) -> StorageResult<()>;

    // Block operations
    async fn get_blocks(&self, doc_id: &str) -> StorageResult<Vec<Block>>;
    async fn get_block(&self, block_id: &str) -> StorageResult<Block>;
    async fn create_block(&self, ctx: &WriteContext, block: NewBlock) -> StorageResult<Block>;
    async fn update_block(
        &self,
        ctx: &WriteContext,
        block_id: &str,
        data: BlockUpdate,
    ) -> StorageResult<()>;
    async fn delete_block(&self, ctx: &WriteContext, block_id: &str) -> StorageResult<()>;
    async fn replace_blocks(
        &self,
        ctx: &WriteContext,
        document_id: &str,
        blocks: Vec<NewBlock>,
    ) -> StorageResult<Vec<Block>>;

    // Canvas operations
    async fn create_canvas(&self, ctx: &WriteContext, canvas: NewCanvas) -> StorageResult<Canvas>;
    async fn list_canvases(&self, workspace_id: &str) -> StorageResult<Vec<Canvas>>;
    async fn get_canvas_with_graph(&self, canvas_id: &str) -> StorageResult<CanvasGraph>;
    async fn update_canvas_graph(
        &self,
        ctx: &WriteContext,
        canvas_id: &str,
        nodes: Vec<NewCanvasNode>,
        edges: Vec<NewCanvasEdge>,
    ) -> StorageResult<CanvasGraph>;
    async fn delete_canvas(&self, ctx: &WriteContext, canvas_id: &str) -> StorageResult<()>;

    // AI-Ready Data Architecture (Â§2.3.14)
    async fn create_ai_bronze_record(
        &self,
        ctx: &WriteContext,
        record: NewBronzeRecord,
    ) -> StorageResult<BronzeRecord>;
    async fn get_ai_bronze_record(&self, bronze_id: &str) -> StorageResult<Option<BronzeRecord>>;
    async fn list_ai_bronze_records(&self, workspace_id: &str) -> StorageResult<Vec<BronzeRecord>>;
    async fn mark_ai_bronze_deleted(
        &self,
        ctx: &WriteContext,
        bronze_id: &str,
    ) -> StorageResult<()>;

    async fn create_ai_silver_record(
        &self,
        ctx: &WriteContext,
        record: NewSilverRecord,
    ) -> StorageResult<SilverRecord>;
    async fn get_ai_silver_record(&self, silver_id: &str) -> StorageResult<Option<SilverRecord>>;
    async fn list_ai_silver_records_by_bronze(
        &self,
        bronze_id: &str,
    ) -> StorageResult<Vec<SilverRecord>>;
    async fn list_ai_silver_records(&self, workspace_id: &str) -> StorageResult<Vec<SilverRecord>>;
    async fn supersede_ai_silver_record(
        &self,
        ctx: &WriteContext,
        superseded_silver_id: &str,
        new_silver_id: &str,
    ) -> StorageResult<()>;

    async fn upsert_ai_embedding_model(
        &self,
        ctx: &WriteContext,
        model: EmbeddingModelRecord,
    ) -> StorageResult<()>;
    async fn list_ai_embedding_models(&self) -> StorageResult<Vec<EmbeddingModelRecord>>;
    async fn set_ai_embedding_default_model(
        &self,
        ctx: &WriteContext,
        model_id: &str,
        model_version: &str,
    ) -> StorageResult<()>;
    async fn get_ai_embedding_registry(&self) -> StorageResult<Option<EmbeddingRegistry>>;

    // AI Job operations (CX-DBP-021)
    async fn get_ai_job(&self, job_id: &str) -> StorageResult<AiJob>;
    async fn list_ai_jobs(&self, filter: AiJobListFilter) -> StorageResult<Vec<AiJob>>;
    async fn create_ai_job(&self, job: NewAiJob) -> StorageResult<AiJob>;
    async fn update_ai_job_status(&self, update: JobStatusUpdate) -> StorageResult<AiJob>;
    async fn set_job_outputs(&self, job_id: &str, outputs: Option<Value>) -> StorageResult<()>;

    // Workflow runs
    async fn create_workflow_run(
        &self,
        job_id: Uuid,
        status: JobState,
        last_heartbeat: Option<DateTime<Utc>>,
    ) -> StorageResult<WorkflowRun>;
    async fn update_workflow_run_status(
        &self,
        run_id: Uuid,
        status: JobState,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowRun>;
    async fn heartbeat_workflow(&self, run_id: Uuid, at: DateTime<Utc>) -> StorageResult<()>;
    async fn create_workflow_node_execution(
        &self,
        exec: NewNodeExecution,
    ) -> StorageResult<WorkflowNodeExecution>;
    async fn update_workflow_node_execution_status(
        &self,
        exec_id: Uuid,
        status: JobState,
        output: Option<Value>,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowNodeExecution>;
    async fn list_workflow_node_executions(
        &self,
        run_id: Uuid,
    ) -> StorageResult<Vec<WorkflowNodeExecution>>;
    async fn find_stalled_workflows(&self, threshold_secs: u64) -> StorageResult<Vec<WorkflowRun>>;

    // Mutation guard
    async fn validate_write_with_guard(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> StorageResult<MutationMetadata>;

    // AI Job Pruning [§2.3.11]
    async fn prune_ai_jobs(
        &self,
        cutoff: DateTime<Utc>,
        min_versions: u32,
        dry_run: bool,
    ) -> StorageResult<PruneReport>;

    /// Run database migrations.
    async fn run_migrations(&self) -> StorageResult<()>;

    /// Returns the current schema migration version from `_sqlx_migrations`.
    async fn migration_version(&self) -> StorageResult<i64>;

    fn as_any(&self) -> &dyn std::any::Any;
}

use std::sync::Arc;

/// [CX-DBP-041] Initialize the storage backend based on environment configuration.
/// Defaults to SQLite if DATABASE_URL is not provided or starts with sqlite://.
pub async fn init_storage() -> Result<Arc<dyn Database>, StorageError> {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        // Fallback to local sqlite file if not configured via env
        "sqlite://data/handshake.db".to_string()
    });

    if db_url.starts_with("sqlite://") {
        let db = sqlite::SqliteDatabase::connect(&db_url, 5).await?;
        db.run_migrations().await?;
        Ok(db.into_arc())
    } else if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
        let db = postgres::PostgresDatabase::connect(&db_url, 5).await?;
        db.run_migrations().await?;
        Ok(db.into_arc())
    } else {
        Err(StorageError::Validation("unsupported database protocol"))
    }
}

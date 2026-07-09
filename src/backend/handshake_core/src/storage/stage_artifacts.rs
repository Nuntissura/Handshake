//! WP-KERNEL-012 MT-066: the Stage capture artifact store — the evidence-grade
//! provenance record behind `GET /workspaces/:ws/stage/artifacts/:artifact_id`
//! (`stage_interop::StageClient::fetch_stage_artifact`).
//!
//! The Stage pane (Pillar 17) captures an inline text artifact — a document, a
//! selection, a canvas node, or an atelier item — as a PROVENANCE descriptor
//! (metadata, not content bytes). The frontend embed-back leg
//! (`stage_interop::StageArtifactRef::is_evidence_grade`) REFUSES an artifact
//! whose `sha256` OR `manifest_ref` is empty (`ProvenanceMissing`), so this
//! store guarantees BOTH are non-empty: `content_sha256` is the canonical-JSON
//! SHA-256 of the captured `content_json` (same canonical form as kernel
//! ContextBundle / `knowledge_rich_documents` hashing, so it is replayable) and
//! `manifest_ref` is `manifest://{artifact_id}`.
//!
//! Scope (MT-066, minimal viable): INLINE TEXT captures. Binary/blob captures
//! (e.g. `image/png` via an ArtifactStore handle) are DEFERRED — the frontend
//! GET is metadata-only so binary defers cleanly. Backed by
//! `stage_capture_artifacts` (migration 0341) over the shared PostgreSQL pool —
//! PostgreSQL authority only, no SQLite.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use super::StorageError;

/// The manifest schema token embedded in every Stage capture manifest JSONB.
pub const STAGE_CAPTURE_MANIFEST_SCHEMA: &str = "hsk.stage.capture_manifest@1";

/// The allowed Stage capture content kinds (mirrors the migration CHECK).
const STAGE_CONTENT_KINDS: [&str; 4] = ["document", "selection", "canvas_node", "atelier_item"];

/// One stored Stage capture artifact (the provenance descriptor, metadata only).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StageCaptureArtifact {
    pub artifact_id: String,
    pub workspace_id: String,
    pub content_kind: String,
    pub label: String,
    pub content_type: String,
    pub content_json: Value,
    /// Canonical-JSON SHA-256 of `content_json` (lowercase 64-hex). Never empty.
    pub content_sha256: String,
    /// The full manifest provenance descriptor (JSONB).
    pub manifest: Value,
    /// `manifest://{artifact_id}` — the manifest record reference. Never empty.
    pub manifest_ref: String,
    pub source_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert input for a new inline-text Stage capture artifact.
#[derive(Clone, Debug)]
pub struct NewStageCaptureArtifact {
    pub workspace_id: String,
    pub content_kind: String,
    pub label: String,
    pub content_type: String,
    pub content_json: Value,
    pub source_ref: Option<String>,
}

/// Pool-backed store for Stage capture artifacts. Cheap to construct per request
/// (wraps a pooled handle; never reconnects).
#[derive(Clone)]
pub struct StageArtifactStore {
    pool: PgPool,
}

impl StageArtifactStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Canonical-JSON SHA-256 of a value (same canonical form as kernel
    /// ContextBundle / `knowledge_rich_documents` hashing, so hashes replay).
    fn content_sha256(content: &Value) -> String {
        crate::kernel::context_bundle::sha256_hex(&crate::kernel::context_bundle::canonical_json_bytes(
            content,
        ))
    }

    /// Insert a new inline-text Stage capture artifact. Computes the
    /// canonical-JSON `content_sha256`, mints `artifact_id = STGA-{32 hex}`,
    /// builds the manifest JSONB + `manifest_ref = manifest://{artifact_id}`,
    /// and returns the stored row. The manifest ALWAYS carries a non-empty
    /// sha256 + manifest_ref (the evidence-grade twin of the frontend gate).
    pub async fn insert_stage_artifact(
        &self,
        input: NewStageCaptureArtifact,
    ) -> Result<StageCaptureArtifact, StorageError> {
        if input.workspace_id.trim().is_empty() {
            return Err(StorageError::Validation(
                "stage artifact workspace_id is required",
            ));
        }
        let content_kind = input.content_kind.trim();
        if !STAGE_CONTENT_KINDS.contains(&content_kind) {
            return Err(StorageError::Validation(
                "stage artifact content_kind must be document|selection|canvas_node|atelier_item",
            ));
        }
        let content_type = input.content_type.trim();
        if content_type.is_empty() {
            return Err(StorageError::Validation(
                "stage artifact content_type is required",
            ));
        }

        let artifact_id = format!("STGA-{}", Uuid::now_v7().simple());
        let content_sha256 = Self::content_sha256(&input.content_json);
        let manifest_ref = format!("manifest://{artifact_id}");
        // Normalize the optional source ref to a non-empty owned value.
        let source_ref = input
            .source_ref
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let manifest = json!({
            "schema": STAGE_CAPTURE_MANIFEST_SCHEMA,
            "sha256": content_sha256.clone(),
            "manifest_ref": manifest_ref.clone(),
            "content_type": content_type,
            "source_ref": source_ref.clone(),
        });

        let row = sqlx::query(
            r#"
            INSERT INTO stage_capture_artifacts
                (artifact_id, workspace_id, content_kind, label, content_type,
                 content_json, content_sha256, manifest, manifest_ref, source_ref)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING artifact_id, workspace_id, content_kind, label, content_type,
                      content_json, content_sha256, manifest, manifest_ref, source_ref,
                      created_at, updated_at
            "#,
        )
        .bind(&artifact_id)
        .bind(&input.workspace_id)
        .bind(content_kind)
        .bind(&input.label)
        .bind(content_type)
        .bind(&input.content_json)
        .bind(&content_sha256)
        .bind(&manifest)
        .bind(&manifest_ref)
        .bind(source_ref.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(StorageError::from)?;
        Ok(map_artifact_row(&row))
    }

    /// Read one Stage capture artifact by id, scoped to the workspace. Returns
    /// `None` when no such artifact exists in that workspace.
    pub async fn get_stage_artifact(
        &self,
        workspace_id: &str,
        artifact_id: &str,
    ) -> Result<Option<StageCaptureArtifact>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT artifact_id, workspace_id, content_kind, label, content_type,
                   content_json, content_sha256, manifest, manifest_ref, source_ref,
                   created_at, updated_at
            FROM stage_capture_artifacts
            WHERE workspace_id = $1 AND artifact_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(artifact_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from)?;
        Ok(row.as_ref().map(map_artifact_row))
    }
}

/// Map one `stage_capture_artifacts` row to the domain type.
fn map_artifact_row(row: &PgRow) -> StageCaptureArtifact {
    StageCaptureArtifact {
        artifact_id: row.get("artifact_id"),
        workspace_id: row.get("workspace_id"),
        content_kind: row.get("content_kind"),
        label: row.get("label"),
        content_type: row.get("content_type"),
        content_json: row.get("content_json"),
        content_sha256: row.get("content_sha256"),
        manifest: row.get("manifest"),
        manifest_ref: row.get("manifest_ref"),
        source_ref: row.get("source_ref"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

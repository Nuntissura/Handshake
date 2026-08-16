//! WP-KERNEL-012 MT-066/074: exact-byte Stage capture authority.
//!
//! The privileged API stores the exact bounded byte sequence, computes SHA-256
//! over those bytes, creates a portable manifest, a completed Job History row,
//! and an EventLedger receipt in one transaction. Idempotent retries
//! return the original artifact; reuse of a key with different input conflicts.
//!
//! The descriptor GET never substitutes for content retrieval: the dedicated
//! `/content` route returns the exact bytes and the native embed-back client
//! verifies size and digest before constructing an embed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use super::{postgres::append_kernel_event_with_executor, StorageError};
use crate::kernel::NewKernelEvent;

/// The manifest schema token embedded in every Stage capture manifest JSONB.
pub const STAGE_CAPTURE_MANIFEST_SCHEMA: &str = "hsk.stage.capture_manifest@1";

/// The allowed Stage capture content kinds (mirrors the migration CHECK).
const STAGE_CONTENT_KINDS: [&str; 4] = ["document", "selection", "canvas_node", "atelier_item"];

/// One stored Stage capture artifact plus its exact captured bytes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StageCaptureArtifact {
    pub artifact_id: String,
    pub workspace_id: String,
    pub content_kind: String,
    pub label: String,
    pub content_type: String,
    pub content_json: Value,
    /// Exact captured bytes. The embed-back client retrieves these from the
    /// dedicated content endpoint and verifies this byte sequence against
    /// `content_sha256` before constructing an embed.
    pub content_bytes: Vec<u8>,
    pub size_bytes: i64,
    /// SHA-256 of `content_bytes` (lowercase 64-hex). Never empty.
    pub content_sha256: String,
    /// The full manifest provenance descriptor (JSONB).
    pub manifest: Value,
    /// `manifest://{artifact_id}` — the manifest record reference. Never empty.
    pub manifest_ref: String,
    pub source_ref: Option<String>,
    pub idempotency_key: String,
    pub request_hash: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub correlation_id: String,
    pub approval_id: String,
    pub job_id: Option<String>,
    pub event_ledger_event_id: Option<String>,
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
    pub content_bytes: Vec<u8>,
    pub source_ref: Option<String>,
    pub idempotency_key: String,
    pub request_hash: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub correlation_id: String,
    pub approval_id: String,
    pub decision_receipt: NewKernelEvent,
    pub receipt: NewKernelEvent,
}

#[derive(Clone, Debug)]
pub struct StageArtifactInsertResult {
    pub artifact: StageCaptureArtifact,
    pub replayed: bool,
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

    fn content_sha256(content: &[u8]) -> String {
        hex::encode(Sha256::digest(content))
    }

    /// Insert a new bounded exact-byte Stage capture artifact. Computes the
    /// byte-exact `content_sha256`, mints `artifact_id = STGA-{32 hex}`,
    /// builds the manifest JSONB + `manifest_ref = manifest://{artifact_id}`,
    /// and returns the stored row. The manifest ALWAYS carries a non-empty
    /// sha256 + manifest_ref (the evidence-grade twin of the frontend gate).
    pub async fn insert_stage_artifact(
        &self,
        mut input: NewStageCaptureArtifact,
    ) -> Result<StageArtifactInsertResult, StorageError> {
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
        if input.content_bytes.is_empty() || input.content_bytes.len() > 16 * 1024 {
            return Err(StorageError::Validation(
                "stage artifact content_bytes must be 1..=16384 bytes",
            ));
        }
        if input.idempotency_key.trim().is_empty()
            || input.idempotency_key.len() > 256
            || input.request_hash.len() != 64
            || !input
                .request_hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || !matches!(input.actor_kind.as_str(), "operator" | "system")
            || input.actor_id.trim().is_empty()
            || input.correlation_id.trim().is_empty()
            || input.approval_id.trim().is_empty()
        {
            return Err(StorageError::Validation(
                "stage artifact privileged identity/idempotency contract is invalid",
            ));
        }

        let mut tx = self.pool.begin().await.map_err(StorageError::from)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("{}:{}", input.workspace_id, input.idempotency_key))
            .execute(&mut *tx)
            .await
            .map_err(StorageError::from)?;

        if let Some(row) = sqlx::query(&format!(
            "{} WHERE workspace_id = $1 AND idempotency_key = $2",
            select_artifact_sql()
        ))
        .bind(&input.workspace_id)
        .bind(&input.idempotency_key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(StorageError::from)?
        {
            let existing = map_artifact_row(&row);
            if existing.request_hash != input.request_hash {
                return Err(StorageError::Conflict(
                    "stage capture idempotency key was reused with a different request",
                ));
            }
            tx.commit().await.map_err(StorageError::from)?;
            return Ok(StageArtifactInsertResult {
                artifact: existing,
                replayed: true,
            });
        }

        let artifact_id = format!("STGA-{}", Uuid::now_v7().simple());
        let content_sha256 = Self::content_sha256(&input.content_bytes);
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
            "size_bytes": input.content_bytes.len(),
            "correlation_id": input.correlation_id.clone(),
        });

        let job_id = Uuid::now_v7().to_string();
        let trace_id = Uuid::now_v7().to_string();
        let now = Utc::now();
        let entity_refs = json!([
            {"entity_id": input.workspace_id.clone(), "entity_kind": "workspace"},
            {"entity_id": artifact_id.clone(), "entity_kind": "stage_capture_artifact"}
        ])
        .to_string();
        let job_inputs = json!({
            "workspace_id": input.workspace_id.clone(),
            "artifact_id": artifact_id.clone(),
            "content_kind": content_kind,
            "content_sha256": content_sha256,
            "size_bytes": input.content_bytes.len(),
            "source_ref": source_ref,
            "correlation_id": input.correlation_id.clone(),
            "approval_id": input.approval_id.clone(),
        })
        .to_string();
        let job_outputs = json!({
            "artifact_id": artifact_id.clone(),
            "artifact_ref": format!("artifact://sha256/{content_sha256}"),
            "manifest_ref": manifest_ref.clone(),
            "sha256": content_sha256.clone(),
        })
        .to_string();
        sqlx::query(
            r#"
            INSERT INTO ai_jobs (
                id, trace_id, workflow_run_id, job_kind, status, status_reason,
                protocol_id, profile_id, capability_profile_id, access_mode,
                safety_mode, entity_refs, planned_operations, metrics, job_inputs,
                job_outputs, created_at, updated_at
            ) VALUES (
                $1, $2, NULL, 'workflow_run', 'completed', 'stage_capture_stored',
                'hsk.stage.capture@1', 'default', 'stage.jobs.enqueue', 'apply_scoped',
                'strict', $3, '[]', '{}', $4, $5, $6, $6
            )
            "#,
        )
        .bind(&job_id)
        .bind(&trace_id)
        .bind(entity_refs)
        .bind(job_inputs)
        .bind(job_outputs)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(StorageError::from)?;

        input.decision_receipt.aggregate_id = artifact_id.clone();
        input.decision_receipt.payload = json!({
            "workspace_id": input.workspace_id.clone(),
            "artifact_id": artifact_id.clone(),
            "capability_id": "stage.jobs.enqueue",
            "decision_outcome": "allow",
            "approval_id": input.approval_id.clone(),
            "correlation_id": input.correlation_id.clone(),
            "job_id": job_id.clone(),
        });
        input.decision_receipt.payload_hash = crate::kernel::context_bundle::sha256_hex(
            &crate::kernel::context_bundle::canonical_json_bytes(&input.decision_receipt.payload),
        );
        let decision_event =
            append_kernel_event_with_executor(&mut *tx, input.decision_receipt).await?;

        input.receipt.aggregate_id = artifact_id.clone();
        input.receipt.payload = json!({
            "receipt_kind": "stage_capture_stored",
            "workspace_id": input.workspace_id.clone(),
            "artifact_id": artifact_id.clone(),
            "artifact_ref": format!("artifact://sha256/{content_sha256}"),
            "manifest_ref": manifest_ref.clone(),
            "sha256": content_sha256.clone(),
            "size_bytes": input.content_bytes.len(),
            "content_kind": content_kind,
            "source_ref": source_ref,
            "job_id": job_id.clone(),
            "capability_id": "stage.jobs.enqueue",
            "decision_outcome": "allow",
            "decision_event_id": decision_event.event_id,
            "approval_id": input.approval_id.clone(),
            "correlation_id": input.correlation_id.clone(),
        });
        input.receipt.payload_hash = crate::kernel::context_bundle::sha256_hex(
            &crate::kernel::context_bundle::canonical_json_bytes(&input.receipt.payload),
        );
        let stored_event = append_kernel_event_with_executor(&mut *tx, input.receipt).await?;

        let row = sqlx::query(
            r#"
            INSERT INTO stage_capture_artifacts
                (artifact_id, workspace_id, content_kind, label, content_type,
                 content_json, content_bytes, size_bytes, content_sha256, manifest,
                 manifest_ref, source_ref, idempotency_key, request_hash,
                 actor_kind, actor_id, correlation_id, approval_id, job_id,
                 event_ledger_event_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                    $13, $14, $15, $16, $17, $18, $19, $20)
            RETURNING artifact_id, workspace_id, content_kind, label, content_type,
                      content_json, content_bytes, size_bytes, content_sha256,
                      manifest, manifest_ref, source_ref, idempotency_key,
                      request_hash, actor_kind, actor_id, correlation_id,
                      approval_id, job_id, event_ledger_event_id, created_at, updated_at
            "#,
        )
        .bind(&artifact_id)
        .bind(&input.workspace_id)
        .bind(content_kind)
        .bind(&input.label)
        .bind(content_type)
        .bind(&input.content_json)
        .bind(&input.content_bytes)
        .bind(input.content_bytes.len() as i64)
        .bind(&content_sha256)
        .bind(&manifest)
        .bind(&manifest_ref)
        .bind(source_ref.as_deref())
        .bind(&input.idempotency_key)
        .bind(&input.request_hash)
        .bind(&input.actor_kind)
        .bind(&input.actor_id)
        .bind(&input.correlation_id)
        .bind(&input.approval_id)
        .bind(&job_id)
        .bind(&stored_event.event_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(StorageError::from)?;
        let artifact = map_artifact_row(&row);
        tx.commit().await.map_err(StorageError::from)?;
        Ok(StageArtifactInsertResult {
            artifact,
            replayed: false,
        })
    }

    /// Read one Stage capture artifact by id, scoped to the workspace. Returns
    /// `None` when no such artifact exists in that workspace.
    pub async fn get_stage_artifact(
        &self,
        workspace_id: &str,
        artifact_id: &str,
    ) -> Result<Option<StageCaptureArtifact>, StorageError> {
        let row = sqlx::query(&format!(
            "{} WHERE workspace_id = $1 AND artifact_id = $2",
            select_artifact_sql()
        ))
        .bind(workspace_id)
        .bind(artifact_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from)?;
        Ok(row.as_ref().map(map_artifact_row))
    }
}

fn select_artifact_sql() -> &'static str {
    r#"
    SELECT artifact_id, workspace_id, content_kind, label, content_type,
           content_json, content_bytes, size_bytes, content_sha256, manifest,
           manifest_ref, source_ref, idempotency_key, request_hash, actor_kind,
           actor_id, correlation_id, approval_id, job_id, event_ledger_event_id,
           created_at, updated_at
    FROM stage_capture_artifacts
    "#
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
        content_bytes: row.get("content_bytes"),
        size_bytes: row.get("size_bytes"),
        content_sha256: row.get("content_sha256"),
        manifest: row.get("manifest"),
        manifest_ref: row.get("manifest_ref"),
        source_ref: row.get("source_ref"),
        idempotency_key: row.get("idempotency_key"),
        request_hash: row.get("request_hash"),
        actor_kind: row.get("actor_kind"),
        actor_id: row.get("actor_id"),
        correlation_id: row.get("correlation_id"),
        approval_id: row.get("approval_id"),
        job_id: row.get("job_id"),
        event_ledger_event_id: row.get("event_ledger_event_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

//! Character-sheet exports + share packs (MT-199, CKC fold-in).
//!
//! Translates CastKit-Codex `library.js` export surface
//! (`exportBundle`, `exportSharePack`, `_createSheetVersion` export columns,
//! and the web-portfolio manifest model) into the Handshake atelier domain.
//!
//! CKC source: app/backend/library.js (`exportBundle` ~L6530, `exportSharePack`
//! ~L6603, `_createSheetVersion` export_format/export_relative_path ~L5592),
//! app/backend/backup.js (manifest intent). CKC stores exports as files on disk
//! under `exports/`; Handshake instead records each export as a durable
//! request -> result -> manifest-entry graph in PostgreSQL, where rendered bytes
//! live in the ArtifactStore (`artifact_ref`) and are never written to random
//! filesystem paths or `.GOV`. SQLite is forbidden (MT-004).
//!
//! Data contract (MT-199):
//!   * `atelier_export_request`  - an operator/model ask to export a character,
//!     pinned to an immutable append-only sheet version (reuses MT-012) and a
//!     requested format. This is the unit operators retry / audit.
//!   * `atelier_export_result`   - the rendered output for a request: the
//!     ArtifactStore ref + content hash + byte length of the produced file.
//!   * `atelier_export_manifest_entry` - the share-pack manifest: the ordered
//!     set of items (sheet, media assets) bundled under one export, mirroring
//!     CKC `manifest.json`. Reuses `atelier_media_asset` for image entries.
//!
//! Events (MT-005): every mutation emits a new atelier event family defined in
//! this module so MT-005 coverage extends to the export seam.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierError, AtelierResult, AtelierStore};

/// Atelier export event families (MT-199). Defined here, surfaced to MT-005
/// coverage by the parent appending these to `event_family::ALL`.
pub mod export_event_family {
    /// An export was requested for a character + pinned sheet version.
    pub const EXPORT_REQUESTED: &str = "atelier.export.requested";
    /// A requested export was rendered into an ArtifactStore-backed result.
    pub const EXPORT_RENDERED: &str = "atelier.export.rendered";
    /// An item (sheet/media) was attached to a share-pack manifest.
    pub const EXPORT_MANIFEST_ITEM_ADDED: &str = "atelier.export.manifest_item_added";

    /// All export event families (parity/coverage helper, mirrors
    /// `event_family::ALL` shape).
    pub const ALL: &[&str] = &[
        EXPORT_REQUESTED,
        EXPORT_RENDERED,
        EXPORT_MANIFEST_ITEM_ADDED,
    ];
}

/// Re-export at module root so callers can write `exports::EXPORT_REQUESTED`.
pub use export_event_family::{
    EXPORT_MANIFEST_ITEM_ADDED, EXPORT_RENDERED, EXPORT_REQUESTED,
};

/// Output format for a character-sheet export (MT-199).
///
/// Mirrors CKC `exportBundle` (txt/md/pdf) plus a structured JSON form. Stored
/// as a stable lowercase token; PDF rendering itself happens out-of-band (CKC
/// renders PDF in the Electron main process) -- the request/result graph is
/// format-agnostic and only pins which format was asked for and produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    PlainText,
    Markdown,
    Json,
    Pdf,
}

impl ExportFormat {
    /// Stable DB token (also the CKC `export_format` value where it overlaps).
    pub fn as_token(self) -> &'static str {
        match self {
            ExportFormat::PlainText => "txt",
            ExportFormat::Markdown => "md",
            ExportFormat::Json => "json",
            ExportFormat::Pdf => "pdf",
        }
    }

    /// Parse a stored token back into a format. Unknown tokens are a validation
    /// error rather than a silent default, so a corrupt row never masquerades
    /// as plain text.
    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "txt" => Ok(ExportFormat::PlainText),
            "md" => Ok(ExportFormat::Markdown),
            "json" => Ok(ExportFormat::Json),
            "pdf" => Ok(ExportFormat::Pdf),
            other => Err(AtelierError::Validation(format!(
                "unknown export format token: {other}"
            ))),
        }
    }
}

/// Lifecycle status of an export request (MT-199).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    /// Requested but not yet rendered.
    Pending,
    /// A result row was recorded for this request.
    Rendered,
}

impl ExportStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            ExportStatus::Pending => "pending",
            ExportStatus::Rendered => "rendered",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "pending" => Ok(ExportStatus::Pending),
            "rendered" => Ok(ExportStatus::Rendered),
            other => Err(AtelierError::Validation(format!(
                "unknown export status token: {other}"
            ))),
        }
    }
}

/// A durable export request pinned to an immutable sheet version (MT-199).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportRequest {
    pub export_id: Uuid,
    pub character_internal_id: Uuid,
    /// Pinned append-only sheet version (MT-012); the export reflects exactly
    /// this snapshot even if the sheet is edited later.
    pub sheet_version_id: Uuid,
    pub format: ExportFormat,
    pub status: ExportStatus,
    /// Operator-facing label for the share pack / bundle (e.g. CKC setName).
    pub label: Option<String>,
    pub requested_by: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Parameters to open a new export request.
#[derive(Clone, Debug)]
pub struct NewExportRequest {
    pub character_internal_id: Uuid,
    pub sheet_version_id: Uuid,
    pub format: ExportFormat,
    pub label: Option<String>,
    pub requested_by: String,
}

/// The rendered artifact for an export request (MT-199).
///
/// Bytes live in the ArtifactStore behind `artifact_ref`; `content_hash` lets a
/// share pack be deduped/verified the same way CKC stored `sheet_bytes_hash`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportResult {
    pub result_id: Uuid,
    pub export_id: Uuid,
    pub artifact_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub created_at_utc: DateTime<Utc>,
}

/// Kind of item bundled into a share-pack manifest (MT-199).
///
/// Mirrors CKC `manifest.json` sections (`includeSheet`, `images`, `docs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestItemKind {
    /// The exported character sheet itself.
    Sheet,
    /// A media asset (image) from the DAM (MT-015).
    Media,
}

impl ManifestItemKind {
    pub fn as_token(self) -> &'static str {
        match self {
            ManifestItemKind::Sheet => "sheet",
            ManifestItemKind::Media => "media",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "sheet" => Ok(ManifestItemKind::Sheet),
            "media" => Ok(ManifestItemKind::Media),
            other => Err(AtelierError::Validation(format!(
                "unknown manifest item kind token: {other}"
            ))),
        }
    }
}

/// One ordered entry in a share-pack manifest (MT-199).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestEntry {
    pub entry_id: Uuid,
    pub export_id: Uuid,
    pub seq: i64,
    pub kind: ManifestItemKind,
    /// ArtifactStore ref for the bundled bytes (sheet result or media asset).
    pub artifact_ref: String,
    /// Stable relative path inside the share pack (mirrors CKC manifest paths,
    /// e.g. `sheet/character.txt`, `images/<id>__name.png`).
    pub pack_path: String,
    pub created_at_utc: DateTime<Utc>,
}

fn request_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ExportRequest> {
    let format_token: String = row.get("format");
    let status_token: String = row.get("status");
    Ok(ExportRequest {
        export_id: row.get("export_id"),
        character_internal_id: row.get("character_internal_id"),
        sheet_version_id: row.get("sheet_version_id"),
        format: ExportFormat::from_token(&format_token)?,
        status: ExportStatus::from_token(&status_token)?,
        label: row.get("label"),
        requested_by: row.get("requested_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn result_from_row(row: &sqlx::postgres::PgRow) -> ExportResult {
    ExportResult {
        result_id: row.get("result_id"),
        export_id: row.get("export_id"),
        artifact_ref: row.get("artifact_ref"),
        content_hash: row.get("content_hash"),
        byte_len: row.get("byte_len"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn entry_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ManifestEntry> {
    let kind_token: String = row.get("kind");
    Ok(ManifestEntry {
        entry_id: row.get("entry_id"),
        export_id: row.get("export_id"),
        seq: row.get("seq"),
        kind: ManifestItemKind::from_token(&kind_token)?,
        artifact_ref: row.get("artifact_ref"),
        pack_path: row.get("pack_path"),
        created_at_utc: row.get("created_at_utc"),
    })
}

impl AtelierStore {
    /// Open an export request pinned to a specific sheet version (MT-199).
    ///
    /// The sheet version is validated to belong to the named character so an
    /// export can never silently bundle another character's snapshot. Reuses
    /// the append-only sheet history (MT-012): the export reflects exactly the
    /// pinned version even if the sheet is edited afterwards.
    pub async fn request_sheet_export(
        &self,
        new: &NewExportRequest,
    ) -> AtelierResult<ExportRequest> {
        if new.requested_by.trim().is_empty() {
            return Err(AtelierError::Validation(
                "requested_by must not be empty".into(),
            ));
        }

        // Pin validation: the sheet version must exist and belong to the
        // character being exported.
        let owner: Option<Uuid> = sqlx::query_scalar(
            "SELECT character_internal_id FROM atelier_sheet_version WHERE version_id = $1",
        )
        .bind(new.sheet_version_id)
        .fetch_optional(self.pool())
        .await?;
        match owner {
            None => {
                return Err(AtelierError::NotFound(format!(
                    "sheet version_id={}",
                    new.sheet_version_id
                )));
            }
            Some(owner_id) if owner_id != new.character_internal_id => {
                return Err(AtelierError::Validation(format!(
                    "sheet version {} does not belong to character {}",
                    new.sheet_version_id, new.character_internal_id
                )));
            }
            Some(_) => {}
        }

        let row = sqlx::query(
            r#"INSERT INTO atelier_export_request
                 (character_internal_id, sheet_version_id, format, status, label, requested_by)
               VALUES ($1, $2, $3, 'pending', $4, $5)
               RETURNING export_id, character_internal_id, sheet_version_id, format,
                         status, label, requested_by, created_at_utc"#,
        )
        .bind(new.character_internal_id)
        .bind(new.sheet_version_id)
        .bind(new.format.as_token())
        .bind(&new.label)
        .bind(&new.requested_by)
        .fetch_one(self.pool())
        .await?;
        let request = request_from_row(&row)?;

        self.record_event(
            EXPORT_REQUESTED,
            "atelier_export_request",
            &request.export_id.to_string(),
            serde_json::json!({
                "character_internal_id": request.character_internal_id,
                "sheet_version_id": request.sheet_version_id,
                "format": request.format.as_token(),
                "requested_by": request.requested_by,
            }),
        )
        .await?;
        Ok(request)
    }

    /// Fetch an export request by id.
    pub async fn get_export_request(&self, export_id: Uuid) -> AtelierResult<ExportRequest> {
        let row = sqlx::query(
            r#"SELECT export_id, character_internal_id, sheet_version_id, format,
                      status, label, requested_by, created_at_utc
               FROM atelier_export_request WHERE export_id = $1"#,
        )
        .bind(export_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("export_id={export_id}")))?;
        request_from_row(&row)
    }

    /// Record the rendered artifact for a pending export and flip it to
    /// `rendered` (MT-199).
    ///
    /// Idempotent on `(export_id, content_hash)`: re-recording identical bytes
    /// returns the existing result rather than duplicating it, mirroring the
    /// content-hash dedup used by the DAM (MT-015) and CKC `sheet_bytes_hash`.
    /// Recording a result for an unknown request is a not-found error so a
    /// result can never dangle.
    pub async fn record_export_result(
        &self,
        export_id: Uuid,
        artifact_ref: &str,
        content_hash: &str,
        byte_len: i64,
    ) -> AtelierResult<ExportResult> {
        if artifact_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "artifact_ref must not be empty".into(),
            ));
        }
        // Guard: the request must exist (also flips status below).
        let _ = self.get_export_request(export_id).await?;

        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_export_result
                 (export_id, artifact_ref, content_hash, byte_len)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (export_id, content_hash)
                 DO UPDATE SET artifact_ref = EXCLUDED.artifact_ref
               RETURNING result_id, export_id, artifact_ref, content_hash,
                         byte_len, created_at_utc"#,
        )
        .bind(export_id)
        .bind(artifact_ref)
        .bind(content_hash)
        .bind(byte_len)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE atelier_export_request SET status = 'rendered' WHERE export_id = $1")
            .bind(export_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        let result = result_from_row(&row);
        self.record_event(
            EXPORT_RENDERED,
            "atelier_export_request",
            &export_id.to_string(),
            serde_json::json!({
                "result_id": result.result_id,
                "content_hash": result.content_hash,
                "byte_len": result.byte_len,
            }),
        )
        .await?;
        Ok(result)
    }

    /// The rendered result for an export, if one has been recorded.
    pub async fn get_export_result(
        &self,
        export_id: Uuid,
    ) -> AtelierResult<Option<ExportResult>> {
        let row = sqlx::query(
            r#"SELECT result_id, export_id, artifact_ref, content_hash, byte_len,
                      created_at_utc
               FROM atelier_export_result WHERE export_id = $1
               ORDER BY created_at_utc DESC LIMIT 1"#,
        )
        .bind(export_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(result_from_row))
    }

    /// Attach an item to an export's share-pack manifest (MT-199).
    ///
    /// Computes the next manifest sequence inside a transaction so concurrent
    /// adds cannot collide on `(export_id, seq)`. Mirrors CKC `exportSharePack`
    /// assembling `manifest.images` / `manifest.docs`; here every bundled item
    /// is an ArtifactStore ref plus a stable relative pack path.
    pub async fn add_manifest_entry(
        &self,
        export_id: Uuid,
        kind: ManifestItemKind,
        artifact_ref: &str,
        pack_path: &str,
    ) -> AtelierResult<ManifestEntry> {
        if pack_path.trim().is_empty() {
            return Err(AtelierError::Validation("pack_path must not be empty".into()));
        }
        // Guard: the export must exist.
        let _ = self.get_export_request(export_id).await?;

        let mut tx = self.pool().begin().await?;

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM atelier_export_manifest_entry WHERE export_id = $1",
        )
        .bind(export_id)
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_export_manifest_entry
                 (export_id, seq, kind, artifact_ref, pack_path)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING entry_id, export_id, seq, kind, artifact_ref, pack_path,
                         created_at_utc"#,
        )
        .bind(export_id)
        .bind(next_seq)
        .bind(kind.as_token())
        .bind(artifact_ref)
        .bind(pack_path)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let entry = entry_from_row(&row)?;
        self.record_event(
            EXPORT_MANIFEST_ITEM_ADDED,
            "atelier_export_request",
            &export_id.to_string(),
            serde_json::json!({
                "entry_id": entry.entry_id,
                "seq": entry.seq,
                "kind": entry.kind.as_token(),
                "pack_path": entry.pack_path,
            }),
        )
        .await?;
        Ok(entry)
    }

    /// The ordered share-pack manifest for an export (ascending sequence),
    /// mirroring CKC `manifest.json` item ordering.
    pub async fn export_manifest(&self, export_id: Uuid) -> AtelierResult<Vec<ManifestEntry>> {
        let rows = sqlx::query(
            r#"SELECT entry_id, export_id, seq, kind, artifact_ref, pack_path,
                      created_at_utc
               FROM atelier_export_manifest_entry
               WHERE export_id = $1
               ORDER BY seq ASC"#,
        )
        .bind(export_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(entry_from_row).collect()
    }
}

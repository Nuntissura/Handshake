//! Bulk atelier operations (MT-014).
//!
//! Bulk mutations validate the complete target set before writing, then commit
//! all target changes, one durable receipt, and one EventLedger event in the
//! same PostgreSQL transaction.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};
use std::collections::HashSet;
use uuid::Uuid;

use super::exports::{
    ExportFormat, ExportRequest, ExportStatus, NewExportRequest, EXPORT_REQUESTED,
};
use super::search::{normalize_tag, search_event_family};
use super::{event_family, AtelierError, AtelierResult, AtelierStore};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BulkOperationReceipt {
    pub receipt_id: Uuid,
    pub operation: String,
    pub requested_by: String,
    pub target_count: i64,
    pub mutation_count: i64,
    pub status: String,
    pub payload: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct BulkTagRequest {
    pub character_internal_ids: Vec<Uuid>,
    pub tags: Vec<String>,
    pub requested_by: String,
}

#[derive(Clone, Debug)]
pub struct BulkTrashMediaRequest {
    pub asset_ids: Vec<Uuid>,
    pub reason: String,
    pub requested_by: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DeletionTargetKind {
    MediaAsset,
    SheetVersion,
}

impl DeletionTargetKind {
    pub fn as_token(self) -> &'static str {
        match self {
            Self::MediaAsset => "media_asset",
            Self::SheetVersion => "sheet_version",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DeletionTargetRef {
    pub target_type: DeletionTargetKind,
    pub target_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletionImpactPreviewRequest {
    pub targets: Vec<DeletionTargetRef>,
    pub reason: String,
    pub requested_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletionArchiveRequest {
    pub targets: Vec<DeletionTargetRef>,
    pub reason: String,
    pub requested_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletionRestoreRequest {
    pub targets: Vec<DeletionTargetRef>,
    pub reason: String,
    pub requested_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletionImpactTarget {
    pub target_type: DeletionTargetKind,
    pub target_id: Uuid,
    pub currently_archived: bool,
    pub would_archive: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletionImpactPreview {
    pub requested_by: String,
    pub reason: String,
    pub target_count: i64,
    pub would_archive_count: i64,
    pub already_archived_count: i64,
    pub targets: Vec<DeletionImpactTarget>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BulkExportRequestResult {
    pub receipt: BulkOperationReceipt,
    pub exports: Vec<ExportRequest>,
}

fn receipt_from_row(row: &sqlx::postgres::PgRow) -> BulkOperationReceipt {
    BulkOperationReceipt {
        receipt_id: row.get("receipt_id"),
        operation: row.get("operation"),
        requested_by: row.get("requested_by"),
        target_count: row.get("target_count"),
        mutation_count: row.get("mutation_count"),
        status: row.get("status"),
        payload: row.get("payload"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn export_request_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ExportRequest> {
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

fn dedup_uuids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut seen = HashSet::new();
    ids.iter().copied().filter(|id| seen.insert(*id)).collect()
}

fn dedup_deletion_targets(targets: &[DeletionTargetRef]) -> Vec<DeletionTargetRef> {
    let mut seen = HashSet::new();
    targets
        .iter()
        .filter(|target| seen.insert((target.target_type, target.target_id)))
        .cloned()
        .collect()
}

fn normalize_tags(tags: &[String]) -> AtelierResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = normalize_tag(tag);
        if tag.is_empty() {
            return Err(AtelierError::Validation(
                "bulk tag text must not be empty".into(),
            ));
        }
        if seen.insert(tag.clone()) {
            normalized.push(tag);
        }
    }
    Ok(normalized)
}

fn require_requester(requested_by: &str) -> AtelierResult<&str> {
    let requested_by = requested_by.trim();
    if requested_by.is_empty() {
        return Err(AtelierError::Validation(
            "requested_by must not be empty".into(),
        ));
    }
    Ok(requested_by)
}

fn require_reason(reason: &str) -> AtelierResult<&str> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AtelierError::Validation("reason must not be empty".into()));
    }
    Ok(reason)
}

async fn require_all_characters_exist(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> AtelierResult<()> {
    if ids.is_empty() {
        return Err(AtelierError::Validation(
            "bulk operation requires at least one character target".into(),
        ));
    }
    let existing: Vec<Uuid> =
        sqlx::query_scalar("SELECT internal_id FROM atelier_character WHERE internal_id = ANY($1)")
            .bind(ids)
            .fetch_all(&mut **tx)
            .await?;
    if existing.len() == ids.len() {
        return Ok(());
    }
    let existing: HashSet<Uuid> = existing.into_iter().collect();
    let missing: Vec<String> = ids
        .iter()
        .filter(|id| !existing.contains(id))
        .map(Uuid::to_string)
        .collect();
    Err(AtelierError::NotFound(format!(
        "bulk character targets missing: {}",
        missing.join(", ")
    )))
}

async fn require_all_media_assets_exist(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> AtelierResult<()> {
    if ids.is_empty() {
        return Err(AtelierError::Validation(
            "bulk trash requires at least one media asset target".into(),
        ));
    }
    let existing: Vec<Uuid> =
        sqlx::query_scalar("SELECT asset_id FROM atelier_media_asset WHERE asset_id = ANY($1)")
            .bind(ids)
            .fetch_all(&mut **tx)
            .await?;
    if existing.len() == ids.len() {
        return Ok(());
    }
    let existing: HashSet<Uuid> = existing.into_iter().collect();
    let missing: Vec<String> = ids
        .iter()
        .filter(|id| !existing.contains(id))
        .map(Uuid::to_string)
        .collect();
    Err(AtelierError::NotFound(format!(
        "bulk media targets missing: {}",
        missing.join(", ")
    )))
}

async fn collect_deletion_target_states(
    tx: &mut Transaction<'_, Postgres>,
    targets: &[DeletionTargetRef],
) -> AtelierResult<Vec<DeletionImpactTarget>> {
    if targets.is_empty() {
        return Err(AtelierError::Validation(
            "deletion operation requires at least one target".into(),
        ));
    }

    let mut states = Vec::with_capacity(targets.len());
    for target in targets {
        let target_type = target.target_type.as_token();
        let exists: bool = match target.target_type {
            DeletionTargetKind::MediaAsset => {
                sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM atelier_media_asset WHERE asset_id = $1)",
                )
                .bind(target.target_id)
                .fetch_one(&mut **tx)
                .await?
            }
            DeletionTargetKind::SheetVersion => {
                sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM atelier_sheet_version WHERE version_id = $1)",
                )
                .bind(target.target_id)
                .fetch_one(&mut **tx)
                .await?
            }
        };
        if !exists {
            return Err(AtelierError::NotFound(format!(
                "{target_type} target_id={}",
                target.target_id
            )));
        }

        let currently_archived: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                   SELECT 1 FROM atelier_trash_marker
                   WHERE target_type = $1 AND target_id = $2
               )"#,
        )
        .bind(target_type)
        .bind(target.target_id)
        .fetch_one(&mut **tx)
        .await?;

        states.push(DeletionImpactTarget {
            target_type: target.target_type,
            target_id: target.target_id,
            currently_archived,
            would_archive: !currently_archived,
        });
    }

    Ok(states)
}

impl AtelierStore {
    pub(crate) async fn record_bulk_operation_receipt_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        operation: &str,
        requested_by: &str,
        target_count: i64,
        mutation_count: i64,
        payload: serde_json::Value,
    ) -> AtelierResult<BulkOperationReceipt> {
        let row = sqlx::query(
            r#"INSERT INTO atelier_bulk_operation_receipt
                 (operation, requested_by, target_count, mutation_count, status, payload)
               VALUES ($1, $2, $3, $4, 'applied', $5)
               RETURNING receipt_id, operation, requested_by, target_count,
                         mutation_count, status, payload, created_at_utc"#,
        )
        .bind(operation)
        .bind(requested_by)
        .bind(target_count)
        .bind(mutation_count)
        .bind(payload)
        .fetch_one(&mut **tx)
        .await?;
        let receipt = receipt_from_row(&row);
        self.record_event_in_tx(
            tx,
            event_family::BULK_OPERATION_APPLIED,
            "atelier_bulk_operation_receipt",
            &receipt.receipt_id.to_string(),
            serde_json::json!({
                "receipt_id": receipt.receipt_id,
                "operation": receipt.operation,
                "requested_by": receipt.requested_by,
                "target_count": receipt.target_count,
                "mutation_count": receipt.mutation_count,
                "status": receipt.status,
                "receipt_payload": receipt.payload,
            }),
        )
        .await?;
        Ok(receipt)
    }

    pub async fn get_bulk_operation_receipt(
        &self,
        receipt_id: Uuid,
    ) -> AtelierResult<BulkOperationReceipt> {
        let row = sqlx::query(
            r#"SELECT receipt_id, operation, requested_by, target_count,
                      mutation_count, status, payload, created_at_utc
               FROM atelier_bulk_operation_receipt
               WHERE receipt_id = $1"#,
        )
        .bind(receipt_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("bulk receipt_id={receipt_id}")))?;
        Ok(receipt_from_row(&row))
    }

    pub async fn bulk_tag_characters_with_receipt(
        &self,
        request: &BulkTagRequest,
    ) -> AtelierResult<BulkOperationReceipt> {
        let requested_by = require_requester(&request.requested_by)?;
        let character_ids = dedup_uuids(&request.character_internal_ids);
        let tags = normalize_tags(&request.tags)?;
        if tags.is_empty() {
            return Err(AtelierError::Validation(
                "bulk tag requires at least one tag".into(),
            ));
        }

        let mut tx = self.pool().begin().await?;
        require_all_characters_exist(&mut tx, &character_ids).await?;

        let mut tag_ids = Vec::with_capacity(tags.len());
        for tag in &tags {
            let tag_id: Uuid = sqlx::query_scalar(
                r#"INSERT INTO atelier_tag (text)
                   VALUES ($1)
                   ON CONFLICT (text) DO UPDATE SET text = EXCLUDED.text
                   RETURNING tag_id"#,
            )
            .bind(tag)
            .fetch_one(&mut *tx)
            .await?;
            tag_ids.push(tag_id);
        }

        let mut written: i64 = 0;
        for character_id in &character_ids {
            for tag_id in &tag_ids {
                let result = sqlx::query(
                    r#"INSERT INTO atelier_character_tag
                         (character_internal_id, tag_id, tag_type)
                       VALUES ($1, $2, 'manual')
                       ON CONFLICT (character_internal_id, tag_id)
                       DO UPDATE SET tag_type = EXCLUDED.tag_type"#,
                )
                .bind(character_id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await?;
                written += result.rows_affected() as i64;
            }
        }

        self.record_event_in_tx(
            &mut tx,
            search_event_family::CHARACTER_TAGGED,
            "atelier_character_tag",
            "bulk",
            serde_json::json!({
                "tag_ids": &tag_ids,
                "tags": &tags,
                "character_count": character_ids.len(),
                "tag_count": tag_ids.len(),
                "links_written": written,
                "mode": "bulk_manual",
            }),
        )
        .await?;

        let receipt = self
            .record_bulk_operation_receipt_in_tx(
                &mut tx,
                "bulk_tag_characters",
                requested_by,
                character_ids.len() as i64,
                written,
                serde_json::json!({
                    "character_count": character_ids.len(),
                    "tags": tags,
                    "tag_count": tag_ids.len(),
                    "mode": "bulk_manual",
                }),
            )
            .await?;
        tx.commit().await?;
        Ok(receipt)
    }

    pub async fn bulk_request_sheet_exports(
        &self,
        requests: &[NewExportRequest],
        requested_by: &str,
    ) -> AtelierResult<BulkExportRequestResult> {
        let requested_by = require_requester(requested_by)?;
        if requests.is_empty() {
            return Err(AtelierError::Validation(
                "bulk export requires at least one request".into(),
            ));
        }
        for request in requests {
            let row_requested_by = require_requester(&request.requested_by)?;
            if row_requested_by != requested_by {
                return Err(AtelierError::Validation(format!(
                    "bulk export requested_by mismatch: row requested_by={row_requested_by} receipt requested_by={requested_by}"
                )));
            }
        }

        let mut tx = self.pool().begin().await?;
        for request in requests {
            let owner: Option<Uuid> = sqlx::query_scalar(
                "SELECT character_internal_id FROM atelier_sheet_version WHERE version_id = $1",
            )
            .bind(request.sheet_version_id)
            .fetch_optional(&mut *tx)
            .await?;
            match owner {
                None => {
                    return Err(AtelierError::NotFound(format!(
                        "sheet version_id={}",
                        request.sheet_version_id
                    )));
                }
                Some(owner_id) if owner_id != request.character_internal_id => {
                    return Err(AtelierError::Validation(format!(
                        "sheet version {} does not belong to character {}",
                        request.sheet_version_id, request.character_internal_id
                    )));
                }
                Some(_) => {}
            }
        }

        let mut exports = Vec::with_capacity(requests.len());
        for request in requests {
            let row = sqlx::query(
                r#"INSERT INTO atelier_export_request
                     (character_internal_id, sheet_version_id, format, status, label, requested_by)
                   VALUES ($1, $2, $3, 'pending', $4, $5)
                   RETURNING export_id, character_internal_id, sheet_version_id, format,
                             status, label, requested_by, created_at_utc"#,
            )
            .bind(request.character_internal_id)
            .bind(request.sheet_version_id)
            .bind(request.format.as_token())
            .bind(&request.label)
            .bind(&request.requested_by)
            .fetch_one(&mut *tx)
            .await?;
            let export = export_request_from_row(&row)?;
            self.record_event_in_tx(
                &mut tx,
                EXPORT_REQUESTED,
                "atelier_export_request",
                &export.export_id.to_string(),
                serde_json::json!({
                    "sheet_version_id": export.sheet_version_id,
                    "format": export.format.as_token(),
                    "requested_by": export.requested_by,
                    "bulk": true,
                }),
            )
            .await?;
            exports.push(export);
        }

        let receipt = self
            .record_bulk_operation_receipt_in_tx(
                &mut tx,
                "bulk_request_sheet_exports",
                requested_by,
                requests.len() as i64,
                exports.len() as i64,
                serde_json::json!({
                    "export_count": exports.len(),
                    "exports": exports
                        .iter()
                        .map(|export| serde_json::json!({
                            "export_id": export.export_id,
                            "sheet_version_id": export.sheet_version_id,
                            "format": export.format.as_token(),
                            "label": &export.label,
                            "requested_by": &export.requested_by,
                        }))
                        .collect::<Vec<_>>(),
                }),
            )
            .await?;
        tx.commit().await?;
        Ok(BulkExportRequestResult { receipt, exports })
    }

    pub async fn bulk_trash_media_assets(
        &self,
        request: &BulkTrashMediaRequest,
    ) -> AtelierResult<BulkOperationReceipt> {
        let requested_by = require_requester(&request.requested_by)?;
        let reason = request.reason.trim();
        if reason.is_empty() {
            return Err(AtelierError::Validation(
                "bulk trash reason must not be empty".into(),
            ));
        }
        let asset_ids = dedup_uuids(&request.asset_ids);

        let mut tx = self.pool().begin().await?;
        require_all_media_assets_exist(&mut tx, &asset_ids).await?;

        let mut written: i64 = 0;
        for asset_id in &asset_ids {
            let result = sqlx::query(
                r#"INSERT INTO atelier_trash_marker
                     (target_type, target_id, reason, requested_by)
                   VALUES ('media_asset', $1, $2, $3)
                   ON CONFLICT (target_type, target_id)
                   DO UPDATE SET reason = EXCLUDED.reason,
                                 requested_by = EXCLUDED.requested_by,
                                 created_at_utc = NOW()"#,
            )
            .bind(asset_id)
            .bind(reason)
            .bind(requested_by)
            .execute(&mut *tx)
            .await?;
            written += result.rows_affected() as i64;
        }

        let receipt = self
            .record_bulk_operation_receipt_in_tx(
                &mut tx,
                "bulk_trash_media_assets",
                requested_by,
                asset_ids.len() as i64,
                written,
                serde_json::json!({
                    "asset_ids": asset_ids,
                    "reason": reason,
                }),
            )
            .await?;
        tx.commit().await?;
        Ok(receipt)
    }

    pub async fn preview_deletion_impact(
        &self,
        request: &DeletionImpactPreviewRequest,
    ) -> AtelierResult<DeletionImpactPreview> {
        let requested_by = require_requester(&request.requested_by)?.to_string();
        let reason = require_reason(&request.reason)?.to_string();
        let targets = dedup_deletion_targets(&request.targets);
        let mut tx = self.pool().begin().await?;
        let states = collect_deletion_target_states(&mut tx, &targets).await?;
        tx.rollback().await?;
        let already_archived_count = states
            .iter()
            .filter(|target| target.currently_archived)
            .count() as i64;
        let would_archive_count =
            states.iter().filter(|target| target.would_archive).count() as i64;
        Ok(DeletionImpactPreview {
            requested_by,
            reason,
            target_count: states.len() as i64,
            would_archive_count,
            already_archived_count,
            targets: states,
        })
    }

    pub async fn archive_deletion_targets(
        &self,
        request: &DeletionArchiveRequest,
    ) -> AtelierResult<BulkOperationReceipt> {
        let requested_by = require_requester(&request.requested_by)?;
        let reason = require_reason(&request.reason)?;
        let targets = dedup_deletion_targets(&request.targets);
        let mut tx = self.pool().begin().await?;
        let states = collect_deletion_target_states(&mut tx, &targets).await?;

        let mut written: i64 = 0;
        for target in &targets {
            let result = sqlx::query(
                r#"INSERT INTO atelier_trash_marker
                     (target_type, target_id, reason, requested_by)
                   VALUES ($1, $2, $3, $4)
                   ON CONFLICT (target_type, target_id)
                   DO UPDATE SET reason = EXCLUDED.reason,
                                 requested_by = EXCLUDED.requested_by,
                                 created_at_utc = NOW()"#,
            )
            .bind(target.target_type.as_token())
            .bind(target.target_id)
            .bind(reason)
            .bind(requested_by)
            .execute(&mut *tx)
            .await?;
            written += result.rows_affected() as i64;
        }

        let receipt = self
            .record_bulk_operation_receipt_in_tx(
                &mut tx,
                "archive_deletion_targets",
                requested_by,
                targets.len() as i64,
                written,
                serde_json::json!({
                    "reason": reason,
                    "targets": states
                        .iter()
                        .map(|target| serde_json::json!({
                            "target_type": target.target_type.as_token(),
                            "target_id": target.target_id,
                            "previously_archived": target.currently_archived,
                            "would_archive": target.would_archive,
                        }))
                        .collect::<Vec<_>>(),
                }),
            )
            .await?;
        tx.commit().await?;
        Ok(receipt)
    }

    pub async fn restore_deletion_targets(
        &self,
        request: &DeletionRestoreRequest,
    ) -> AtelierResult<BulkOperationReceipt> {
        let requested_by = require_requester(&request.requested_by)?;
        let reason = require_reason(&request.reason)?;
        let targets = dedup_deletion_targets(&request.targets);
        let mut tx = self.pool().begin().await?;
        let states = collect_deletion_target_states(&mut tx, &targets).await?;

        let mut written: i64 = 0;
        for target in &targets {
            let result = sqlx::query(
                r#"DELETE FROM atelier_trash_marker
                   WHERE target_type = $1 AND target_id = $2"#,
            )
            .bind(target.target_type.as_token())
            .bind(target.target_id)
            .execute(&mut *tx)
            .await?;
            written += result.rows_affected() as i64;
        }

        let receipt = self
            .record_bulk_operation_receipt_in_tx(
                &mut tx,
                "restore_deletion_targets",
                requested_by,
                targets.len() as i64,
                written,
                serde_json::json!({
                    "reason": reason,
                    "targets": states
                        .iter()
                        .map(|target| serde_json::json!({
                            "target_type": target.target_type.as_token(),
                            "target_id": target.target_id,
                            "previously_archived": target.currently_archived,
                        }))
                        .collect::<Vec<_>>(),
                }),
            )
            .await?;
        tx.commit().await?;
        Ok(receipt)
    }

    pub async fn is_media_asset_trashed(&self, asset_id: Uuid) -> AtelierResult<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                   SELECT 1 FROM atelier_trash_marker
                   WHERE target_type = 'media_asset' AND target_id = $1
               )"#,
        )
        .bind(asset_id)
        .fetch_one(self.pool())
        .await?;
        Ok(exists)
    }

    pub async fn is_sheet_version_trashed(&self, version_id: Uuid) -> AtelierResult<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                   SELECT 1 FROM atelier_trash_marker
                   WHERE target_type = 'sheet_version' AND target_id = $1
               )"#,
        )
        .bind(version_id)
        .fetch_one(self.pool())
        .await?;
        Ok(exists)
    }
}

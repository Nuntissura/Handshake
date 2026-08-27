//! Bulk atelier operations (MT-014).
//!
//! Bulk mutations validate the complete target set before writing, then
//! commit all target changes of one kind plus their event atomically. The
//! durable receipt and its event follow in their own atomic statement, so a
//! receipt can never describe writes that did not land.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::exports::{
    ExportFormat, ExportRequest, ExportStatus, NewExportRequest, EXPORT_REQUESTED,
};
use super::search::{normalize_tag, search_event_family};
use super::{atelier_event_sql, event_family, AtelierError, AtelierResult, AtelierStore};

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

/// One `atelier_bulk_operation_receipt` row as the store returns it.
#[derive(SurrealValue)]
struct BulkReceiptRow {
    receipt_id: SurrealUuid,
    operation: String,
    requested_by: String,
    target_count: i64,
    mutation_count: i64,
    status: String,
    payload: serde_json::Value,
    created_at_utc: Datetime,
}

impl From<BulkReceiptRow> for BulkOperationReceipt {
    fn from(row: BulkReceiptRow) -> Self {
        BulkOperationReceipt {
            receipt_id: row.receipt_id.into(),
            operation: row.operation,
            requested_by: row.requested_by,
            target_count: row.target_count,
            mutation_count: row.mutation_count,
            status: row.status,
            payload: row.payload,
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

/// One `atelier_export_request` row as the store returns it, links projected.
#[derive(SurrealValue)]
struct ExportRequestRow {
    export_id: SurrealUuid,
    character_internal_id: SurrealUuid,
    sheet_version_id: SurrealUuid,
    format: String,
    status: String,
    label: Option<String>,
    requested_by: String,
    created_at_utc: Datetime,
}

impl TryFrom<ExportRequestRow> for ExportRequest {
    type Error = AtelierError;

    fn try_from(row: ExportRequestRow) -> AtelierResult<Self> {
        Ok(ExportRequest {
            export_id: row.export_id.into(),
            character_internal_id: row.character_internal_id.into(),
            sheet_version_id: row.sheet_version_id.into(),
            format: ExportFormat::from_token(&row.format)?,
            status: ExportStatus::from_token(&row.status)?,
            label: row.label,
            requested_by: row.requested_by,
            created_at_utc: row.created_at_utc.into(),
        })
    }
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

/// Deterministic trash-marker id for a `(target_type, target_id)` pair. The
/// stable UUID replaces the former `ON CONFLICT (target_type, target_id)` upsert
/// key: the same pair always maps to the same record, so archive replays
/// update in place and the unique index stays the last line of defence.
pub(crate) fn trash_marker_uuid(target_type: &str, target_id: Uuid) -> Uuid {
    let digest =
        Sha256::digest(format!("atelier_trash_marker:{target_type}:{target_id}").as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[derive(SurrealValue)]
struct UuidSetBinding {
    ids: Vec<SurrealUuid>,
}

const EXISTING_CHARACTERS_STATEMENT: &str =
    "SELECT VALUE internal_id FROM atelier_character WHERE internal_id IN $ids;";

const EXISTING_MEDIA_ASSETS_STATEMENT: &str =
    "SELECT VALUE asset_id FROM atelier_media_asset WHERE asset_id IN $ids;";

#[derive(SurrealValue)]
struct EnsureTagBindings {
    tag_rid: RecordId,
    tag_id: SurrealUuid,
    text: String,
}

/// Create-if-absent by unique tag text, returning the canonical tag id, in
/// one atomic statement (the former `ON CONFLICT (text) DO UPDATE`).
const ENSURE_TAG_STATEMENT: &str = "RETURN { \
       LET $existing = (SELECT VALUE tag_id FROM atelier_tag WHERE text = $text LIMIT 1); \
       IF $existing = [] { \
         CREATE $tag_rid CONTENT { tag_id: $tag_id, text: $text }; \
       }; \
       RETURN (SELECT VALUE tag_id FROM atelier_tag WHERE text = $text LIMIT 1); };";

/// One character/tag pair travelling to [`BULK_TAG_STATEMENT`].
#[derive(Clone, SurrealValue)]
struct CharacterTagPair {
    character_ref: RecordId,
    tag_ref: RecordId,
    pair_key: Vec<SurrealUuid>,
}

#[derive(Clone, SurrealValue)]
struct BulkTagBindings {
    pairs: Vec<CharacterTagPair>,
}

/// Upsert every character/tag link and append the CHARACTER_TAGGED event in
/// one atomic statement. The link record id is the `[character, tag]` uuid
/// pair, which is what makes the replay an update instead of a duplicate.
const BULK_TAG_STATEMENT: &str = concat!(
    "RETURN { ",
    atelier_event_sql!(),
    " FOR $pair IN $domain.pairs { \
         UPSERT type::record('atelier_character_tag', $pair.pair_key) SET \
           character_internal_id = $pair.character_ref, \
           tag_id = $pair.tag_ref, \
           tag_type = 'manual'; \
       }; \
       RETURN array::len($domain.pairs); };"
);

#[derive(Clone, SurrealValue)]
struct RecordReceiptBindings {
    receipt_rid: RecordId,
    receipt_id: SurrealUuid,
    operation: String,
    requested_by: String,
    target_count: i64,
    mutation_count: i64,
    payload: serde_json::Value,
}

const RECORD_RECEIPT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.receipt_rid; ",
    atelier_event_sql!(),
    " RETURN (CREATE $rid CONTENT { \
         receipt_id: $domain.receipt_id, \
         operation: $domain.operation, \
         requested_by: $domain.requested_by, \
         target_count: $domain.target_count, \
         mutation_count: $domain.mutation_count, \
         status: 'applied', \
         payload: $domain.payload \
       }); };"
);

#[derive(SurrealValue)]
struct ReceiptIdBinding {
    receipt_id: SurrealUuid,
}

const GET_RECEIPT_STATEMENT: &str =
    "SELECT receipt_id, operation, requested_by, target_count, mutation_count, status, \
            payload, created_at_utc \
     FROM atelier_bulk_operation_receipt WHERE receipt_id = $receipt_id LIMIT 1;";

#[derive(SurrealValue)]
struct SheetVersionIdBinding {
    version_id: SurrealUuid,
}

const SHEET_VERSION_OWNER_STATEMENT: &str =
    "SELECT VALUE record::id(character_internal_id) FROM atelier_sheet_version \
     WHERE version_id = $version_id LIMIT 1;";

#[derive(Clone, SurrealValue)]
struct CreateExportRequestBindings {
    export_rid: RecordId,
    export_id: SurrealUuid,
    character_ref: RecordId,
    sheet_version_ref: RecordId,
    format: String,
    label: Option<String>,
    requested_by: String,
}

const CREATE_BULK_EXPORT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.export_rid; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         export_id: $domain.export_id, \
         character_internal_id: $domain.character_ref, \
         sheet_version_id: $domain.sheet_version_ref, \
         format: $domain.format, \
         status: 'pending', \
         label: $domain.label, \
         requested_by: $domain.requested_by \
       }; \
       RETURN (SELECT export_id, record::id(character_internal_id) AS character_internal_id, \
                      record::id(sheet_version_id) AS sheet_version_id, format, status, \
                      label, requested_by, created_at_utc \
               FROM $rid); };"
);

/// One trash-marker upsert travelling to [`ARCHIVE_TARGETS_STATEMENT`].
#[derive(Clone, SurrealValue)]
struct TrashMarkerInsert {
    marker_id: SurrealUuid,
    target_type: String,
    target_id: SurrealUuid,
}

#[derive(Clone, SurrealValue)]
struct ArchiveTargetsBindings {
    markers: Vec<TrashMarkerInsert>,
    reason: String,
    requested_by: String,
}

/// Upsert every trash marker in one atomic statement; a replay refreshes
/// reason/requester/timestamp exactly like the former conflict update.
const ARCHIVE_TARGETS_STATEMENT: &str = "RETURN { \
       FOR $m IN $markers { \
         UPSERT type::record('atelier_trash_marker', $m.marker_id) SET \
           marker_id = $m.marker_id, \
           target_type = $m.target_type, \
           target_id = $m.target_id, \
           reason = $reason, \
           requested_by = $requested_by, \
           created_at_utc = time::now(); \
       }; \
       RETURN array::len($markers); };";

#[derive(SurrealValue)]
struct RestoreTargetsBindings {
    marker_ids: Vec<SurrealUuid>,
}

/// Delete the named markers atomically, reporting how many actually existed
/// (the former `rows_affected` sum).
const RESTORE_TARGETS_STATEMENT: &str = "RETURN { \
       LET $existing = count(SELECT id FROM atelier_trash_marker \
                             WHERE marker_id IN $marker_ids); \
       DELETE atelier_trash_marker WHERE marker_id IN $marker_ids; \
       RETURN $existing; };";

#[derive(SurrealValue)]
struct TrashMarkerLookupBindings {
    target_type: String,
    target_id: SurrealUuid,
}

const TRASH_MARKER_EXISTS_STATEMENT: &str = "RETURN count(SELECT id FROM atelier_trash_marker \
                  WHERE target_type = $target_type AND target_id = $target_id) > 0;";

#[derive(SurrealValue)]
struct TargetExistsBinding {
    target_id: SurrealUuid,
}

const MEDIA_ASSET_EXISTS_STATEMENT: &str =
    "RETURN record::exists(type::record('atelier_media_asset', $target_id));";

const SHEET_VERSION_EXISTS_STATEMENT: &str =
    "RETURN record::exists(type::record('atelier_sheet_version', $target_id));";

impl AtelierStore {
    async fn require_all_characters_exist(&self, ids: &[Uuid]) -> AtelierResult<()> {
        if ids.is_empty() {
            return Err(AtelierError::Validation(
                "bulk operation requires at least one character target".into(),
            ));
        }
        let bindings = UuidSetBinding {
            ids: ids.iter().copied().map(SurrealUuid::from).collect(),
        };
        let existing: Vec<SurrealUuid> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(EXISTING_CHARACTERS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        if existing.len() == ids.len() {
            return Ok(());
        }
        let existing: HashSet<Uuid> = existing.into_iter().map(Into::into).collect();
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

    async fn require_all_media_assets_exist(&self, ids: &[Uuid]) -> AtelierResult<()> {
        if ids.is_empty() {
            return Err(AtelierError::Validation(
                "bulk trash requires at least one media asset target".into(),
            ));
        }
        let bindings = UuidSetBinding {
            ids: ids.iter().copied().map(SurrealUuid::from).collect(),
        };
        let existing: Vec<SurrealUuid> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(EXISTING_MEDIA_ASSETS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        if existing.len() == ids.len() {
            return Ok(());
        }
        let existing: HashSet<Uuid> = existing.into_iter().map(Into::into).collect();
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
        &self,
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
            let statement = match target.target_type {
                DeletionTargetKind::MediaAsset => MEDIA_ASSET_EXISTS_STATEMENT,
                DeletionTargetKind::SheetVersion => SHEET_VERSION_EXISTS_STATEMENT,
            };
            let bindings = TargetExistsBinding {
                target_id: SurrealUuid::from(target.target_id),
            };
            let exists: Option<bool> = self
                .store()
                .with_data_operation(move |ctx| {
                    Box::pin(async move { ctx.query_first(statement, bindings).await })
                })
                .await?;
            if !exists.unwrap_or(false) {
                return Err(AtelierError::NotFound(format!(
                    "{target_type} target_id={}",
                    target.target_id
                )));
            }

            let bindings = TrashMarkerLookupBindings {
                target_type: target_type.to_owned(),
                target_id: SurrealUuid::from(target.target_id),
            };
            let currently_archived: Option<bool> = self
                .store()
                .with_data_operation(move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(TRASH_MARKER_EXISTS_STATEMENT, bindings)
                            .await
                    })
                })
                .await?;
            let currently_archived = currently_archived.unwrap_or(false);

            states.push(DeletionImpactTarget {
                target_type: target.target_type,
                target_id: target.target_id,
                currently_archived,
                would_archive: !currently_archived,
            });
        }

        Ok(states)
    }

    /// Record the durable receipt for one applied bulk operation and its
    /// `BULK_OPERATION_APPLIED` event in one atomic statement.
    pub(crate) async fn record_bulk_operation_receipt(
        &self,
        operation: &str,
        requested_by: &str,
        target_count: i64,
        mutation_count: i64,
        payload: serde_json::Value,
    ) -> AtelierResult<BulkOperationReceipt> {
        let receipt_id = Uuid::now_v7();
        let bindings = RecordReceiptBindings {
            receipt_rid: RecordId::new(
                "atelier_bulk_operation_receipt",
                SurrealUuid::from(receipt_id),
            ),
            receipt_id: SurrealUuid::from(receipt_id),
            operation: operation.to_owned(),
            requested_by: requested_by.to_owned(),
            target_count,
            mutation_count,
            payload: payload.clone(),
        };
        let row: Option<BulkReceiptRow> = self
            .write_with_event(
                RECORD_RECEIPT_STATEMENT,
                bindings,
                event_family::BULK_OPERATION_APPLIED,
                "atelier_bulk_operation_receipt",
                &receipt_id.to_string(),
                serde_json::json!({
                    "receipt_id": receipt_id,
                    "operation": operation,
                    "requested_by": requested_by,
                    "target_count": target_count,
                    "mutation_count": mutation_count,
                    "status": "applied",
                    "receipt_payload": payload,
                }),
            )
            .await?;
        Ok(row
            .ok_or_else(|| {
                AtelierError::Internal(
                    "recording a bulk operation receipt returned no row".to_owned(),
                )
            })?
            .into())
    }

    pub async fn get_bulk_operation_receipt(
        &self,
        receipt_id: Uuid,
    ) -> AtelierResult<BulkOperationReceipt> {
        let bindings = ReceiptIdBinding {
            receipt_id: SurrealUuid::from(receipt_id),
        };
        let row: Option<BulkReceiptRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_RECEIPT_STATEMENT, bindings).await })
            })
            .await?;
        row.map(BulkOperationReceipt::from)
            .ok_or_else(|| AtelierError::NotFound(format!("bulk receipt_id={receipt_id}")))
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

        self.require_all_characters_exist(&character_ids).await?;

        let mut tag_ids = Vec::with_capacity(tags.len());
        for tag in &tags {
            let candidate_id = Uuid::now_v7();
            let bindings = EnsureTagBindings {
                tag_rid: RecordId::new("atelier_tag", SurrealUuid::from(candidate_id)),
                tag_id: SurrealUuid::from(candidate_id),
                text: tag.clone(),
            };
            let tag_id: Option<SurrealUuid> = self
                .store()
                .with_data_operation(move |ctx| {
                    Box::pin(async move { ctx.query_first(ENSURE_TAG_STATEMENT, bindings).await })
                })
                .await?;
            let tag_id: Uuid = tag_id
                .ok_or_else(|| {
                    AtelierError::Internal("ensuring a bulk tag returned no id".to_owned())
                })?
                .into();
            tag_ids.push(tag_id);
        }

        let mut pairs = Vec::with_capacity(character_ids.len() * tag_ids.len());
        for character_id in &character_ids {
            for tag_id in &tag_ids {
                pairs.push(CharacterTagPair {
                    character_ref: RecordId::new(
                        "atelier_character",
                        SurrealUuid::from(*character_id),
                    ),
                    tag_ref: RecordId::new("atelier_tag", SurrealUuid::from(*tag_id)),
                    pair_key: vec![SurrealUuid::from(*character_id), SurrealUuid::from(*tag_id)],
                });
            }
        }
        let written = pairs.len() as i64;
        let bindings = BulkTagBindings { pairs };
        let applied: Option<i64> = self
            .write_with_event(
                BULK_TAG_STATEMENT,
                bindings,
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
        if applied.is_none() {
            return Err(AtelierError::Internal(
                "bulk tagging returned no result".to_owned(),
            ));
        }

        self.record_bulk_operation_receipt(
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
        .await
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

        for request in requests {
            let bindings = SheetVersionIdBinding {
                version_id: SurrealUuid::from(request.sheet_version_id),
            };
            let owner: Option<SurrealUuid> = self
                .store()
                .with_data_operation(move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(SHEET_VERSION_OWNER_STATEMENT, bindings)
                            .await
                    })
                })
                .await?;
            match owner {
                None => {
                    return Err(AtelierError::NotFound(format!(
                        "sheet version_id={}",
                        request.sheet_version_id
                    )));
                }
                Some(owner_id) if Uuid::from(owner_id) != request.character_internal_id => {
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
            let export_id = Uuid::now_v7();
            let bindings = CreateExportRequestBindings {
                export_rid: RecordId::new("atelier_export_request", SurrealUuid::from(export_id)),
                export_id: SurrealUuid::from(export_id),
                character_ref: RecordId::new(
                    "atelier_character",
                    SurrealUuid::from(request.character_internal_id),
                ),
                sheet_version_ref: RecordId::new(
                    "atelier_sheet_version",
                    SurrealUuid::from(request.sheet_version_id),
                ),
                format: request.format.as_token().to_owned(),
                label: request.label.clone(),
                requested_by: request.requested_by.clone(),
            };
            let row: Option<ExportRequestRow> = self
                .write_with_event(
                    CREATE_BULK_EXPORT_STATEMENT,
                    bindings,
                    EXPORT_REQUESTED,
                    "atelier_export_request",
                    &export_id.to_string(),
                    serde_json::json!({
                        "sheet_version_id": request.sheet_version_id,
                        "format": request.format.as_token(),
                        "requested_by": request.requested_by,
                        "bulk": true,
                    }),
                )
                .await?;
            let export: ExportRequest = row
                .ok_or_else(|| {
                    AtelierError::Internal(
                        "creating a bulk export request returned no row".to_owned(),
                    )
                })?
                .try_into()?;
            exports.push(export);
        }

        let receipt = self
            .record_bulk_operation_receipt(
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

        self.require_all_media_assets_exist(&asset_ids).await?;

        let markers: Vec<TrashMarkerInsert> = asset_ids
            .iter()
            .map(|asset_id| TrashMarkerInsert {
                marker_id: SurrealUuid::from(trash_marker_uuid("media_asset", *asset_id)),
                target_type: "media_asset".to_owned(),
                target_id: SurrealUuid::from(*asset_id),
            })
            .collect();
        let bindings = ArchiveTargetsBindings {
            markers,
            reason: reason.to_owned(),
            requested_by: requested_by.to_owned(),
        };
        let written: Option<i64> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(ARCHIVE_TARGETS_STATEMENT, bindings).await })
            })
            .await?;
        let written = written.ok_or_else(|| {
            AtelierError::Internal("bulk trashing media assets returned no result".to_owned())
        })?;

        self.record_bulk_operation_receipt(
            "bulk_trash_media_assets",
            requested_by,
            asset_ids.len() as i64,
            written,
            serde_json::json!({
                "asset_ids": asset_ids,
                "reason": reason,
            }),
        )
        .await
    }

    pub async fn preview_deletion_impact(
        &self,
        request: &DeletionImpactPreviewRequest,
    ) -> AtelierResult<DeletionImpactPreview> {
        let requested_by = require_requester(&request.requested_by)?.to_string();
        let reason = require_reason(&request.reason)?.to_string();
        let targets = dedup_deletion_targets(&request.targets);
        let states = self.collect_deletion_target_states(&targets).await?;
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
        let states = self.collect_deletion_target_states(&targets).await?;

        let markers: Vec<TrashMarkerInsert> = targets
            .iter()
            .map(|target| TrashMarkerInsert {
                marker_id: SurrealUuid::from(trash_marker_uuid(
                    target.target_type.as_token(),
                    target.target_id,
                )),
                target_type: target.target_type.as_token().to_owned(),
                target_id: SurrealUuid::from(target.target_id),
            })
            .collect();
        let bindings = ArchiveTargetsBindings {
            markers,
            reason: reason.to_owned(),
            requested_by: requested_by.to_owned(),
        };
        let written: Option<i64> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(ARCHIVE_TARGETS_STATEMENT, bindings).await })
            })
            .await?;
        let written = written.ok_or_else(|| {
            AtelierError::Internal("archiving deletion targets returned no result".to_owned())
        })?;

        self.record_bulk_operation_receipt(
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
        .await
    }

    pub async fn restore_deletion_targets(
        &self,
        request: &DeletionRestoreRequest,
    ) -> AtelierResult<BulkOperationReceipt> {
        let requested_by = require_requester(&request.requested_by)?;
        let reason = require_reason(&request.reason)?;
        let targets = dedup_deletion_targets(&request.targets);
        let states = self.collect_deletion_target_states(&targets).await?;

        let marker_ids: Vec<SurrealUuid> = targets
            .iter()
            .map(|target| {
                SurrealUuid::from(trash_marker_uuid(
                    target.target_type.as_token(),
                    target.target_id,
                ))
            })
            .collect();
        let bindings = RestoreTargetsBindings { marker_ids };
        let written: Option<i64> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(RESTORE_TARGETS_STATEMENT, bindings).await })
            })
            .await?;
        let written = written.ok_or_else(|| {
            AtelierError::Internal("restoring deletion targets returned no result".to_owned())
        })?;

        self.record_bulk_operation_receipt(
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
        .await
    }

    pub async fn is_media_asset_trashed(&self, asset_id: Uuid) -> AtelierResult<bool> {
        let bindings = TrashMarkerLookupBindings {
            target_type: "media_asset".to_owned(),
            target_id: SurrealUuid::from(asset_id),
        };
        let exists: Option<bool> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(TRASH_MARKER_EXISTS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(exists.unwrap_or(false))
    }

    pub async fn is_sheet_version_trashed(&self, version_id: Uuid) -> AtelierResult<bool> {
        let bindings = TrashMarkerLookupBindings {
            target_type: "sheet_version".to_owned(),
            target_id: SurrealUuid::from(version_id),
        };
        let exists: Option<bool> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(TRASH_MARKER_EXISTS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(exists.unwrap_or(false))
    }
}

//! Embedded SurrealDB implementation of the original Loom storage surface.
//!
//! The functions in this module deliberately accept the lease-bound
//! [`SurrealDataContext`]. `SurrealDatabase` remains responsible for running the
//! write guard and passes the resulting [`MutationMetadata`] into mutating
//! functions. Multi-record invariants are kept in one SurrealQL transaction.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde_json::{json, Value as JsonValue};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{event_ledger, SurrealDataContext, SurrealStorageError};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::{
    Asset, LoomBacklink, LoomBlock, LoomBlockContentType, LoomBlockDerived,
    LoomBlockMutationReceipt, LoomBlockSearchResult, LoomBlockUpdate, LoomCollection,
    LoomCollectionMember, LoomCollectionWithMembers, LoomEdge, LoomEdgeCreatedBy, LoomEdgeType,
    LoomFolder, LoomFolderSortMode, LoomFolderUpdate, LoomGraph, LoomGraphEdge, LoomGraphNode,
    LoomGraphSearchResult, LoomMutationEventReceipt, LoomSearchFilters, LoomSearchResultKind,
    LoomSearchSourceKind, LoomSourceAnchor, LoomTagHub, LoomUnlinkedMention, LoomViewFilters,
    LoomViewGroup, LoomViewResponse, LoomViewType, MediaAssetTier, MediaTier, MediaTierStatus,
    MediaTierUpsert, MutationMetadata, NewAsset, NewLoomBlock, NewLoomEdge, NewLoomFolder,
    PreviewStatus, StorageError, StorageResult,
};

const ASSETS_TABLE: &str = "assets";
const BLOCKS_TABLE: &str = "loom_blocks";
const EDGES_TABLE: &str = "loom_edges";
const COLLECTIONS_TABLE: &str = "loom_collections";

/// The embedded database is single-process. Serializing Loom read-decide-write
/// paths prevents unique-index losers and metric lost updates while retaining
/// the relational backend's transaction semantics.
static LOOM_MUTATION_LOCK: Mutex<()> = Mutex::const_new(());

fn map_err(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

fn guarded_err(error: SurrealStorageError) -> StorageError {
    let rendered = error.to_string();
    if rendered.contains("HSK-LOOM-NOT-FOUND") {
        StorageError::NotFound("loom_block")
    } else if rendered.contains("HSK-LOOM-FOLDER-NOT-FOUND") {
        StorageError::NotFound("loom_folder")
    } else if rendered.contains("HSK-LOOM-COLLECTION-NOT-FOUND") {
        StorageError::NotFound("loom_collection")
    } else if rendered.contains("HSK-LOOM-EDGE-NOT-FOUND") {
        StorageError::NotFound("loom_edge")
    } else if rendered.contains("HSK-LOOM-STALE") {
        StorageError::Conflict("loom_block_stale_updated_at")
    } else if rendered.contains("HSK-LOOM-WORKSPACE-CONFLICT") {
        StorageError::Conflict("loom_workspace_conflict")
    } else if rendered.contains("HSK-MEDIA-TIER-NOT-FOUND") {
        StorageError::NotFound("media_asset_tier")
    } else if rendered.contains("uq_loom_folders_sibling_name") {
        StorageError::Conflict("loom_folder_sibling_name")
    } else {
        StorageError::Database(rendered)
    }
}

fn require_guarded_resource(metadata: &MutationMetadata, resource_id: &str) -> StorageResult<()> {
    if metadata.resource_id == resource_id {
        Ok(())
    } else {
        Err(StorageError::Guard("guarded resource id mismatch"))
    }
}

fn thing(table: &str, id: impl Into<String>) -> RecordId {
    RecordId::new(table, id.into())
}

fn record_key(record: RecordId, table: &'static str) -> StorageResult<String> {
    if record.table.as_str() != table {
        return Err(StorageError::Serialization(format!(
            "expected {table} record link, got {}",
            record.table.as_str()
        )));
    }
    match record.key {
        RecordIdKey::String(id) => Ok(id),
        _ => Err(StorageError::Serialization(format!(
            "{table} record link is not a string key"
        ))),
    }
}

fn opt_record_key(record: Option<RecordId>, table: &'static str) -> StorageResult<Option<String>> {
    record.map(|record| record_key(record, table)).transpose()
}

#[derive(SurrealValue)]
struct WorkspaceBinding {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct WorkspaceRecordBinding {
    workspace: RecordId,
    record: RecordId,
}

#[derive(SurrealValue)]
struct AssetHashBinding {
    workspace: RecordId,
    content_hash: String,
}

#[derive(SurrealValue)]
struct AssetContent {
    asset_id: String,
    workspace_id: RecordId,
    kind: String,
    mime: String,
    original_filename: Option<String>,
    content_hash: String,
    size_bytes: i64,
    width: Option<i64>,
    height: Option<i64>,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    created_at: Datetime,
    classification: String,
    exportable: bool,
    is_proxy_of: Option<String>,
    proxy_asset_id: Option<String>,
}

#[derive(SurrealValue)]
struct AssetRow {
    asset_id: String,
    workspace_id: RecordId,
    kind: String,
    mime: String,
    original_filename: Option<String>,
    content_hash: String,
    size_bytes: i64,
    width: Option<i64>,
    height: Option<i64>,
    created_at: Datetime,
    classification: String,
    exportable: bool,
    is_proxy_of: Option<String>,
    proxy_asset_id: Option<String>,
}

fn asset_to_domain(row: AssetRow) -> StorageResult<Asset> {
    Ok(Asset {
        asset_id: row.asset_id,
        workspace_id: record_key(row.workspace_id, "workspaces")?,
        kind: row.kind,
        mime: row.mime,
        original_filename: row.original_filename,
        content_hash: row.content_hash,
        size_bytes: row.size_bytes,
        width: row.width,
        height: row.height,
        created_at: row.created_at.into_inner(),
        classification: row.classification,
        exportable: row.exportable,
        is_proxy_of: row.is_proxy_of,
        proxy_asset_id: row.proxy_asset_id,
    })
}

pub(crate) async fn create_asset(
    db: &SurrealDataContext<'_>,
    asset_id: String,
    asset: NewAsset,
    metadata: MutationMetadata,
) -> StorageResult<Asset> {
    require_guarded_resource(&metadata, &asset_id)?;
    let content = AssetContent {
        asset_id: asset_id.clone(),
        workspace_id: thing("workspaces", asset.workspace_id),
        kind: asset.kind,
        mime: asset.mime,
        original_filename: asset.original_filename,
        content_hash: asset.content_hash,
        size_bytes: asset.size_bytes,
        width: asset.width,
        height: asset.height,
        last_job_id: metadata.job_id.map(|id| id.to_string()),
        last_workflow_id: metadata.workflow_id.map(|id| id.to_string()),
        last_actor_id: metadata.actor_id,
        edit_event_id: metadata.edit_event_id.to_string(),
        last_actor_kind: metadata.actor_kind.as_str().to_owned(),
        created_at: Datetime::from(metadata.timestamp),
        classification: asset.classification,
        exportable: asset.exportable,
        is_proxy_of: asset.is_proxy_of,
        proxy_asset_id: asset.proxy_asset_id,
    };
    let row = db
        .create_if_absent::<AssetRow, _>(ASSETS_TABLE, &asset_id, content)
        .await
        .map_err(map_err)?
        .ok_or(StorageError::Conflict("asset_id"))?;
    asset_to_domain(row)
}

pub(crate) async fn get_asset(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    asset_id: &str,
) -> StorageResult<Asset> {
    let row = db
        .query_first::<AssetRow, _>(
            "SELECT * FROM $record WHERE workspace_id = $workspace LIMIT 1;",
            WorkspaceRecordBinding {
                workspace: thing("workspaces", workspace_id),
                record: thing(ASSETS_TABLE, asset_id),
            },
        )
        .await
        .map_err(map_err)?
        .ok_or(StorageError::NotFound("asset"))?;
    asset_to_domain(row)
}

pub(crate) async fn find_asset_by_content_hash(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    content_hash: &str,
) -> StorageResult<Option<Asset>> {
    db.query_first::<AssetRow, _>(
        "SELECT * FROM assets WHERE workspace_id = $workspace AND content_hash = $content_hash LIMIT 1;",
        AssetHashBinding {
            workspace: thing("workspaces", workspace_id),
            content_hash: content_hash.to_owned(),
        },
    )
    .await
    .map_err(map_err)?
    .map(asset_to_domain)
    .transpose()
}

#[derive(SurrealValue)]
struct MediaTierRow {
    tier_row_id: String,
    workspace_id: RecordId,
    asset_id: RecordId,
    tier: String,
    status: String,
    tier_asset_id: Option<RecordId>,
    content_hash: Option<String>,
    failure_reason: Option<String>,
    attempt_count: i64,
    created_at: Datetime,
    updated_at: Datetime,
}

fn media_tier_to_domain(row: MediaTierRow) -> StorageResult<MediaAssetTier> {
    Ok(MediaAssetTier {
        tier_row_id: row.tier_row_id,
        workspace_id: record_key(row.workspace_id, "workspaces")?,
        asset_id: record_key(row.asset_id, ASSETS_TABLE)?,
        tier: MediaTier::from_str(&row.tier)?,
        status: MediaTierStatus::from_str(&row.status)?,
        tier_asset_id: opt_record_key(row.tier_asset_id, ASSETS_TABLE)?,
        content_hash: row.content_hash,
        failure_reason: row.failure_reason,
        attempt_count: i32::try_from(row.attempt_count)
            .map_err(|_| StorageError::Serialization("attempt_count exceeds i32".to_owned()))?,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct MediaTierUpsertBinding {
    record: RecordId,
    workspace: RecordId,
    asset: RecordId,
    tier: String,
    status: String,
    tier_asset: Option<RecordId>,
    content_hash: Option<String>,
    failure_reason: Option<String>,
}

#[derive(SurrealValue)]
struct MediaTierKeyBinding {
    workspace: RecordId,
    asset: RecordId,
    tier: String,
}

pub(crate) async fn upsert_media_tier(
    db: &SurrealDataContext<'_>,
    upsert: MediaTierUpsert,
) -> StorageResult<MediaAssetTier> {
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    let row_id = format!("{}--{}", upsert.asset_id, upsert.tier.as_str());
    let result = db
        .query_first::<MediaTierRow, _>(
            "IF (SELECT VALUE workspace_id FROM $record LIMIT 1)[0] != NONE AND (SELECT VALUE workspace_id FROM $record LIMIT 1)[0] != $workspace { THROW 'HSK-LOOM-WORKSPACE-CONFLICT'; } ELSE { RETURN UPSERT $record SET tier_row_id = record::id($record), workspace_id = $workspace, asset_id = $asset, tier = $tier, status = $status, tier_asset_id = $tier_asset, content_hash = $content_hash, failure_reason = $failure_reason, attempt_count = attempt_count ?? 0, updated_at = time::now() RETURN AFTER; };",
            MediaTierUpsertBinding {
                record: thing("media_asset_tiers", row_id),
                workspace: thing("workspaces", upsert.workspace_id),
                asset: thing(ASSETS_TABLE, upsert.asset_id),
                tier: upsert.tier.as_str().to_owned(),
                status: upsert.status.as_str().to_owned(),
                tier_asset: upsert.tier_asset_id.map(|id| thing(ASSETS_TABLE, id)),
                content_hash: upsert.content_hash,
                failure_reason: upsert.failure_reason,
            },
        )
        .await
        .map_err(guarded_err)?
        .ok_or_else(|| StorageError::Database("media tier upsert returned no row".to_owned()))?;
    media_tier_to_domain(result)
}

#[derive(SurrealValue)]
struct MediaTierStatusBinding {
    workspace: RecordId,
    asset: RecordId,
    tier: String,
    status: String,
    failure_reason: Option<String>,
}

pub(crate) async fn set_media_tier_status(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    asset_id: &str,
    tier: MediaTier,
    status: MediaTierStatus,
    failure_reason: Option<String>,
) -> StorageResult<MediaAssetTier> {
    let row = db
        .query_values_at::<MediaTierRow, _>(
            "BEGIN TRANSACTION; LET $rows = UPDATE media_asset_tiers SET attempt_count += IF $status = 'pending' AND status != 'pending' { 1 } ELSE { 0 }, status = $status, failure_reason = $failure_reason, updated_at = time::now() WHERE workspace_id = $workspace AND asset_id = $asset AND tier = $tier RETURN AFTER; IF array::len($rows) = 0 { THROW 'HSK-MEDIA-TIER-NOT-FOUND'; } ELSE { RETURN $rows[0]; }; COMMIT TRANSACTION;",
            MediaTierStatusBinding {
                workspace: thing("workspaces", workspace_id),
                asset: thing(ASSETS_TABLE, asset_id),
                tier: tier.as_str().to_owned(),
                status: status.as_str().to_owned(),
                failure_reason,
            },
            2,
        )
        .await
        .map_err(guarded_err)?
        .into_iter()
        .next()
        .ok_or(StorageError::NotFound("media_asset_tier"))?;
    media_tier_to_domain(row)
}

pub(crate) async fn get_media_tier(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    asset_id: &str,
    tier: MediaTier,
) -> StorageResult<Option<MediaAssetTier>> {
    db.query_first::<MediaTierRow, _>(
        "SELECT * FROM media_asset_tiers WHERE workspace_id = $workspace AND asset_id = $asset AND tier = $tier LIMIT 1;",
        MediaTierKeyBinding {
            workspace: thing("workspaces", workspace_id),
            asset: thing(ASSETS_TABLE, asset_id),
            tier: tier.as_str().to_owned(),
        },
    )
    .await
    .map_err(map_err)?
    .map(media_tier_to_domain)
    .transpose()
}

pub(crate) async fn list_media_tiers(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    asset_id: &str,
) -> StorageResult<Vec<MediaAssetTier>> {
    let rows = db
        .query_values::<MediaTierRow, _>(
            "SELECT * FROM media_asset_tiers WHERE workspace_id = $workspace AND asset_id = $record ORDER BY tier ASC;",
            WorkspaceRecordBinding {
                workspace: thing("workspaces", workspace_id),
                record: thing(ASSETS_TABLE, asset_id),
            },
        )
        .await
        .map_err(map_err)?;
    rows.into_iter().map(media_tier_to_domain).collect()
}

pub(crate) async fn list_failed_media_tiers(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
) -> StorageResult<Vec<MediaAssetTier>> {
    db.query_values::<MediaTierRow, _>(
        "SELECT * FROM media_asset_tiers WHERE workspace_id = $workspace AND status = 'failed' ORDER BY updated_at DESC, tier_row_id ASC;",
        WorkspaceBinding {
            workspace: thing("workspaces", workspace_id),
        },
    )
    .await
    .map_err(map_err)?
    .into_iter()
    .map(media_tier_to_domain)
    .collect()
}

pub(crate) async fn delete_media_tiers(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    asset_id: &str,
) -> StorageResult<u64> {
    let rows = db
        .query_values::<MediaTierRow, _>(
            "DELETE media_asset_tiers WHERE workspace_id = $workspace AND asset_id = $record RETURN BEFORE;",
            WorkspaceRecordBinding {
                workspace: thing("workspaces", workspace_id),
                record: thing(ASSETS_TABLE, asset_id),
            },
        )
        .await
        .map_err(map_err)?;
    Ok(rows.len() as u64)
}

#[derive(SurrealValue)]
struct CollectionRow {
    collection_id: String,
    workspace_id: RecordId,
    title: Option<String>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct CollectionContent {
    collection_id: String,
    workspace_id: RecordId,
    title: Option<String>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct CollectionMemberRow {
    asset_id: RecordId,
    position: i64,
}

#[derive(SurrealValue)]
struct CollectionMemberInput {
    record: RecordId,
    asset: RecordId,
    position: i64,
}

#[derive(SurrealValue)]
struct CollectionOrderBinding {
    collection: RecordId,
    workspace: RecordId,
    members: Vec<CollectionMemberInput>,
}

fn collection_to_domain(row: CollectionRow) -> StorageResult<LoomCollection> {
    Ok(LoomCollection {
        collection_id: row.collection_id,
        workspace_id: record_key(row.workspace_id, "workspaces")?,
        title: row.title,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

pub(crate) async fn create_loom_collection(
    db: &SurrealDataContext<'_>,
    collection_id: String,
    workspace_id: &str,
    title: Option<String>,
    metadata: MutationMetadata,
) -> StorageResult<LoomCollection> {
    require_guarded_resource(&metadata, &collection_id)?;
    let row = db
        .create_if_absent::<CollectionRow, _>(
            COLLECTIONS_TABLE,
            &collection_id,
            CollectionContent {
                collection_id: collection_id.clone(),
                workspace_id: thing("workspaces", workspace_id),
                title,
                created_at: Datetime::from(metadata.timestamp),
                updated_at: Datetime::from(metadata.timestamp),
            },
        )
        .await
        .map_err(map_err)?
        .ok_or(StorageError::Conflict("loom_collection_id"))?;
    collection_to_domain(row)
}

pub(crate) async fn get_loom_collection(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    collection_id: &str,
) -> StorageResult<LoomCollectionWithMembers> {
    let binding = WorkspaceRecordBinding {
        workspace: thing("workspaces", workspace_id),
        record: thing(COLLECTIONS_TABLE, collection_id),
    };
    let row = db
        .query_first::<CollectionRow, _>(
            "SELECT * FROM $record WHERE workspace_id = $workspace LIMIT 1;",
            binding,
        )
        .await
        .map_err(map_err)?
        .ok_or(StorageError::NotFound("loom_collection"))?;
    let members = db
        .query_values::<CollectionMemberRow, _>(
            "SELECT asset_id, position FROM loom_collection_members WHERE collection_id = $record ORDER BY position ASC;",
            WorkspaceRecordBinding {
                workspace: thing("workspaces", workspace_id),
                record: thing(COLLECTIONS_TABLE, collection_id),
            },
        )
        .await
        .map_err(map_err)?
        .into_iter()
        .map(|row| {
            Ok(LoomCollectionMember {
                asset_id: record_key(row.asset_id, ASSETS_TABLE)?,
                position: i32::try_from(row.position).map_err(|_| {
                    StorageError::Serialization("collection position exceeds i32".to_owned())
                })?,
            })
        })
        .collect::<StorageResult<Vec<_>>>()?;
    Ok(LoomCollectionWithMembers {
        collection: collection_to_domain(row)?,
        members,
    })
}

pub(crate) async fn set_loom_collection_order(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    collection_id: &str,
    asset_ids: &[String],
) -> StorageResult<LoomCollectionWithMembers> {
    if asset_ids.iter().collect::<HashSet<_>>().len() != asset_ids.len() {
        return Err(StorageError::Conflict("duplicate loom collection asset"));
    }
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    let collection = thing(COLLECTIONS_TABLE, collection_id);
    let members = asset_ids
        .iter()
        .enumerate()
        .map(|(position, asset_id)| CollectionMemberInput {
            record: thing(
                "loom_collection_members",
                format!("{collection_id}--{asset_id}"),
            ),
            asset: thing(ASSETS_TABLE, asset_id),
            position: position as i64,
        })
        .collect();
    db.execute_returning(
        "BEGIN TRANSACTION; IF (SELECT VALUE id FROM $collection WHERE workspace_id = $workspace LIMIT 1)[0] = NONE { THROW 'HSK-LOOM-COLLECTION-NOT-FOUND'; }; DELETE loom_collection_members WHERE collection_id = $collection; FOR $member IN $members { CREATE $member.record SET collection_id = $collection, asset_id = $member.asset, position = $member.position; }; UPDATE $collection SET updated_at = time::now() RETURN AFTER; COMMIT TRANSACTION;",
        CollectionOrderBinding {
            collection,
            workspace: thing("workspaces", workspace_id),
            members,
        },
    )
    .await
    .map_err(guarded_err)?;
    get_loom_collection(db, workspace_id, collection_id).await
}

#[derive(SurrealValue)]
struct BlockRow {
    block_id: String,
    workspace_id: RecordId,
    content_type: String,
    document_id: Option<RecordId>,
    asset_id: Option<RecordId>,
    title: Option<String>,
    original_filename: Option<String>,
    content_hash: Option<String>,
    pinned: bool,
    favorite: bool,
    pin_order: Option<i64>,
    journal_date: Option<String>,
    created_at: Datetime,
    updated_at: Datetime,
    imported_at: Option<Datetime>,
    backlink_count: i64,
    mention_count: i64,
    tag_count: i64,
    derived_json: JsonValue,
    preview_status: String,
    thumbnail_asset_id: Option<RecordId>,
    proxy_asset_id: Option<RecordId>,
}

fn block_to_domain(row: BlockRow) -> StorageResult<LoomBlock> {
    let mut derived: LoomBlockDerived =
        serde_json::from_value(row.derived_json).unwrap_or_default();
    derived.backlink_count = row.backlink_count;
    derived.mention_count = row.mention_count;
    derived.tag_count = row.tag_count;
    derived.preview_status = PreviewStatus::from_str(&row.preview_status)?;
    derived.thumbnail_asset_id = opt_record_key(row.thumbnail_asset_id, ASSETS_TABLE)?;
    derived.proxy_asset_id = opt_record_key(row.proxy_asset_id, ASSETS_TABLE)?;
    Ok(LoomBlock {
        block_id: row.block_id,
        workspace_id: record_key(row.workspace_id, "workspaces")?,
        content_type: LoomBlockContentType::from_str(&row.content_type)?,
        document_id: opt_record_key(row.document_id, "documents")?,
        asset_id: opt_record_key(row.asset_id, ASSETS_TABLE)?,
        title: row.title,
        original_filename: row.original_filename,
        content_hash: row.content_hash,
        pinned: row.pinned,
        favorite: row.favorite,
        pin_order: row
            .pin_order
            .map(i32::try_from)
            .transpose()
            .map_err(|_| StorageError::Serialization("pin_order exceeds i32".to_owned()))?,
        journal_date: row.journal_date,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
        imported_at: row.imported_at.map(Datetime::into_inner),
        derived,
    })
}

fn loom_search_text(block: &LoomBlock) -> String {
    [
        block.title.as_deref(),
        block.original_filename.as_deref(),
        block.derived.full_text_index.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}

#[derive(SurrealValue)]
struct BlockContent {
    block_id: String,
    workspace_id: RecordId,
    content_type: String,
    document_id: Option<RecordId>,
    asset_id: Option<RecordId>,
    title: Option<String>,
    original_filename: Option<String>,
    content_hash: Option<String>,
    pinned: bool,
    favorite: bool,
    pin_order: Option<i64>,
    journal_date: Option<String>,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    created_at: Datetime,
    updated_at: Datetime,
    imported_at: Option<Datetime>,
    backlink_count: i64,
    mention_count: i64,
    tag_count: i64,
    derived_json: JsonValue,
    preview_status: String,
    thumbnail_asset_id: Option<RecordId>,
    proxy_asset_id: Option<RecordId>,
}

#[derive(SurrealValue)]
struct BlockCreateBinding {
    block: RecordId,
    content: BlockContent,
    search: RecordId,
    workspace: RecordId,
    content_type: String,
    search_text: String,
}

fn block_content(
    id: String,
    block: NewLoomBlock,
    metadata: MutationMetadata,
) -> StorageResult<BlockContent> {
    let derived_json = serde_json::to_value(&block.derived)?;
    Ok(BlockContent {
        block_id: id,
        workspace_id: thing("workspaces", block.workspace_id),
        content_type: block.content_type.as_str().to_owned(),
        document_id: block.document_id.map(|id| thing("documents", id)),
        asset_id: block.asset_id.map(|id| thing(ASSETS_TABLE, id)),
        title: block.title,
        original_filename: block.original_filename,
        content_hash: block.content_hash,
        pinned: block.pinned,
        favorite: false,
        pin_order: None,
        journal_date: block.journal_date,
        last_job_id: metadata.job_id.map(|id| id.to_string()),
        last_workflow_id: metadata.workflow_id.map(|id| id.to_string()),
        last_actor_id: metadata.actor_id,
        edit_event_id: metadata.edit_event_id.to_string(),
        last_actor_kind: metadata.actor_kind.as_str().to_owned(),
        created_at: Datetime::from(metadata.timestamp),
        updated_at: Datetime::from(metadata.timestamp),
        imported_at: block.imported_at.map(Datetime::from),
        backlink_count: block.derived.backlink_count,
        mention_count: block.derived.mention_count,
        tag_count: block.derived.tag_count,
        derived_json,
        preview_status: block.derived.preview_status.as_str().to_owned(),
        thumbnail_asset_id: block
            .derived
            .thumbnail_asset_id
            .map(|id| thing(ASSETS_TABLE, id)),
        proxy_asset_id: block
            .derived
            .proxy_asset_id
            .map(|id| thing(ASSETS_TABLE, id)),
    })
}

pub(crate) async fn create_loom_block(
    db: &SurrealDataContext<'_>,
    block: NewLoomBlock,
    metadata: MutationMetadata,
) -> StorageResult<LoomBlock> {
    let id = block
        .block_id
        .clone()
        .unwrap_or_else(|| Uuid::now_v7().to_string());
    if id.trim().is_empty() || id.trim() != id {
        return Err(StorageError::Validation(
            "loom block_id must be non-empty without surrounding whitespace",
        ));
    }
    require_guarded_resource(&metadata, &id)?;
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    let preview = LoomBlock {
        block_id: id.clone(),
        workspace_id: block.workspace_id.clone(),
        content_type: block.content_type.clone(),
        document_id: block.document_id.clone(),
        asset_id: block.asset_id.clone(),
        title: block.title.clone(),
        original_filename: block.original_filename.clone(),
        content_hash: block.content_hash.clone(),
        pinned: block.pinned,
        favorite: false,
        pin_order: None,
        journal_date: block.journal_date.clone(),
        created_at: metadata.timestamp,
        updated_at: metadata.timestamp,
        imported_at: block.imported_at,
        derived: block.derived.clone(),
    };
    let content = block_content(id.clone(), block, metadata)?;
    let row = db
        .query_values_at::<BlockRow, _>(
            "BEGIN TRANSACTION; CREATE $block CONTENT $content RETURN AFTER; UPSERT $search SET block_id = $block, workspace_id = $workspace, content_type = $content_type, search_text = $search_text, indexed_at = time::now(); COMMIT TRANSACTION;",
            BlockCreateBinding {
                block: thing(BLOCKS_TABLE, id.clone()),
                content,
                search: thing("loom_block_search_index", id),
                workspace: thing("workspaces", preview.workspace_id.clone()),
                content_type: preview.content_type.as_str().to_owned(),
                search_text: loom_search_text(&preview),
            },
            1,
        )
        .await
        .map_err(map_err)?
        .into_iter()
        .next()
        .ok_or_else(|| StorageError::Database("loom block create returned no row".to_owned()))?;
    block_to_domain(row)
}

#[derive(SurrealValue)]
struct JournalBinding {
    workspace: RecordId,
    journal_date: String,
}

pub(crate) async fn get_or_create_daily_journal_block(
    db: &SurrealDataContext<'_>,
    new_block_id: String,
    workspace_id: &str,
    journal_date: &str,
    metadata: MutationMetadata,
) -> StorageResult<LoomBlock> {
    require_guarded_resource(&metadata, &new_block_id)?;
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    if let Some(row) = db
        .query_first::<BlockRow, _>(
            "SELECT * FROM loom_blocks WHERE workspace_id = $workspace AND content_type = 'journal' AND journal_date = $journal_date LIMIT 1;",
            JournalBinding {
                workspace: thing("workspaces", workspace_id),
                journal_date: journal_date.to_owned(),
            },
        )
        .await
        .map_err(map_err)?
    {
        return block_to_domain(row);
    }
    let title = format!("Daily Note {journal_date}");
    let mut derived = LoomBlockDerived::default();
    derived.full_text_index = Some(format!("# {title}\n\n"));
    // Call the unlocked implementation inline because this function owns the mutex.
    let id = new_block_id;
    let block = NewLoomBlock {
        block_id: Some(id.clone()),
        workspace_id: workspace_id.to_owned(),
        content_type: LoomBlockContentType::Journal,
        document_id: None,
        asset_id: None,
        title: Some(title),
        original_filename: None,
        content_hash: None,
        pinned: false,
        journal_date: Some(journal_date.to_owned()),
        imported_at: None,
        derived,
    };
    let preview = LoomBlock {
        block_id: id.clone(),
        workspace_id: workspace_id.to_owned(),
        content_type: block.content_type.clone(),
        document_id: None,
        asset_id: None,
        title: block.title.clone(),
        original_filename: None,
        content_hash: None,
        pinned: false,
        favorite: false,
        pin_order: None,
        journal_date: block.journal_date.clone(),
        created_at: metadata.timestamp,
        updated_at: metadata.timestamp,
        imported_at: None,
        derived: block.derived.clone(),
    };
    let content = block_content(id.clone(), block, metadata)?;
    let row = db
        .query_values_at::<BlockRow, _>(
            "BEGIN TRANSACTION; CREATE $block CONTENT $content RETURN AFTER; UPSERT $search SET block_id = $block, workspace_id = $workspace, content_type = 'journal', search_text = $search_text, indexed_at = time::now(); COMMIT TRANSACTION;",
            BlockCreateBinding {
                block: thing(BLOCKS_TABLE, id.clone()),
                content,
                search: thing("loom_block_search_index", id),
                workspace: thing("workspaces", workspace_id),
                content_type: "journal".to_owned(),
                search_text: loom_search_text(&preview),
            },
            1,
        )
        .await
        .map_err(map_err)?
        .into_iter()
        .next()
        .ok_or_else(|| StorageError::Database("journal create returned no row".to_owned()))?;
    block_to_domain(row)
}

pub(crate) async fn get_loom_block(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<LoomBlock> {
    let row = db
        .query_first::<BlockRow, _>(
            "SELECT * FROM $record WHERE workspace_id = $workspace LIMIT 1;",
            WorkspaceRecordBinding {
                workspace: thing("workspaces", workspace_id),
                record: thing(BLOCKS_TABLE, block_id),
            },
        )
        .await
        .map_err(map_err)?
        .ok_or(StorageError::NotFound("loom_block"))?;
    block_to_domain(row)
}

pub(crate) async fn find_loom_block_by_content_hash(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    content_hash: &str,
) -> StorageResult<Option<LoomBlock>> {
    db.query_first::<BlockRow, _>(
        "SELECT * FROM loom_blocks WHERE workspace_id = $workspace AND content_hash = $content_hash LIMIT 1;",
        AssetHashBinding {
            workspace: thing("workspaces", workspace_id),
            content_hash: content_hash.to_owned(),
        },
    )
    .await
    .map_err(map_err)?
    .map(block_to_domain)
    .transpose()
}

pub(crate) async fn find_loom_block_by_asset_id(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    asset_id: &str,
) -> StorageResult<Option<LoomBlock>> {
    db.query_first::<BlockRow, _>(
        "SELECT * FROM loom_blocks WHERE workspace_id = $workspace AND asset_id = $record ORDER BY updated_at DESC LIMIT 1;",
        WorkspaceRecordBinding {
            workspace: thing("workspaces", workspace_id),
            record: thing(ASSETS_TABLE, asset_id),
        },
    )
    .await
    .map_err(map_err)?
    .map(block_to_domain)
    .transpose()
}

#[derive(SurrealValue)]
struct BlockUpdateBinding {
    block: RecordId,
    workspace: RecordId,
    title: Option<String>,
    pinned: Option<bool>,
    favorite: Option<bool>,
    pin_order: Option<i64>,
    journal_date: Option<String>,
    expected_updated_at: Option<Datetime>,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    updated_at: Datetime,
    search: RecordId,
    search_text: String,
}

pub(crate) async fn update_loom_block(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
    update: LoomBlockUpdate,
    metadata: MutationMetadata,
) -> StorageResult<LoomBlock> {
    require_guarded_resource(&metadata, block_id)?;
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    let existing = get_loom_block(db, workspace_id, block_id).await?;
    let mut projected = existing.clone();
    if let Some(value) = update.title.clone() {
        projected.title = Some(value);
    }
    if let Some(value) = update.pinned {
        projected.pinned = value;
    }
    if let Some(value) = update.favorite {
        projected.favorite = value;
    }
    if let Some(value) = update.pin_order {
        projected.pin_order = Some(value);
    }
    if let Some(value) = update.journal_date.clone() {
        projected.journal_date = Some(value);
    }
    let rows = db
        .query_values_at::<BlockRow, _>(
            "BEGIN TRANSACTION; IF (SELECT VALUE workspace_id FROM $block LIMIT 1)[0] != $workspace { THROW 'HSK-LOOM-NOT-FOUND'; }; IF $expected_updated_at != NONE AND (SELECT VALUE updated_at FROM $block LIMIT 1)[0] != $expected_updated_at { THROW 'HSK-LOOM-STALE'; }; UPDATE $block SET title = IF $title = NONE { title } ELSE { $title }, pinned = IF $pinned = NONE { pinned } ELSE { $pinned }, favorite = IF $favorite = NONE { favorite } ELSE { $favorite }, pin_order = IF $pin_order = NONE { pin_order } ELSE { $pin_order }, journal_date = IF $journal_date = NONE { journal_date } ELSE { $journal_date }, last_job_id = $last_job_id, last_workflow_id = $last_workflow_id, last_actor_id = $last_actor_id, edit_event_id = $edit_event_id, last_actor_kind = $last_actor_kind, updated_at = $updated_at RETURN AFTER; UPDATE $search SET search_text = $search_text, indexed_at = time::now(); COMMIT TRANSACTION;",
            BlockUpdateBinding {
                block: thing(BLOCKS_TABLE, block_id),
                workspace: thing("workspaces", workspace_id),
                title: update.title,
                pinned: update.pinned,
                favorite: update.favorite,
                pin_order: update.pin_order.map(i64::from),
                journal_date: update.journal_date,
                expected_updated_at: update.expected_updated_at.map(Datetime::from),
                last_job_id: metadata.job_id.map(|id| id.to_string()),
                last_workflow_id: metadata.workflow_id.map(|id| id.to_string()),
                last_actor_id: metadata.actor_id,
                edit_event_id: metadata.edit_event_id.to_string(),
                last_actor_kind: metadata.actor_kind.as_str().to_owned(),
                updated_at: Datetime::from(metadata.timestamp),
                search: thing("loom_block_search_index", block_id),
                search_text: loom_search_text(&projected),
            },
            3,
        )
        .await
        .map_err(guarded_err)?;
    block_to_domain(
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("loom_block"))?,
    )
}

#[derive(SurrealValue)]
struct PreviewBinding {
    block: RecordId,
    workspace: RecordId,
    preview_status: String,
    thumbnail: Option<RecordId>,
    proxy: Option<RecordId>,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    updated_at: Datetime,
}

pub(crate) async fn set_loom_block_preview(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
    preview_status: PreviewStatus,
    thumbnail_asset_id: Option<String>,
    proxy_asset_id: Option<String>,
    metadata: MutationMetadata,
) -> StorageResult<()> {
    require_guarded_resource(&metadata, block_id)?;
    let count = db
        .execute_returning(
            "UPDATE $block SET preview_status = $preview_status, thumbnail_asset_id = $thumbnail, proxy_asset_id = $proxy, last_job_id = $last_job_id, last_workflow_id = $last_workflow_id, last_actor_id = $last_actor_id, edit_event_id = $edit_event_id, last_actor_kind = $last_actor_kind, updated_at = $updated_at WHERE workspace_id = $workspace RETURN AFTER;",
            PreviewBinding {
                block: thing(BLOCKS_TABLE, block_id),
                workspace: thing("workspaces", workspace_id),
                preview_status: preview_status.as_str().to_owned(),
                thumbnail: thumbnail_asset_id.map(|id| thing(ASSETS_TABLE, id)),
                proxy: proxy_asset_id.map(|id| thing(ASSETS_TABLE, id)),
                last_job_id: metadata.job_id.map(|id| id.to_string()),
                last_workflow_id: metadata.workflow_id.map(|id| id.to_string()),
                last_actor_id: metadata.actor_id,
                edit_event_id: metadata.edit_event_id.to_string(),
                last_actor_kind: metadata.actor_kind.as_str().to_owned(),
                updated_at: Datetime::from(metadata.timestamp),
            },
        )
        .await
        .map_err(map_err)?;
    if count == 0 {
        Err(StorageError::NotFound("loom_block"))
    } else {
        Ok(())
    }
}

pub(crate) async fn delete_loom_block(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<()> {
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    let affected: Vec<RecordId> = list_loom_edges_for_block(db, workspace_id, block_id)
        .await?
        .into_iter()
        .flat_map(|edge| [edge.source_block_id, edge.target_block_id])
        .filter(|id| id != block_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|id| thing(BLOCKS_TABLE, id))
        .collect();
    db.execute_returning(
        "BEGIN TRANSACTION; IF (SELECT VALUE id FROM $record WHERE workspace_id = $workspace LIMIT 1)[0] = NONE { THROW 'HSK-LOOM-NOT-FOUND'; }; DELETE $record RETURN BEFORE; FOR $block IN $affected { UPDATE $block SET mention_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $block AND edge_type = 'mention')), tag_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $block AND edge_type = 'tag')), backlink_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND target_block_id = $block AND edge_type IN ['mention', 'tag'])); }; COMMIT TRANSACTION;",
        DeleteBlockBinding {
            workspace: thing("workspaces", workspace_id),
            record: thing(BLOCKS_TABLE, block_id),
            affected,
        },
    )
    .await
    .map_err(guarded_err)?;
    Ok(())
}

#[derive(SurrealValue)]
struct DeleteBlockBinding {
    workspace: RecordId,
    record: RecordId,
    affected: Vec<RecordId>,
}

#[derive(SurrealValue)]
struct EdgeRow {
    edge_id: String,
    workspace_id: RecordId,
    source_block_id: RecordId,
    target_block_id: RecordId,
    edge_type: String,
    created_by: String,
    created_at: Datetime,
    crdt_site_id: Option<String>,
    source_document_id: Option<String>,
    source_text_block_id: Option<String>,
    offset_start: Option<i64>,
    offset_end: Option<i64>,
}

fn edge_to_domain(row: EdgeRow) -> StorageResult<LoomEdge> {
    let source_anchor = match (
        row.source_document_id,
        row.source_text_block_id,
        row.offset_start,
        row.offset_end,
    ) {
        (Some(document_id), Some(block_id), Some(offset_start), Some(offset_end)) => {
            Some(LoomSourceAnchor {
                document_id,
                block_id,
                offset_start,
                offset_end,
            })
        }
        // Rich-document backlink projection knows the owning document and text block, but its
        // extraction contract does not claim character offsets. Keep that internal ownership
        // metadata without fabricating a positioned public source anchor.
        (Some(_), Some(_), None, None) => None,
        (None, None, None, None) => None,
        _ => {
            return Err(StorageError::Serialization(
                "partial loom edge source anchor".to_owned(),
            ));
        }
    };
    Ok(LoomEdge {
        edge_id: row.edge_id,
        workspace_id: record_key(row.workspace_id, "workspaces")?,
        source_block_id: record_key(row.source_block_id, BLOCKS_TABLE)?,
        target_block_id: record_key(row.target_block_id, BLOCKS_TABLE)?,
        edge_type: LoomEdgeType::from_str(&row.edge_type)?,
        created_by: LoomEdgeCreatedBy::from_str(&row.created_by)?,
        created_at: row.created_at.into_inner(),
        crdt_site_id: row.crdt_site_id,
        source_anchor,
    })
}

#[derive(SurrealValue)]
struct EdgeContent {
    edge_id: String,
    workspace_id: RecordId,
    source_block_id: RecordId,
    target_block_id: RecordId,
    edge_type: String,
    created_by: String,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    created_at: Datetime,
    crdt_site_id: Option<String>,
    source_document_id: Option<String>,
    source_text_block_id: Option<String>,
    offset_start: Option<i64>,
    offset_end: Option<i64>,
}

#[derive(SurrealValue)]
struct EdgeCreateBinding {
    edge: RecordId,
    content: EdgeContent,
    workspace: RecordId,
    source: RecordId,
    target: RecordId,
}

pub(crate) async fn create_loom_edge(
    db: &SurrealDataContext<'_>,
    edge: NewLoomEdge,
    metadata: MutationMetadata,
) -> StorageResult<LoomEdge> {
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    let id = edge
        .edge_id
        .clone()
        .unwrap_or_else(|| Uuid::now_v7().to_string());
    require_guarded_resource(&metadata, &id)?;
    let (source_document_id, source_text_block_id, offset_start, offset_end) =
        match edge.source_anchor {
            Some(anchor) => (
                Some(anchor.document_id),
                Some(anchor.block_id),
                Some(anchor.offset_start),
                Some(anchor.offset_end),
            ),
            None => (None, None, None, None),
        };
    let source = thing(BLOCKS_TABLE, edge.source_block_id);
    let target = thing(BLOCKS_TABLE, edge.target_block_id);
    let rows = db
        .query_values_at::<EdgeRow, _>(
            "BEGIN TRANSACTION; IF (SELECT VALUE workspace_id FROM $source LIMIT 1)[0] != $workspace OR (SELECT VALUE workspace_id FROM $target LIMIT 1)[0] != $workspace { THROW 'HSK-LOOM-NOT-FOUND'; }; CREATE $edge CONTENT $content RETURN AFTER; UPDATE $source SET mention_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $source AND edge_type = 'mention')), tag_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $source AND edge_type = 'tag')), backlink_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND target_block_id = $source AND edge_type IN ['mention', 'tag'])); UPDATE $target SET mention_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $target AND edge_type = 'mention')), tag_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $target AND edge_type = 'tag')), backlink_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND target_block_id = $target AND edge_type IN ['mention', 'tag'])); COMMIT TRANSACTION;",
            EdgeCreateBinding {
                edge: thing(EDGES_TABLE, id.clone()),
                content: EdgeContent {
                    edge_id: id,
                    workspace_id: thing("workspaces", edge.workspace_id.clone()),
                    source_block_id: source.clone(),
                    target_block_id: target.clone(),
                    edge_type: edge.edge_type.as_str().to_owned(),
                    created_by: edge.created_by.as_str().to_owned(),
                    last_job_id: metadata.job_id.map(|id| id.to_string()),
                    last_workflow_id: metadata.workflow_id.map(|id| id.to_string()),
                    last_actor_id: metadata.actor_id,
                    edit_event_id: metadata.edit_event_id.to_string(),
                    last_actor_kind: metadata.actor_kind.as_str().to_owned(),
                    created_at: Datetime::from(metadata.timestamp),
                    crdt_site_id: edge.crdt_site_id,
                    source_document_id,
                    source_text_block_id,
                    offset_start,
                    offset_end,
                },
                workspace: thing("workspaces", edge.workspace_id),
                source,
                target,
            },
            2,
        )
        .await
        .map_err(guarded_err)?;
    edge_to_domain(
        rows.into_iter()
            .next()
            .ok_or_else(|| StorageError::Database("loom edge create returned no row".to_owned()))?,
    )
}

pub(crate) async fn delete_loom_edge(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    edge_id: &str,
) -> StorageResult<LoomEdge> {
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    let existing = db
        .query_first::<EdgeRow, _>(
            "SELECT * FROM $record WHERE workspace_id = $workspace LIMIT 1;",
            WorkspaceRecordBinding {
                workspace: thing("workspaces", workspace_id),
                record: thing(EDGES_TABLE, edge_id),
            },
        )
        .await
        .map_err(map_err)?
        .ok_or(StorageError::NotFound("loom_edge"))?;
    let mapped = edge_to_domain(existing)?;
    db.execute_returning(
        "BEGIN TRANSACTION; IF (SELECT VALUE workspace_id FROM $record LIMIT 1)[0] != $workspace { THROW 'HSK-LOOM-EDGE-NOT-FOUND'; }; DELETE $record RETURN BEFORE; UPDATE $source SET mention_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $source AND edge_type = 'mention')), tag_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $source AND edge_type = 'tag')), backlink_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND target_block_id = $source AND edge_type IN ['mention', 'tag'])); UPDATE $target SET mention_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $target AND edge_type = 'mention')), tag_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $target AND edge_type = 'tag')), backlink_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND target_block_id = $target AND edge_type IN ['mention', 'tag'])); COMMIT TRANSACTION;",
        DeleteEdgeBinding {
            workspace: thing("workspaces", workspace_id),
            record: thing(EDGES_TABLE, edge_id),
            source: thing(BLOCKS_TABLE, mapped.source_block_id.clone()),
            target: thing(BLOCKS_TABLE, mapped.target_block_id.clone()),
        },
    )
    .await
    .map_err(guarded_err)?;
    Ok(mapped)
}

#[derive(SurrealValue)]
struct DeleteEdgeBinding {
    workspace: RecordId,
    record: RecordId,
    source: RecordId,
    target: RecordId,
}

#[derive(SurrealValue)]
struct EdgeListBinding {
    workspace: RecordId,
    block: RecordId,
}

async fn edge_query(
    db: &SurrealDataContext<'_>,
    statement: &'static str,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Vec<LoomEdge>> {
    db.query_values::<EdgeRow, _>(
        statement,
        EdgeListBinding {
            workspace: thing("workspaces", workspace_id),
            block: thing(BLOCKS_TABLE, block_id),
        },
    )
    .await
    .map_err(map_err)?
    .into_iter()
    .map(edge_to_domain)
    .collect()
}

pub(crate) async fn list_loom_edges_for_block(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Vec<LoomEdge>> {
    edge_query(db, "SELECT * FROM loom_edges WHERE workspace_id = $workspace AND (source_block_id = $block OR target_block_id = $block) ORDER BY created_at ASC, edge_id ASC;", workspace_id, block_id).await
}

pub(crate) async fn get_backlinks(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Vec<LoomEdge>> {
    edge_query(db, "SELECT * FROM loom_edges WHERE workspace_id = $workspace AND target_block_id = $block ORDER BY created_at ASC, edge_id ASC;", workspace_id, block_id).await
}

pub(crate) async fn get_outgoing_edges(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Vec<LoomEdge>> {
    edge_query(db, "SELECT * FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $block ORDER BY created_at ASC, edge_id ASC;", workspace_id, block_id).await
}

async fn workspace_blocks(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
) -> StorageResult<Vec<LoomBlock>> {
    db.query_values::<BlockRow, _>(
        "SELECT * FROM loom_blocks WHERE workspace_id = $workspace;",
        WorkspaceBinding {
            workspace: thing("workspaces", workspace_id),
        },
    )
    .await
    .map_err(map_err)?
    .into_iter()
    .map(block_to_domain)
    .collect()
}

async fn workspace_edges(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
) -> StorageResult<Vec<LoomEdge>> {
    db.query_values::<EdgeRow, _>(
        "SELECT * FROM loom_edges WHERE workspace_id = $workspace;",
        WorkspaceBinding {
            workspace: thing("workspaces", workspace_id),
        },
    )
    .await
    .map_err(map_err)?
    .into_iter()
    .map(edge_to_domain)
    .collect()
}

pub(crate) async fn traverse_graph(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    start_block_id: &str,
    max_depth: u32,
    edge_types: &[LoomEdgeType],
) -> StorageResult<Vec<(LoomBlock, u32)>> {
    if max_depth == 0 {
        return Ok(Vec::new());
    }
    let blocks: HashMap<_, _> = workspace_blocks(db, workspace_id)
        .await?
        .into_iter()
        .map(|block| (block.block_id.clone(), block))
        .collect();
    let allowed: HashSet<_> = edge_types.iter().map(LoomEdgeType::as_str).collect();
    let mut outgoing: HashMap<String, Vec<(String, &str)>> = HashMap::new();
    for edge in workspace_edges(db, workspace_id).await? {
        outgoing
            .entry(edge.source_block_id)
            .or_default()
            .push((edge.target_block_id, edge.edge_type.as_str()));
    }
    let mut queue = VecDeque::from([(start_block_id.to_owned(), 0_u32)]);
    let mut depths: HashMap<String, u32> = HashMap::new();
    let mut seen = HashSet::from([start_block_id.to_owned()]);
    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        for (target, edge_type) in outgoing.get(&current).into_iter().flatten() {
            if !allowed.is_empty() && !allowed.contains(edge_type) {
                continue;
            }
            let next_depth = depth + 1;
            depths
                .entry(target.clone())
                .and_modify(|old| *old = (*old).min(next_depth))
                .or_insert(next_depth);
            if seen.insert(target.clone()) {
                queue.push_back((target.clone(), next_depth));
            }
        }
    }
    let mut result: Vec<_> = depths
        .into_iter()
        .filter_map(|(id, depth)| blocks.get(&id).cloned().map(|block| (block, depth)))
        .collect();
    result.sort_by(|(left_block, left_depth), (right_block, right_depth)| {
        left_depth
            .cmp(right_depth)
            .then_with(|| left_block.block_id.cmp(&right_block.block_id))
    });
    Ok(result)
}

pub(crate) async fn recompute_block_metrics(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<()> {
    let count = db
        .execute_returning(
            "UPDATE $record SET mention_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $record AND edge_type = 'mention')), tag_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $record AND edge_type = 'tag')), backlink_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND target_block_id = $record AND edge_type IN ['mention', 'tag'])) WHERE workspace_id = $workspace RETURN AFTER;",
            WorkspaceRecordBinding {
                workspace: thing("workspaces", workspace_id),
                record: thing(BLOCKS_TABLE, block_id),
            },
        )
        .await
        .map_err(map_err)?;
    if count == 0 {
        Err(StorageError::NotFound("loom_block"))
    } else {
        Ok(())
    }
}

pub(crate) async fn recompute_all_metrics(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
) -> StorageResult<()> {
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    let block_ids: Vec<_> = workspace_blocks(db, workspace_id)
        .await?
        .into_iter()
        .map(|block| block.block_id)
        .collect();
    for block_id in block_ids {
        recompute_block_metrics(db, workspace_id, &block_id).await?;
    }
    Ok(())
}

async fn asset_mimes(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
) -> StorageResult<HashMap<String, String>> {
    Ok(db
        .query_values::<AssetRow, _>(
            "SELECT * FROM assets WHERE workspace_id = $workspace;",
            WorkspaceBinding {
                workspace: thing("workspaces", workspace_id),
            },
        )
        .await
        .map_err(map_err)?
        .into_iter()
        .map(|row| (row.asset_id, row.mime))
        .collect())
}

fn block_matches_view(
    block: &LoomBlock,
    filters: &LoomViewFilters,
    mimes: &HashMap<String, String>,
    edges: &[LoomEdge],
) -> bool {
    if filters
        .content_type
        .as_ref()
        .is_some_and(|kind| kind != &block.content_type)
    {
        return false;
    }
    if filters.mime.as_ref().is_some_and(|mime| {
        block
            .asset_id
            .as_ref()
            .and_then(|id| mimes.get(id))
            .is_none_or(|actual| actual != mime)
    }) {
        return false;
    }
    let date = if block.content_type == LoomBlockContentType::Journal {
        block.journal_date.clone()
    } else {
        Some(block.updated_at.format("%Y-%m-%d").to_string())
    };
    if filters.date_from.is_some_and(|from| {
        let boundary = from.format("%Y-%m-%d").to_string();
        date.as_deref().is_none_or(|d| d < boundary.as_str())
    }) || filters.date_to.is_some_and(|to| {
        let boundary = to.format("%Y-%m-%d").to_string();
        date.as_deref().is_none_or(|d| d > boundary.as_str())
    }) {
        return false;
    }
    let has_edge = |kind: LoomEdgeType, ids: &[String]| {
        ids.is_empty()
            || edges.iter().any(|edge| {
                edge.source_block_id == block.block_id
                    && edge.edge_type == kind
                    && ids.contains(&edge.target_block_id)
            })
    };
    has_edge(LoomEdgeType::Tag, &filters.tag_ids)
        && has_edge(LoomEdgeType::Mention, &filters.mention_ids)
}

pub(crate) async fn query_loom_view(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    view_type: LoomViewType,
    filters: LoomViewFilters,
    limit: u32,
    offset: u32,
) -> StorageResult<LoomViewResponse> {
    let edges = workspace_edges(db, workspace_id).await?;
    let mimes = asset_mimes(db, workspace_id).await?;
    let mut blocks: Vec<_> = workspace_blocks(db, workspace_id)
        .await?
        .into_iter()
        .filter(|block| block_matches_view(block, &filters, &mimes, &edges))
        .collect();
    match view_type {
        LoomViewType::All | LoomViewType::Favorites | LoomViewType::Unlinked => {
            if view_type == LoomViewType::Favorites {
                blocks.retain(|block| block.favorite);
            } else if view_type == LoomViewType::Unlinked {
                blocks.retain(|block| {
                    !edges.iter().any(|edge| {
                        matches!(edge.edge_type, LoomEdgeType::Mention | LoomEdgeType::Tag)
                            && (edge.source_block_id == block.block_id
                                || edge.target_block_id == block.block_id)
                    })
                });
            }
            blocks.sort_by(|left, right| {
                right
                    .updated_at
                    .cmp(&left.updated_at)
                    .then_with(|| left.block_id.cmp(&right.block_id))
            });
            let blocks = blocks
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect();
            Ok(match view_type {
                LoomViewType::All => LoomViewResponse::All { blocks },
                LoomViewType::Favorites => LoomViewResponse::Favorites { blocks },
                _ => LoomViewResponse::Unlinked { blocks },
            })
        }
        LoomViewType::Pins => {
            blocks.retain(|block| block.pinned);
            blocks.sort_by(|left, right| {
                left.pin_order
                    .is_none()
                    .cmp(&right.pin_order.is_none())
                    .then_with(|| left.pin_order.cmp(&right.pin_order))
                    .then_with(|| right.updated_at.cmp(&left.updated_at))
                    .then_with(|| left.block_id.cmp(&right.block_id))
            });
            Ok(LoomViewResponse::Pins {
                blocks: blocks
                    .into_iter()
                    .skip(offset as usize)
                    .take(limit as usize)
                    .collect(),
            })
        }
        LoomViewType::Sorted => {
            let by_id: HashMap<_, _> = blocks
                .into_iter()
                .map(|block| (block.block_id.clone(), block))
                .collect();
            let mut grouped: BTreeMap<(String, String), Vec<LoomBlock>> = BTreeMap::new();
            for edge in &edges {
                if !matches!(edge.edge_type, LoomEdgeType::Mention | LoomEdgeType::Tag) {
                    continue;
                }
                if let Some(block) = by_id.get(&edge.source_block_id) {
                    grouped
                        .entry((
                            edge.edge_type.as_str().to_owned(),
                            edge.target_block_id.clone(),
                        ))
                        .or_default()
                        .push(block.clone());
                }
            }
            let groups = grouped
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .map(|((edge_type, target_block_id), mut blocks)| {
                    blocks.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
                    blocks.truncate(100);
                    Ok(LoomViewGroup {
                        edge_type: LoomEdgeType::from_str(&edge_type)?,
                        target_block_id,
                        blocks,
                    })
                })
                .collect::<StorageResult<Vec<_>>>()?;
            Ok(LoomViewResponse::Sorted { groups })
        }
    }
}

fn source_kind(block: &LoomBlock) -> LoomSearchSourceKind {
    match block.content_type {
        LoomBlockContentType::File | LoomBlockContentType::AnnotatedFile => {
            LoomSearchSourceKind::File
        }
        LoomBlockContentType::TagHub => LoomSearchSourceKind::TagHub,
        _ => LoomSearchSourceKind::LoomBlock,
    }
}

fn reachable_target(
    start: &str,
    targets: &[String],
    kind: LoomEdgeType,
    max_depth: u32,
    edges: &[LoomEdge],
) -> bool {
    if targets.is_empty() {
        return true;
    }
    let mut queue = VecDeque::from([(start.to_owned(), 0_u32)]);
    let mut seen = HashSet::from([start.to_owned()]);
    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_depth.max(1) {
            continue;
        }
        for edge in edges.iter().filter(|edge| edge.source_block_id == current) {
            if edge.edge_type == kind && targets.contains(&edge.target_block_id) {
                return true;
            }
            if seen.insert(edge.target_block_id.clone()) {
                queue.push_back((edge.target_block_id.clone(), depth + 1));
            }
        }
    }
    false
}

fn text_matcher(
    query: &str,
    filters: &LoomSearchFilters,
) -> StorageResult<Box<dyn Fn(&str) -> bool + Send + Sync>> {
    if filters.is_regex {
        let pattern = if filters.case_sensitive {
            query.to_owned()
        } else {
            format!("(?i:{query})")
        };
        let regex = Regex::new(&pattern)
            .map_err(|_| StorageError::Validation("invalid loom search regex"))?;
        return Ok(Box::new(move |haystack| regex.is_match(haystack)));
    }
    let case_sensitive = filters.case_sensitive;
    let needles: Vec<String> = query
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(|token| {
            if case_sensitive {
                token.to_owned()
            } else {
                token.to_lowercase()
            }
        })
        .collect();
    let whole_word = filters.whole_word;
    Ok(Box::new(move |haystack| {
        let haystack = if case_sensitive {
            haystack.to_owned()
        } else {
            haystack.to_lowercase()
        };
        if whole_word {
            let words: HashSet<&str> = haystack
                .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
                .collect();
            needles.iter().all(|needle| words.contains(needle.as_str()))
        } else {
            needles.iter().all(|needle| haystack.contains(needle))
        }
    }))
}

fn loom_search_source_allowed(
    filters: &LoomSearchFilters,
    source_kind: LoomSearchSourceKind,
) -> bool {
    filters.source_kinds.is_empty() || filters.source_kinds.contains(&source_kind)
}

fn loom_search_has_block_scoped_filters(filters: &LoomSearchFilters) -> bool {
    filters.content_type.is_some()
        || filters.mime.is_some()
        || !filters.tag_ids.is_empty()
        || !filters.mention_ids.is_empty()
        || filters.backlink_depth.is_some()
}

fn loom_search_excerpt(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(240)
        .collect()
}

fn loom_search_path_matches(filters: &LoomSearchFilters, values: &[&str]) -> bool {
    let Some(path) = filters
        .path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return true;
    };
    if filters.case_sensitive {
        values.iter().any(|value| value.contains(path))
    } else {
        let path = path.to_lowercase();
        values
            .iter()
            .any(|value| value.to_lowercase().contains(&path))
    }
}

fn loom_fuzzy_query(query: &str, filters: &LoomSearchFilters) -> Option<String> {
    if filters.case_sensitive || filters.whole_word || filters.is_regex {
        return None;
    }
    let mut tokens = query.split_whitespace();
    let token = tokens.next()?;
    if tokens.next().is_some() {
        return None;
    }
    let compact: String = token
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect();
    (2..=12).contains(&compact.len()).then_some(compact)
}

fn is_ascii_subsequence(needle: &str, haystack: &str) -> bool {
    let mut haystack_chars = haystack.chars();
    needle.chars().all(|needle_ch| {
        haystack_chars
            .by_ref()
            .any(|haystack_ch| haystack_ch == needle_ch)
    })
}

fn loom_fuzzy_forms(value: &str) -> (String, String) {
    let mut compact = String::new();
    let mut initials = String::new();
    let mut at_word_start = true;
    let mut previous: Option<char> = None;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            let lower = ch.to_ascii_lowercase();
            compact.push(lower);
            let camel_boundary = ch.is_ascii_uppercase()
                && previous.is_some_and(|previous| {
                    previous.is_ascii_lowercase() || previous.is_ascii_digit()
                });
            if at_word_start || camel_boundary {
                initials.push(lower);
            }
            at_word_start = false;
        } else {
            at_word_start = true;
        }
        previous = Some(ch);
    }
    (compact, initials)
}

fn loom_typo_max_distance(query_len: usize) -> usize {
    match query_len {
        0..=3 => 0,
        4..=8 => 1,
        9..=16 => 2,
        _ => 3,
    }
}

fn loom_bounded_edit_distance(left: &str, right: &str, max_distance: usize) -> Option<usize> {
    if left.len().abs_diff(right.len()) > max_distance {
        return None;
    }
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];
    for (left_index, left_byte) in left.bytes().enumerate() {
        current[0] = left_index + 1;
        let mut row_min = current[0];
        for (right_index, right_byte) in right.bytes().enumerate() {
            let substitution = usize::from(left_byte != right_byte);
            let value = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution);
            current[right_index + 1] = value;
            row_min = row_min.min(value);
        }
        if row_min > max_distance {
            return None;
        }
        std::mem::swap(&mut previous, &mut current);
    }
    let distance = previous[right.len()];
    (distance <= max_distance).then_some(distance)
}

fn loom_typo_field_score(query: &str, compact: &str) -> Option<f64> {
    let max_distance = loom_typo_max_distance(query.len());
    if max_distance == 0 || compact.len() < 4 {
        return None;
    }
    let min_window = query.len().saturating_sub(max_distance).max(1);
    let max_window = (query.len() + max_distance).min(compact.len());
    let mut best: Option<(usize, usize)> = None;
    for window_len in min_window..=max_window {
        for start in 0..=compact.len().saturating_sub(window_len) {
            if let Some(distance) =
                loom_bounded_edit_distance(query, &compact[start..start + window_len], max_distance)
            {
                let current = (distance, start.min(8));
                if best.is_none_or(|best| current < best) {
                    best = Some(current);
                }
            }
        }
    }
    best.map(|(distance, start_penalty)| {
        14.0 - (distance as f64 * 2.0) - (start_penalty as f64 * 0.25)
    })
    .filter(|score| *score > 0.0)
}

fn loom_fuzzy_field_score(query: &str, value: &str) -> Option<f64> {
    if value.trim().is_empty() {
        return None;
    }
    let (compact, initials) = loom_fuzzy_forms(value);
    if initials.starts_with(query) {
        return Some(24.0);
    }
    if is_ascii_subsequence(query, &initials) {
        return Some(20.0);
    }
    if compact.starts_with(query) {
        return Some(12.0);
    }
    if compact.contains(query) {
        return Some(10.0);
    }
    if query.len() >= 4 && is_ascii_subsequence(query, &compact) {
        return Some(6.0);
    }
    loom_typo_field_score(query, &compact)
}

fn loom_fuzzy_score<'a>(query: &str, fields: impl IntoIterator<Item = &'a str>) -> Option<f64> {
    fields
        .into_iter()
        .filter_map(|field| loom_fuzzy_field_score(query, field))
        .max_by(|left, right| left.total_cmp(right))
}

fn loom_graph_match_score<'a>(
    matcher: &(dyn Fn(&str) -> bool + Send + Sync),
    fuzzy_query: Option<&str>,
    fields: impl IntoIterator<Item = &'a str>,
) -> Option<f64> {
    let fields: Vec<&str> = fields.into_iter().collect();
    if let Some(query) = fuzzy_query {
        loom_fuzzy_score(query, fields)
    } else {
        matcher(&fields.join("\n")).then_some(0.0)
    }
}

const LOOM_GRAPH_SOURCE_ORDER: [LoomSearchSourceKind; 9] = [
    LoomSearchSourceKind::LoomBlock,
    LoomSearchSourceKind::File,
    LoomSearchSourceKind::TagHub,
    LoomSearchSourceKind::Document,
    LoomSearchSourceKind::Symbol,
    LoomSearchSourceKind::WorkPacket,
    LoomSearchSourceKind::MicroTask,
    LoomSearchSourceKind::UserManualPage,
    LoomSearchSourceKind::WikiPage,
];

fn order_loom_graph_results_for_breadth(
    results: Vec<LoomGraphSearchResult>,
) -> Vec<LoomGraphSearchResult> {
    let mut buckets: Vec<VecDeque<LoomGraphSearchResult>> = LOOM_GRAPH_SOURCE_ORDER
        .iter()
        .map(|_| VecDeque::new())
        .collect();
    for result in results {
        let index = LOOM_GRAPH_SOURCE_ORDER
            .iter()
            .position(|candidate| *candidate == result.source_kind)
            .unwrap_or(0);
        buckets[index].push_back(result);
    }
    for bucket in &mut buckets {
        bucket.make_contiguous().sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.ref_id.cmp(&right.ref_id))
        });
    }
    let mut ordered = Vec::new();
    loop {
        let mut added = false;
        for bucket in &mut buckets {
            if let Some(result) = bucket.pop_front() {
                ordered.push(result);
                added = true;
            }
        }
        if !added {
            return ordered;
        }
    }
}

#[derive(SurrealValue)]
struct KnowledgeEntitySearchRow {
    entity_id: String,
    entity_kind: String,
    entity_key: String,
    display_name: String,
    detection_provenance: JsonValue,
}

#[derive(SurrealValue)]
struct RichDocumentSearchRow {
    rich_document_id: String,
    document_id: Option<RecordId>,
    title: String,
    schema_version: String,
    doc_version: i64,
    authority_label: String,
    content_json: JsonValue,
}

#[derive(SurrealValue)]
struct UserManualPageSearchRow {
    page_id: String,
    slug: String,
    title: String,
    body: JsonValue,
}

#[derive(SurrealValue)]
struct EmptySearchBinding {}

#[derive(SurrealValue)]
struct UserManualSectionSearchRow {
    page_id: RecordId,
    title: String,
    body_md: String,
}

#[derive(SurrealValue)]
struct WikiPageSearchRow {
    projection_id: String,
    title: String,
    rendered_content: String,
    page_type: Option<String>,
    rebuild_status: String,
}

pub(crate) async fn search_loom_blocks(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    query: &str,
    filters: LoomSearchFilters,
    limit: u32,
    offset: u32,
) -> StorageResult<Vec<LoomBlockSearchResult>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let matcher = text_matcher(query.trim(), &filters)?;
    let fuzzy_query = loom_fuzzy_query(query.trim(), &filters);
    let edges = workspace_edges(db, workspace_id).await?;
    let mimes = asset_mimes(db, workspace_id).await?;
    let depth = filters.backlink_depth.unwrap_or(1);
    let mut results = Vec::new();
    for block in workspace_blocks(db, workspace_id).await? {
        if filters
            .content_type
            .as_ref()
            .is_some_and(|kind| kind != &block.content_type)
            || (!filters.source_kinds.is_empty()
                && !filters.source_kinds.contains(&source_kind(&block)))
            || filters.mime.as_ref().is_some_and(|mime| {
                block
                    .asset_id
                    .as_ref()
                    .and_then(|id| mimes.get(id))
                    .is_none_or(|actual| actual != mime)
            })
            || !loom_search_path_matches(
                &filters,
                &[
                    block.block_id.as_str(),
                    block.document_id.as_deref().unwrap_or_default(),
                    block.original_filename.as_deref().unwrap_or_default(),
                    block.title.as_deref().unwrap_or_default(),
                ],
            )
            || !reachable_target(
                &block.block_id,
                &filters.tag_ids,
                LoomEdgeType::Tag,
                depth,
                &edges,
            )
            || !reachable_target(
                &block.block_id,
                &filters.mention_ids,
                LoomEdgeType::Mention,
                depth,
                &edges,
            )
        {
            continue;
        }
        let searchable = loom_search_text(&block);
        let fuzzy_score = fuzzy_query.as_deref().and_then(|needle| {
            loom_fuzzy_score(
                needle,
                [
                    block.block_id.as_str(),
                    block.title.as_deref().unwrap_or_default(),
                    block.original_filename.as_deref().unwrap_or_default(),
                    searchable.as_str(),
                ],
            )
        });
        if fuzzy_query.is_some() && fuzzy_score.is_none() {
            continue;
        }
        if fuzzy_query.is_none() && !matcher(&searchable) && !matcher(&block.block_id) {
            continue;
        }
        let score = (if block.pinned { 5.0 } else { 0.0 })
            + (if block.favorite { 3.0 } else { 0.0 })
            + block.derived.tag_count.clamp(0, 10) as f64 * 1.5
            + block.derived.backlink_count.clamp(0, 10) as f64
            + fuzzy_score.unwrap_or(1.0);
        results.push(LoomBlockSearchResult { block, score });
    }
    results.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| right.block.updated_at.cmp(&left.block.updated_at))
            .then_with(|| left.block.block_id.cmp(&right.block.block_id))
    });
    Ok(results
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect())
}

pub(crate) async fn search_loom_graph(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    query: &str,
    filters: LoomSearchFilters,
    limit: u32,
    offset: u32,
) -> StorageResult<Vec<LoomGraphSearchResult>> {
    let query = query.trim();
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let matcher = text_matcher(query, &filters)?;
    let fuzzy_query = loom_fuzzy_query(query, &filters);
    let source_limit = limit.min(500);
    let candidate_limit = offset.saturating_add(source_limit).max(source_limit);
    let mut results = Vec::new();

    for requested_kind in [
        LoomSearchSourceKind::LoomBlock,
        LoomSearchSourceKind::File,
        LoomSearchSourceKind::TagHub,
    ] {
        if !loom_search_source_allowed(&filters, requested_kind) {
            continue;
        }
        let mut block_filters = filters.clone();
        block_filters.source_kinds = vec![requested_kind];
        for row in
            search_loom_blocks(db, workspace_id, query, block_filters, candidate_limit, 0).await?
        {
            let block = row.block;
            let actual_kind = source_kind(&block);
            if actual_kind != requested_kind {
                continue;
            }
            results.push(LoomGraphSearchResult {
                result_kind: LoomSearchResultKind::LoomBlock,
                source_kind: actual_kind,
                ref_id: block.block_id.clone(),
                title: block
                    .title
                    .clone()
                    .or_else(|| block.original_filename.clone())
                    .unwrap_or_else(|| block.block_id.clone()),
                excerpt: block
                    .derived
                    .full_text_index
                    .as_deref()
                    .map(loom_search_excerpt)
                    .unwrap_or_default(),
                metadata: json!({
                    "authority_table": "loom_blocks",
                    "content_type": block.content_type.as_str(),
                    "backlink_count": block.derived.backlink_count,
                    "tag_count": block.derived.tag_count,
                }),
                block: Some(block),
                score: row.score,
            });
        }
    }

    if !loom_search_has_block_scoped_filters(&filters) {
        let workspace = WorkspaceBinding {
            workspace: thing("workspaces", workspace_id),
        };
        let entities = db
            .query_values::<KnowledgeEntitySearchRow, _>(
                "SELECT entity_id, entity_kind, entity_key, display_name, detection_provenance FROM knowledge_entities WHERE workspace_id = $workspace AND lifecycle_state = 'active' AND entity_kind IN ['symbol', 'work_packet', 'micro_task'];",
                workspace,
            )
            .await
            .map_err(map_err)?;
        for row in entities {
            let source_kind = LoomSearchSourceKind::from_str(&row.entity_kind)?;
            if !loom_search_source_allowed(&filters, source_kind)
                || !loom_search_path_matches(
                    &filters,
                    &[&row.entity_id, &row.entity_key, &row.display_name],
                )
            {
                continue;
            }
            let provenance = row.detection_provenance.to_string();
            let Some(score) = loom_graph_match_score(
                matcher.as_ref(),
                fuzzy_query.as_deref(),
                [
                    row.entity_id.as_str(),
                    row.entity_key.as_str(),
                    row.display_name.as_str(),
                    provenance.as_str(),
                ],
            ) else {
                continue;
            };
            results.push(LoomGraphSearchResult {
                result_kind: LoomSearchResultKind::KnowledgeEntity,
                source_kind,
                ref_id: row.entity_id,
                title: row.display_name,
                excerpt: loom_search_excerpt(&row.entity_key),
                block: None,
                score,
                metadata: json!({
                    "authority_table": "knowledge_entities",
                    "entity_key": row.entity_key,
                    "detection_provenance": row.detection_provenance,
                }),
            });
        }

        if loom_search_source_allowed(&filters, LoomSearchSourceKind::Document) {
            let documents = db
                .query_values::<RichDocumentSearchRow, _>(
                    "SELECT rich_document_id, document_id, title, schema_version, doc_version, authority_label, content_json FROM knowledge_rich_documents WHERE workspace_id = $workspace AND deleted_at = NONE;",
                    WorkspaceBinding {
                        workspace: thing("workspaces", workspace_id),
                    },
                )
                .await
                .map_err(map_err)?;
            for row in documents {
                let document_id = opt_record_key(row.document_id, "documents")?;
                let content = row.content_json.to_string();
                if !loom_search_path_matches(
                    &filters,
                    &[
                        &row.rich_document_id,
                        document_id.as_deref().unwrap_or_default(),
                        &row.title,
                    ],
                ) {
                    continue;
                }
                let Some(score) = loom_graph_match_score(
                    matcher.as_ref(),
                    fuzzy_query.as_deref(),
                    [
                        row.rich_document_id.as_str(),
                        document_id.as_deref().unwrap_or_default(),
                        row.title.as_str(),
                        content.as_str(),
                    ],
                ) else {
                    continue;
                };
                results.push(LoomGraphSearchResult {
                    result_kind: LoomSearchResultKind::KnowledgeEntity,
                    source_kind: LoomSearchSourceKind::Document,
                    ref_id: row.rich_document_id.clone(),
                    title: row.title,
                    excerpt: loom_search_excerpt(&content),
                    block: None,
                    score,
                    metadata: json!({
                        "authority_table": "knowledge_rich_documents",
                        "rich_document_id": row.rich_document_id,
                        "document_id": document_id,
                        "schema_version": row.schema_version,
                        "doc_version": row.doc_version,
                        "authority_label": row.authority_label,
                    }),
                });
            }
        }

        if loom_search_source_allowed(&filters, LoomSearchSourceKind::UserManualPage) {
            let pages = db
                .query_values::<UserManualPageSearchRow, _>(
                    "SELECT page_id, slug, title, body FROM user_manual_pages WHERE status = 'current';",
                    EmptySearchBinding {},
                )
                .await
                .map_err(map_err)?;
            let sections = db
                .query_values::<UserManualSectionSearchRow, _>(
                    "SELECT page_id, title, body_md FROM user_manual_sections;",
                    EmptySearchBinding {},
                )
                .await
                .map_err(map_err)?;
            let mut sections_by_page: HashMap<String, Vec<(String, String)>> = HashMap::new();
            for section in sections {
                sections_by_page
                    .entry(record_key(section.page_id, "user_manual_pages")?)
                    .or_default()
                    .push((section.title, section.body_md));
            }
            for row in pages {
                let sections = sections_by_page.remove(&row.page_id).unwrap_or_default();
                let body = row.body.to_string();
                if loom_search_path_matches(&filters, &[&row.slug, &row.title]) {
                    if let Some(score) = loom_graph_match_score(
                        matcher.as_ref(),
                        fuzzy_query.as_deref(),
                        [row.slug.as_str(), row.title.as_str(), body.as_str()],
                    ) {
                        results.push(LoomGraphSearchResult {
                            result_kind: LoomSearchResultKind::UserManualPage,
                            source_kind: LoomSearchSourceKind::UserManualPage,
                            ref_id: row.slug.clone(),
                            title: row.title.clone(),
                            excerpt: loom_search_excerpt(&body),
                            block: None,
                            score,
                            metadata: json!({
                                "authority_table": "user_manual_pages",
                                "page_slug": row.slug.clone(),
                            }),
                        });
                    }
                }
                for (section_title, section_body) in sections {
                    if !loom_search_path_matches(&filters, &[&row.slug, &row.title, &section_title])
                    {
                        continue;
                    }
                    let Some(score) = loom_graph_match_score(
                        matcher.as_ref(),
                        fuzzy_query.as_deref(),
                        [
                            row.slug.as_str(),
                            row.title.as_str(),
                            section_title.as_str(),
                            section_body.as_str(),
                        ],
                    ) else {
                        continue;
                    };
                    results.push(LoomGraphSearchResult {
                        result_kind: LoomSearchResultKind::UserManualPage,
                        source_kind: LoomSearchSourceKind::UserManualPage,
                        ref_id: row.slug.clone(),
                        title: section_title,
                        excerpt: loom_search_excerpt(&section_body),
                        block: None,
                        score,
                        metadata: json!({
                            "authority_table": "user_manual_sections",
                            "page_slug": row.slug.clone(),
                        }),
                    });
                }
            }
        }

        if loom_search_source_allowed(&filters, LoomSearchSourceKind::WikiPage) {
            let pages = db
                .query_values::<WikiPageSearchRow, _>(
                    "SELECT projection_id, title, rendered_content, page_type, rebuild_status FROM knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_kind = 'wiki_page';",
                    WorkspaceBinding {
                        workspace: thing("workspaces", workspace_id),
                    },
                )
                .await
                .map_err(map_err)?;
            for row in pages {
                if !loom_search_path_matches(&filters, &[&row.projection_id, &row.title]) {
                    continue;
                }
                let Some(score) = loom_graph_match_score(
                    matcher.as_ref(),
                    fuzzy_query.as_deref(),
                    [
                        row.projection_id.as_str(),
                        row.title.as_str(),
                        row.rendered_content.as_str(),
                    ],
                ) else {
                    continue;
                };
                results.push(LoomGraphSearchResult {
                    result_kind: LoomSearchResultKind::WikiPage,
                    source_kind: LoomSearchSourceKind::WikiPage,
                    ref_id: row.projection_id.clone(),
                    title: row.title,
                    excerpt: loom_search_excerpt(&row.rendered_content),
                    block: None,
                    score,
                    metadata: json!({
                        "authority_table": "knowledge_wiki_projections",
                        "projection_id": row.projection_id,
                        "page_type": row.page_type,
                        "rebuild_status": row.rebuild_status,
                    }),
                });
            }
        }
    }

    Ok(order_loom_graph_results_for_breadth(results)
        .into_iter()
        .skip(offset as usize)
        .take(source_limit as usize)
        .collect())
}

fn loom_block_scan_text(block: &LoomBlock) -> String {
    [
        block.title.as_deref(),
        block.derived.full_text_index.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

pub(crate) async fn get_backlinks_with_context(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Vec<LoomBacklink>> {
    let target = get_loom_block(db, workspace_id, block_id).await?;
    let target_title = target
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut result = Vec::new();
    for edge in get_backlinks(db, workspace_id, block_id).await? {
        let source_block = get_loom_block(db, workspace_id, &edge.source_block_id).await?;
        let scanned = loom_block_scan_text(&source_block);
        let context_snippet = target_title
            .and_then(|title| {
                crate::storage::loom_find_unlinked_term(&scanned, title)
                    .map(|(start, len)| crate::storage::loom_context_snippet(&scanned, start, len))
            })
            .or_else(|| {
                (!scanned.trim().is_empty())
                    .then(|| crate::storage::loom_context_snippet(&scanned, 0, 0))
            });
        result.push(LoomBacklink {
            edge,
            source_block,
            context_snippet,
        });
    }
    Ok(result)
}

pub(crate) async fn scan_unlinked_mentions(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
    aliases: &[String],
    limit: u32,
) -> StorageResult<Vec<LoomUnlinkedMention>> {
    let target = get_loom_block(db, workspace_id, block_id).await?;
    let mut terms = Vec::new();
    if let Some(title) = target
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        terms.push(title.to_owned());
    }
    terms.extend(
        aliases
            .iter()
            .map(|alias| alias.trim())
            .filter(|alias| !alias.is_empty())
            .map(str::to_owned),
    );
    if terms.is_empty() {
        return Ok(Vec::new());
    }

    let edges = workspace_edges(db, workspace_id).await?;
    let mut blocks = workspace_blocks(db, workspace_id).await?;
    blocks.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.block_id.cmp(&right.block_id))
    });
    let scan_limit = limit.clamp(1, 500) as usize;
    let lowercase_terms: Vec<_> = terms.iter().map(|term| term.to_lowercase()).collect();
    let candidates = blocks.into_iter().filter(|source_block| {
        if source_block.block_id == block_id
            || edges.iter().any(|edge| {
                edge.source_block_id == source_block.block_id
                    && edge.target_block_id == block_id
                    && matches!(
                        edge.edge_type,
                        LoomEdgeType::Mention | LoomEdgeType::Tag | LoomEdgeType::SubTag
                    )
            })
        {
            return false;
        }
        let searchable = [
            source_block.title.as_deref(),
            source_block.original_filename.as_deref(),
            source_block.derived.full_text_index.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
        lowercase_terms.iter().any(|term| searchable.contains(term))
    });
    let mut mentions = Vec::new();
    for source_block in candidates.take(scan_limit) {
        let scanned = loom_block_scan_text(&source_block);
        let best = terms
            .iter()
            .filter_map(|term| {
                crate::storage::loom_find_unlinked_term(&scanned, term)
                    .map(|(start, len)| (term, start, len))
            })
            .min_by_key(|(_, start, _)| *start);
        if let Some((term, start, len)) = best {
            mentions.push(LoomUnlinkedMention {
                source_block,
                matched_term: term.clone(),
                snippet: crate::storage::loom_context_snippet(&scanned, start, len),
                match_offset: start as i64,
            });
        }
    }
    Ok(mentions)
}

#[derive(SurrealValue)]
struct LoomBridgeRow {
    block_id: RecordId,
    entity_id: RecordId,
}

async fn assemble_loom_graph(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    node_ids: &[String],
    depth_by_id: &HashMap<String, u32>,
    edge_types: &[LoomEdgeType],
    truncated: bool,
    suppressed_hub_ids: Vec<String>,
) -> StorageResult<LoomGraph> {
    if node_ids.is_empty() {
        return Ok(LoomGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            truncated,
            suppressed_hub_ids,
        });
    }
    let node_set: HashSet<&str> = node_ids.iter().map(String::as_str).collect();
    let allowed: HashSet<&str> = edge_types.iter().map(LoomEdgeType::as_str).collect();
    let mut blocks: HashMap<String, LoomBlock> = workspace_blocks(db, workspace_id)
        .await?
        .into_iter()
        .filter(|block| node_set.contains(block.block_id.as_str()))
        .map(|block| (block.block_id.clone(), block))
        .collect();
    let mut edges = Vec::new();
    let mut degree: HashMap<String, u32> = HashMap::new();
    let mut domain_edges = workspace_edges(db, workspace_id).await?;
    domain_edges.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.edge_id.cmp(&right.edge_id))
    });
    for edge in domain_edges {
        if node_set.contains(edge.source_block_id.as_str())
            && node_set.contains(edge.target_block_id.as_str())
            && (allowed.is_empty() || allowed.contains(edge.edge_type.as_str()))
        {
            *degree.entry(edge.source_block_id.clone()).or_default() += 1;
            *degree.entry(edge.target_block_id.clone()).or_default() += 1;
            let stale = edge.edge_type == LoomEdgeType::AiSuggested;
            edges.push(LoomGraphEdge { edge, stale });
        }
    }
    let bridges = db
        .query_values::<LoomBridgeRow, _>(
            "SELECT block_id, entity_id FROM loom_block_knowledge_bridge WHERE workspace_id = $workspace;",
            WorkspaceBinding {
                workspace: thing("workspaces", workspace_id),
            },
        )
        .await
        .map_err(map_err)?;
    let entity_by_block = bridges
        .into_iter()
        .map(|row| {
            Ok((
                record_key(row.block_id, BLOCKS_TABLE)?,
                record_key(row.entity_id, "knowledge_entities")?,
            ))
        })
        .collect::<StorageResult<HashMap<_, _>>>()?;
    let mut ordered_ids: Vec<_> = blocks.keys().cloned().collect();
    ordered_ids.sort_by(|left, right| {
        depth_by_id
            .get(left)
            .copied()
            .unwrap_or(0)
            .cmp(&depth_by_id.get(right).copied().unwrap_or(0))
            .then_with(|| left.cmp(right))
    });
    let nodes = ordered_ids
        .into_iter()
        .filter_map(|id| {
            let block = blocks.remove(&id)?;
            let entity_id = entity_by_block.get(&id).cloned();
            Some(LoomGraphNode {
                block,
                depth: depth_by_id.get(&id).copied().unwrap_or(0),
                degree: degree.get(&id).copied().unwrap_or(0),
                stale: entity_id.is_none(),
                entity_id,
            })
        })
        .collect();
    Ok(LoomGraph {
        nodes,
        edges,
        truncated,
        suppressed_hub_ids,
    })
}

pub(crate) async fn local_graph(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    start_block_id: &str,
    max_depth: u32,
    edge_types: &[LoomEdgeType],
    node_limit: u32,
) -> StorageResult<LoomGraph> {
    get_loom_block(db, workspace_id, start_block_id).await?;
    let allowed: HashSet<&str> = edge_types.iter().map(LoomEdgeType::as_str).collect();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for edge in workspace_edges(db, workspace_id).await? {
        if !allowed.is_empty() && !allowed.contains(edge.edge_type.as_str()) {
            continue;
        }
        adjacency
            .entry(edge.source_block_id.clone())
            .or_default()
            .push(edge.target_block_id.clone());
        adjacency
            .entry(edge.target_block_id)
            .or_default()
            .push(edge.source_block_id);
    }
    for neighbors in adjacency.values_mut() {
        neighbors.sort();
        neighbors.dedup();
    }
    let depth_limit = max_depth.max(1);
    let mut queue = VecDeque::from([(start_block_id.to_owned(), 0_u32)]);
    let mut depth_by_id = HashMap::from([(start_block_id.to_owned(), 0_u32)]);
    while let Some((current, depth)) = queue.pop_front() {
        if depth >= depth_limit {
            continue;
        }
        for next in adjacency.get(&current).into_iter().flatten() {
            if !depth_by_id.contains_key(next) {
                depth_by_id.insert(next.clone(), depth + 1);
                queue.push_back((next.clone(), depth + 1));
            }
        }
    }
    let cap = node_limit.clamp(1, 5000) as usize;
    let mut node_ids: Vec<_> = depth_by_id.keys().cloned().collect();
    node_ids.sort_by(|left, right| {
        depth_by_id[left]
            .cmp(&depth_by_id[right])
            .then_with(|| left.cmp(right))
    });
    let truncated = node_ids.len() > cap;
    node_ids.truncate(cap);
    assemble_loom_graph(
        db,
        workspace_id,
        &node_ids,
        &depth_by_id,
        edge_types,
        truncated,
        Vec::new(),
    )
    .await
}

pub(crate) async fn global_graph(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    edge_types: &[LoomEdgeType],
    node_limit: u32,
    hub_degree_threshold: u32,
) -> StorageResult<LoomGraph> {
    let allowed: HashSet<&str> = edge_types.iter().map(LoomEdgeType::as_str).collect();
    let mut degree = HashMap::<String, u32>::new();
    for edge in workspace_edges(db, workspace_id).await? {
        if allowed.is_empty() || allowed.contains(edge.edge_type.as_str()) {
            *degree.entry(edge.source_block_id).or_default() += 1;
            *degree.entry(edge.target_block_id).or_default() += 1;
        }
    }
    let mut suppressed_hub_ids: Vec<_> = degree
        .iter()
        .filter(|(_, value)| hub_degree_threshold > 0 && **value > hub_degree_threshold)
        .map(|(id, _)| id.clone())
        .collect();
    suppressed_hub_ids.sort();
    let suppressed: HashSet<&str> = suppressed_hub_ids.iter().map(String::as_str).collect();
    let mut blocks = workspace_blocks(db, workspace_id).await?;
    blocks.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.block_id.cmp(&right.block_id))
    });
    let cap = node_limit.clamp(1, crate::storage::LOOM_GLOBAL_GRAPH_MAX_NODE_LIMIT) as usize;
    let mut node_ids: Vec<_> = blocks
        .into_iter()
        .map(|block| block.block_id)
        .filter(|id| !suppressed.contains(id.as_str()))
        .collect();
    let truncated = node_ids.len() > cap;
    node_ids.truncate(cap);
    let depth_by_id = node_ids
        .iter()
        .map(|id| (id.clone(), 0_u32))
        .collect::<HashMap<_, _>>();
    assemble_loom_graph(
        db,
        workspace_id,
        &node_ids,
        &depth_by_id,
        edge_types,
        truncated,
        suppressed_hub_ids,
    )
    .await
}

pub(crate) async fn list_tag_hubs(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    limit: u32,
    offset: u32,
) -> StorageResult<Vec<LoomBlock>> {
    let mut blocks: Vec<_> = workspace_blocks(db, workspace_id)
        .await?
        .into_iter()
        .filter(|block| block.content_type == LoomBlockContentType::TagHub)
        .collect();
    blocks.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.block_id.cmp(&right.block_id))
    });
    Ok(blocks
        .into_iter()
        .skip(offset as usize)
        .take(limit.clamp(1, 500) as usize)
        .collect())
}

pub(crate) async fn get_tag_hub(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    tag_block_id: &str,
) -> StorageResult<LoomTagHub> {
    let block = get_loom_block(db, workspace_id, tag_block_id).await?;
    if block.content_type != LoomBlockContentType::TagHub {
        return Err(StorageError::Validation("loom block is not a tag_hub"));
    }
    let blocks: HashMap<_, _> = workspace_blocks(db, workspace_id)
        .await?
        .into_iter()
        .map(|block| (block.block_id.clone(), block))
        .collect();
    let edges = workspace_edges(db, workspace_id).await?;
    let mut incoming = |edge_type: LoomEdgeType| {
        let mut result: Vec<_> = edges
            .iter()
            .filter(|edge| edge.target_block_id == tag_block_id && edge.edge_type == edge_type)
            .filter_map(|edge| blocks.get(&edge.source_block_id).cloned())
            .collect();
        result.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.block_id.cmp(&right.block_id))
        });
        result.dedup_by(|left, right| left.block_id == right.block_id);
        result
    };
    Ok(LoomTagHub {
        block,
        sub_tags: incoming(LoomEdgeType::SubTag),
        tagged_blocks: incoming(LoomEdgeType::Tag),
        backlink_count: edges
            .iter()
            .filter(|edge| edge.target_block_id == tag_block_id)
            .count() as i64,
    })
}

pub(crate) async fn list_blocks_for_tag(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    tag_block_id: &str,
    include_subtags: bool,
    limit: u32,
    offset: u32,
) -> StorageResult<Vec<LoomBlock>> {
    let block = get_loom_block(db, workspace_id, tag_block_id).await?;
    if block.content_type != LoomBlockContentType::TagHub {
        return Err(StorageError::Validation("loom block is not a tag_hub"));
    }
    let edges = workspace_edges(db, workspace_id).await?;
    let mut tag_ids = HashSet::from([tag_block_id.to_owned()]);
    if include_subtags {
        let mut queue = VecDeque::from([tag_block_id.to_owned()]);
        while let Some(parent) = queue.pop_front() {
            for child in edges.iter().filter(|edge| {
                edge.edge_type == LoomEdgeType::SubTag && edge.target_block_id == parent
            }) {
                if tag_ids.insert(child.source_block_id.clone()) {
                    queue.push_back(child.source_block_id.clone());
                }
            }
        }
    }
    let tagged_block_ids: HashSet<_> = edges
        .iter()
        .filter(|edge| {
            edge.edge_type == LoomEdgeType::Tag && tag_ids.contains(&edge.target_block_id)
        })
        .map(|edge| edge.source_block_id.as_str())
        .collect();
    let mut blocks: Vec<_> = workspace_blocks(db, workspace_id)
        .await?
        .into_iter()
        .filter(|block| tagged_block_ids.contains(block.block_id.as_str()))
        .collect();
    blocks.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.block_id.cmp(&right.block_id))
    });
    Ok(blocks
        .into_iter()
        .skip(offset as usize)
        .take(limit.clamp(1, 500) as usize)
        .collect())
}

fn build_loom_mutation_event(
    workspace_id: &str,
    aggregate_kind: &'static str,
    aggregate_id: &str,
    operation: &str,
    detail: JsonValue,
) -> StorageResult<NewKernelEvent> {
    let (event_type, actor_id, source_component, schema_id, payload_type) =
        if aggregate_kind == "loom_folder" {
            (
                KernelEventType::KnowledgeLoomFolderMutated,
                "loom-folder",
                "loom_folder",
                "hsk.loom_folder_mutation@1",
                "knowledge_loom_folder_mutated",
            )
        } else {
            (
                KernelEventType::KnowledgeLoomBlockMutated,
                "loom-block",
                "loom_block",
                "hsk.loom_block_mutation@1",
                "knowledge_loom_block_mutated",
            )
        };
    let run_id = format!(
        "LOOM-{}-{workspace_id}",
        if aggregate_kind == "loom_folder" {
            "FOLDER"
        } else {
            "BLOCK"
        }
    );
    let mut payload = json!({
        "type": payload_type,
        "schema_id": schema_id,
        "workspace_id": workspace_id,
        "operation": operation,
    });
    if let JsonValue::Object(map) = &mut payload {
        map.insert(
            if aggregate_kind == "loom_folder" {
                "folder_id"
            } else {
                "block_id"
            }
            .to_owned(),
            JsonValue::String(aggregate_id.to_owned()),
        );
        if let JsonValue::Object(detail) = detail {
            map.extend(detail);
        }
    }
    NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        event_type,
        KernelActor::System(actor_id.to_owned()),
    )
    .aggregate(aggregate_kind, aggregate_id.to_owned())
    .source_component(source_component)
    .payload(payload)
    .build()
    .map_err(|_| StorageError::Validation("loom mutation event build failed"))
}

#[derive(SurrealValue)]
struct PinMutationBinding {
    block: RecordId,
    workspace: RecordId,
    pin_order: Option<i64>,
    pinned: Option<bool>,
    actor_kind: String,
    actor_id: Option<String>,
    job_id: Option<String>,
    workflow_id: Option<String>,
    edit_event_id: String,
    updated_at: Datetime,
    ledger: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct EventRecordBinding {
    record: RecordId,
}

#[derive(SurrealValue)]
struct MutationEventRow {
    event_id: String,
    event_sequence: i64,
    created_at: Datetime,
}

async fn mutate_pin(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
    pin_order: Option<i32>,
    pinned: Option<bool>,
    operation: &str,
    metadata: MutationMetadata,
) -> StorageResult<(LoomBlock, LoomMutationEventReceipt)> {
    require_guarded_resource(&metadata, block_id)?;
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    let event = build_loom_mutation_event(
        workspace_id,
        "loom_block",
        block_id,
        operation,
        json!({ "fields_changed": if pinned.is_some() { vec!["pin_order", "pinned"] } else { vec!["pin_order"] } }),
    )?;
    let (_, ledger) = event_ledger::prepare_event(event)?;
    let event_record = ledger.record.clone();
    let rows = db
        .query_values_at::<BlockRow, _>(
            "BEGIN TRANSACTION; \
             IF (SELECT VALUE id FROM $block WHERE workspace_id = $workspace LIMIT 1)[0] = NONE { THROW 'HSK-LOOM-NOT-FOUND'; }; \
             IF (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $ledger.idempotency_key LIMIT 1)[0] = NONE { \
                CREATE $ledger.record CONTENT { event_id: $ledger.event_id, event_version: $ledger.event_version, kernel_task_run_id: $ledger.kernel_task_run_id, session_run_id: $ledger.session_run_id, aggregate_type: $ledger.aggregate_type, aggregate_id: $ledger.aggregate_id, idempotency_key: $ledger.idempotency_key, event_type: $ledger.event_type, actor_kind: $ledger.actor_kind, actor_id: $ledger.actor_id, causation_id: $ledger.causation_id, correlation_id: $ledger.correlation_id, payload_hash: $ledger.payload_hash, source_component: $ledger.source_component, payload: $ledger.payload, created_at: $ledger.created_at }; \
             }; \
             UPDATE $block SET pin_order = $pin_order, pinned = $pinned ?? pinned, last_actor_kind = $actor_kind, last_actor_id = $actor_id, last_job_id = $job_id, last_workflow_id = $workflow_id, edit_event_id = $edit_event_id, updated_at = $updated_at, event_ledger_event_id = $ledger.record RETURN AFTER; \
             COMMIT TRANSACTION;",
            PinMutationBinding {
                block: thing(BLOCKS_TABLE, block_id),
                workspace: thing("workspaces", workspace_id),
                pin_order: pin_order.map(i64::from),
                pinned,
                actor_kind: metadata.actor_kind.as_str().to_owned(),
                actor_id: metadata.actor_id,
                job_id: metadata.job_id.map(|id| id.to_string()),
                workflow_id: metadata.workflow_id.map(|id| id.to_string()),
                edit_event_id: metadata.edit_event_id.to_string(),
                updated_at: Datetime::from(metadata.timestamp),
                ledger,
            },
            3,
        )
        .await
        .map_err(guarded_err)?;
    let block = rows
        .into_iter()
        .next()
        .ok_or(StorageError::NotFound("loom_block"))
        .and_then(block_to_domain)?;
    let event: Option<MutationEventRow> = db
        .query_first(
            "SELECT event_id, event_sequence, created_at FROM $record;",
            EventRecordBinding {
                record: event_record,
            },
        )
        .await
        .map_err(guarded_err)?;
    let event = event.ok_or_else(|| {
        StorageError::Database("committed Loom mutation EventLedger row is missing".to_owned())
    })?;
    Ok((
        block,
        LoomMutationEventReceipt {
            event_id: event.event_id,
            event_sequence: event.event_sequence,
            created_at: event.created_at.into_inner(),
        },
    ))
}

pub(crate) async fn set_loom_block_pin_order(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
    pin_order: Option<i32>,
    metadata: MutationMetadata,
) -> StorageResult<LoomBlock> {
    mutate_pin(
        db,
        workspace_id,
        block_id,
        pin_order,
        None,
        if pin_order.is_some() {
            "pin_order_set"
        } else {
            "pin_order_clear"
        },
        metadata,
    )
    .await
    .map(|(block, _)| block)
}

pub(crate) async fn remove_loom_block_pin(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
    metadata: MutationMetadata,
) -> StorageResult<LoomBlockMutationReceipt> {
    mutate_pin(
        db,
        workspace_id,
        block_id,
        None,
        Some(false),
        "pin_removed",
        metadata,
    )
    .await
    .map(|(block, event)| LoomBlockMutationReceipt { block, event })
}

#[derive(Clone, SurrealValue)]
struct FolderRow {
    folder_id: String,
    workspace_id: RecordId,
    parent_folder_id: Option<RecordId>,
    name: String,
    color: Option<String>,
    sort_mode: String,
    sort_order: Option<i64>,
    project_ref: Option<String>,
    created_at: Datetime,
    updated_at: Datetime,
}

fn folder_to_domain(row: FolderRow) -> StorageResult<LoomFolder> {
    Ok(LoomFolder {
        folder_id: row.folder_id,
        workspace_id: record_key(row.workspace_id, "workspaces")?,
        parent_folder_id: opt_record_key(row.parent_folder_id, "loom_folders")?,
        name: row.name,
        color: row.color,
        sort_mode: LoomFolderSortMode::from_str(&row.sort_mode)?,
        sort_order: row
            .sort_order
            .map(i32::try_from)
            .transpose()
            .map_err(|_| StorageError::Serialization("folder sort_order exceeds i32".to_owned()))?,
        project_ref: row.project_ref,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct FolderCreateBinding {
    folder: RecordId,
    workspace: RecordId,
    parent: Option<RecordId>,
    name: String,
    color: Option<String>,
    sort_mode: String,
    sort_order: Option<i64>,
    project_ref: Option<String>,
    ledger: event_ledger::LedgerWrite,
}

pub(crate) async fn create_loom_folder(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    folder: NewLoomFolder,
) -> StorageResult<LoomFolder> {
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    let name = folder.name.trim();
    if name.is_empty() {
        return Err(StorageError::Validation("loom folder name is required"));
    }
    if let Some(parent_id) = folder.parent_folder_id.as_deref() {
        get_loom_folder(db, workspace_id, parent_id).await?;
    }
    let folder_id = folder
        .folder_id
        .unwrap_or_else(|| format!("LFD-{}", Uuid::now_v7().simple()));
    let event =
        build_loom_mutation_event(workspace_id, "loom_folder", &folder_id, "create", json!({}))?;
    let (_, ledger) = event_ledger::prepare_event(event)?;
    let rows = db
        .query_values_at::<FolderRow, _>(
            "BEGIN TRANSACTION; \
             IF (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $ledger.idempotency_key LIMIT 1)[0] = NONE { \
                CREATE $ledger.record CONTENT { event_id: $ledger.event_id, event_version: $ledger.event_version, kernel_task_run_id: $ledger.kernel_task_run_id, session_run_id: $ledger.session_run_id, aggregate_type: $ledger.aggregate_type, aggregate_id: $ledger.aggregate_id, idempotency_key: $ledger.idempotency_key, event_type: $ledger.event_type, actor_kind: $ledger.actor_kind, actor_id: $ledger.actor_id, causation_id: $ledger.causation_id, correlation_id: $ledger.correlation_id, payload_hash: $ledger.payload_hash, source_component: $ledger.source_component, payload: $ledger.payload, created_at: $ledger.created_at }; \
             }; \
             CREATE $folder SET folder_id = record::id($folder), workspace_id = $workspace, parent_folder_id = $parent, name = $name, color = $color, sort_mode = $sort_mode, sort_order = $sort_order, project_ref = $project_ref, event_ledger_event_id = $ledger.record RETURN AFTER; \
             COMMIT TRANSACTION;",
            FolderCreateBinding {
                folder: thing("loom_folders", folder_id),
                workspace: thing("workspaces", workspace_id),
                parent: folder.parent_folder_id.map(|id| thing("loom_folders", id)),
                name: name.to_owned(),
                color: folder.color.map(|value| value.trim().to_owned()),
                sort_mode: folder.sort_mode.as_str().to_owned(),
                sort_order: folder.sort_order.map(i64::from),
                project_ref: folder.project_ref,
                ledger,
            },
            2,
        )
        .await
        .map_err(guarded_err)?;
    rows.into_iter()
        .next()
        .ok_or_else(|| StorageError::Database("loom folder create returned no row".to_owned()))
        .and_then(folder_to_domain)
}

pub(crate) async fn get_loom_folder(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    folder_id: &str,
) -> StorageResult<LoomFolder> {
    db.query_first::<FolderRow, _>(
        "SELECT * FROM $record WHERE workspace_id = $workspace LIMIT 1;",
        WorkspaceRecordBinding {
            workspace: thing("workspaces", workspace_id),
            record: thing("loom_folders", folder_id),
        },
    )
    .await
    .map_err(map_err)?
    .ok_or(StorageError::NotFound("loom_folder"))
    .and_then(folder_to_domain)
}

pub(crate) async fn list_loom_folders(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
) -> StorageResult<Vec<LoomFolder>> {
    let rows = db
        .query_values::<FolderRow, _>(
            "SELECT * FROM loom_folders WHERE workspace_id = $workspace;",
            WorkspaceBinding {
                workspace: thing("workspaces", workspace_id),
            },
        )
        .await
        .map_err(map_err)?;
    let parents: HashMap<_, _> = rows
        .iter()
        .map(|row| {
            Ok((
                row.folder_id.clone(),
                opt_record_key(row.parent_folder_id.clone(), "loom_folders")?,
            ))
        })
        .collect::<StorageResult<_>>()?;
    let depth = |id: &str| {
        let mut current = Some(id.to_owned());
        let mut seen = HashSet::new();
        let mut depth = 0_u32;
        while let Some(candidate) = current {
            if !seen.insert(candidate.clone()) {
                break;
            }
            current = parents.get(&candidate).cloned().flatten();
            if current.is_some() {
                depth += 1;
            }
        }
        depth
    };
    let mut folders = rows
        .into_iter()
        .map(folder_to_domain)
        .collect::<StorageResult<Vec<_>>>()?;
    folders.sort_by(|left, right| {
        depth(&left.folder_id)
            .cmp(&depth(&right.folder_id))
            .then_with(|| left.sort_order.is_none().cmp(&right.sort_order.is_none()))
            .then_with(|| left.sort_order.cmp(&right.sort_order))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.folder_id.cmp(&right.folder_id))
    });
    Ok(folders)
}

#[derive(SurrealValue)]
struct FolderUpdateBinding {
    folder: RecordId,
    workspace: RecordId,
    name: Option<String>,
    set_color: bool,
    color: Option<String>,
    sort_mode: Option<String>,
    set_sort_order: bool,
    sort_order: Option<i64>,
    set_parent: bool,
    parent: Option<RecordId>,
    set_project_ref: bool,
    project_ref: Option<String>,
    ledger: event_ledger::LedgerWrite,
}

pub(crate) async fn update_loom_folder(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    folder_id: &str,
    update: LoomFolderUpdate,
) -> StorageResult<LoomFolder> {
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    get_loom_folder(db, workspace_id, folder_id).await?;
    if let Some(Some(parent_id)) = update.parent_folder_id.as_ref() {
        if parent_id == folder_id {
            return Err(StorageError::Validation(
                "loom folder cannot be its own parent",
            ));
        }
        get_loom_folder(db, workspace_id, parent_id).await?;
        let folders = list_loom_folders(db, workspace_id).await?;
        let parents: HashMap<_, _> = folders
            .into_iter()
            .map(|folder| (folder.folder_id, folder.parent_folder_id))
            .collect();
        let mut current = Some(parent_id.clone());
        let mut seen = HashSet::new();
        while let Some(candidate) = current {
            if candidate == folder_id {
                return Err(StorageError::Validation(
                    "loom folder move would create a cycle",
                ));
            }
            if !seen.insert(candidate.clone()) {
                return Err(StorageError::Validation(
                    "loom folder hierarchy contains a cycle",
                ));
            }
            current = parents.get(&candidate).cloned().flatten();
        }
    }
    let name = update.name.as_deref().map(str::trim);
    if name.is_some_and(str::is_empty) {
        return Err(StorageError::Validation("loom folder name is required"));
    }
    let event =
        build_loom_mutation_event(workspace_id, "loom_folder", folder_id, "update", json!({}))?;
    let (_, ledger) = event_ledger::prepare_event(event)?;
    let rows = db
        .query_values_at::<FolderRow, _>(
            "BEGIN TRANSACTION; \
             IF (SELECT VALUE id FROM $folder WHERE workspace_id = $workspace LIMIT 1)[0] = NONE { THROW 'HSK-LOOM-FOLDER-NOT-FOUND'; }; \
             IF (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $ledger.idempotency_key LIMIT 1)[0] = NONE { \
                CREATE $ledger.record CONTENT { event_id: $ledger.event_id, event_version: $ledger.event_version, kernel_task_run_id: $ledger.kernel_task_run_id, session_run_id: $ledger.session_run_id, aggregate_type: $ledger.aggregate_type, aggregate_id: $ledger.aggregate_id, idempotency_key: $ledger.idempotency_key, event_type: $ledger.event_type, actor_kind: $ledger.actor_kind, actor_id: $ledger.actor_id, causation_id: $ledger.causation_id, correlation_id: $ledger.correlation_id, payload_hash: $ledger.payload_hash, source_component: $ledger.source_component, payload: $ledger.payload, created_at: $ledger.created_at }; \
             }; \
             UPDATE $folder SET name = $name ?? name, color = IF $set_color { $color } ELSE { color }, sort_mode = $sort_mode ?? sort_mode, sort_order = IF $set_sort_order { $sort_order } ELSE { sort_order }, parent_folder_id = IF $set_parent { $parent } ELSE { parent_folder_id }, project_ref = IF $set_project_ref { $project_ref } ELSE { project_ref }, event_ledger_event_id = $ledger.record, updated_at = time::now() RETURN AFTER; \
             COMMIT TRANSACTION;",
            FolderUpdateBinding {
                folder: thing("loom_folders", folder_id),
                workspace: thing("workspaces", workspace_id),
                name: name.map(str::to_owned),
                set_color: update.color.is_some(),
                color: update
                    .color
                    .clone()
                    .flatten()
                    .map(|value| value.trim().to_owned()),
                sort_mode: update.sort_mode.map(|mode| mode.as_str().to_owned()),
                set_sort_order: update.sort_order.is_some(),
                sort_order: update.sort_order.flatten().map(i64::from),
                set_parent: update.parent_folder_id.is_some(),
                parent: update
                    .parent_folder_id
                    .clone()
                    .flatten()
                    .map(|id| thing("loom_folders", id)),
                set_project_ref: update.project_ref.is_some(),
                project_ref: update.project_ref.flatten(),
                ledger,
            },
            3,
        )
        .await
        .map_err(guarded_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::NotFound("loom_folder"))
        .and_then(folder_to_domain)
}

#[derive(SurrealValue)]
struct FolderDeleteBinding {
    folder: RecordId,
    workspace: RecordId,
    ledger: event_ledger::LedgerWrite,
}

pub(crate) async fn delete_loom_folder(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    folder_id: &str,
) -> StorageResult<()> {
    let event =
        build_loom_mutation_event(workspace_id, "loom_folder", folder_id, "delete", json!({}))?;
    let (_, ledger) = event_ledger::prepare_event(event)?;
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    db.query_values_at::<JsonValue, _>(
        "BEGIN TRANSACTION; \
         LET $deleted = (DELETE $folder WHERE workspace_id = $workspace RETURN BEFORE); \
         IF array::len($deleted) = 0 { THROW 'HSK-LOOM-FOLDER-NOT-FOUND'; }; \
         IF (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $ledger.idempotency_key LIMIT 1)[0] = NONE { \
            CREATE $ledger.record CONTENT { event_id: $ledger.event_id, event_version: $ledger.event_version, kernel_task_run_id: $ledger.kernel_task_run_id, session_run_id: $ledger.session_run_id, aggregate_type: $ledger.aggregate_type, aggregate_id: $ledger.aggregate_id, idempotency_key: $ledger.idempotency_key, event_type: $ledger.event_type, actor_kind: $ledger.actor_kind, actor_id: $ledger.actor_id, causation_id: $ledger.causation_id, correlation_id: $ledger.correlation_id, payload_hash: $ledger.payload_hash, source_component: $ledger.source_component, payload: $ledger.payload, created_at: $ledger.created_at }; \
         }; \
         COMMIT TRANSACTION;",
        FolderDeleteBinding {
            folder: thing("loom_folders", folder_id),
            workspace: thing("workspaces", workspace_id),
            ledger,
        },
        4,
    )
    .await
    .map_err(guarded_err)?;
    Ok(())
}

#[derive(SurrealValue)]
struct FolderMemberBinding {
    member: RecordId,
    folder: RecordId,
    block: RecordId,
    workspace: RecordId,
    sort_order: Option<i64>,
    ledger: event_ledger::LedgerWrite,
}

pub(crate) async fn add_block_to_loom_folder(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    folder_id: &str,
    block_id: &str,
    sort_order: Option<i32>,
) -> StorageResult<()> {
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    get_loom_folder(db, workspace_id, folder_id).await?;
    get_loom_block(db, workspace_id, block_id).await?;
    let event = build_loom_mutation_event(
        workspace_id,
        "loom_folder",
        folder_id,
        "add_member",
        json!({ "block_id": block_id }),
    )?;
    let (_, ledger) = event_ledger::prepare_event(event)?;
    db.query_values_at::<JsonValue, _>(
        "BEGIN TRANSACTION; \
         IF (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $ledger.idempotency_key LIMIT 1)[0] = NONE { \
            CREATE $ledger.record CONTENT { event_id: $ledger.event_id, event_version: $ledger.event_version, kernel_task_run_id: $ledger.kernel_task_run_id, session_run_id: $ledger.session_run_id, aggregate_type: $ledger.aggregate_type, aggregate_id: $ledger.aggregate_id, idempotency_key: $ledger.idempotency_key, event_type: $ledger.event_type, actor_kind: $ledger.actor_kind, actor_id: $ledger.actor_id, causation_id: $ledger.causation_id, correlation_id: $ledger.correlation_id, payload_hash: $ledger.payload_hash, source_component: $ledger.source_component, payload: $ledger.payload, created_at: $ledger.created_at }; \
         }; \
         UPSERT $member SET folder_id = $folder, block_id = $block, workspace_id = $workspace, sort_order = $sort_order, event_ledger_event_id = $ledger.record; \
         COMMIT TRANSACTION;",
        FolderMemberBinding {
            member: thing(
                "loom_folder_members",
                format!("{}:{folder_id}--{}:{block_id}", folder_id.len(), block_id.len()),
            ),
            folder: thing("loom_folders", folder_id),
            block: thing(BLOCKS_TABLE, block_id),
            workspace: thing("workspaces", workspace_id),
            sort_order: sort_order.map(i64::from),
            ledger,
        },
        3,
    )
    .await
    .map_err(map_err)?;
    Ok(())
}

pub(crate) async fn remove_block_from_loom_folder(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    folder_id: &str,
    block_id: &str,
) -> StorageResult<()> {
    let event = build_loom_mutation_event(
        workspace_id,
        "loom_folder",
        folder_id,
        "remove_member",
        json!({ "block_id": block_id }),
    )?;
    let (_, ledger) = event_ledger::prepare_event(event)?;
    let _guard = LOOM_MUTATION_LOCK.lock().await;
    db.query_values_at::<JsonValue, _>(
        "BEGIN TRANSACTION; \
         LET $deleted = (DELETE loom_folder_members WHERE workspace_id = $workspace AND folder_id = $folder AND block_id = $block RETURN BEFORE); \
         IF array::len($deleted) > 0 AND (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $ledger.idempotency_key LIMIT 1)[0] = NONE { \
            CREATE $ledger.record CONTENT { event_id: $ledger.event_id, event_version: $ledger.event_version, kernel_task_run_id: $ledger.kernel_task_run_id, session_run_id: $ledger.session_run_id, aggregate_type: $ledger.aggregate_type, aggregate_id: $ledger.aggregate_id, idempotency_key: $ledger.idempotency_key, event_type: $ledger.event_type, actor_kind: $ledger.actor_kind, actor_id: $ledger.actor_id, causation_id: $ledger.causation_id, correlation_id: $ledger.correlation_id, payload_hash: $ledger.payload_hash, source_component: $ledger.source_component, payload: $ledger.payload, created_at: $ledger.created_at }; \
         }; \
         COMMIT TRANSACTION;",
        FolderMemberBinding {
            member: thing(
                "loom_folder_members",
                format!("{}:{folder_id}--{}:{block_id}", folder_id.len(), block_id.len()),
            ),
            folder: thing("loom_folders", folder_id),
            block: thing(BLOCKS_TABLE, block_id),
            workspace: thing("workspaces", workspace_id),
            sort_order: None,
            ledger,
        },
        3,
    )
    .await
    .map_err(map_err)?;
    Ok(())
}

#[derive(SurrealValue)]
struct FolderMemberRow {
    block_id: RecordId,
    sort_order: Option<i64>,
}

pub(crate) async fn list_loom_folder_blocks(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    folder_id: &str,
    limit: u32,
    offset: u32,
) -> StorageResult<Vec<LoomBlock>> {
    let folder = get_loom_folder(db, workspace_id, folder_id).await?;
    let members = db
        .query_values::<FolderMemberRow, _>(
            "SELECT block_id, sort_order FROM loom_folder_members WHERE workspace_id = $workspace AND folder_id = $record;",
            WorkspaceRecordBinding {
                workspace: thing("workspaces", workspace_id),
                record: thing("loom_folders", folder_id),
            },
        )
        .await
        .map_err(map_err)?;
    let order_by_block: HashMap<_, _> = members
        .into_iter()
        .map(|member| {
            Ok((
                record_key(member.block_id, BLOCKS_TABLE)?,
                member.sort_order,
            ))
        })
        .collect::<StorageResult<_>>()?;
    let mut blocks: Vec<_> = workspace_blocks(db, workspace_id)
        .await?
        .into_iter()
        .filter(|block| order_by_block.contains_key(&block.block_id))
        .collect();
    blocks.sort_by(|left, right| match folder.sort_mode {
        LoomFolderSortMode::NameAsc => left
            .title
            .as_deref()
            .unwrap_or_default()
            .cmp(right.title.as_deref().unwrap_or_default())
            .then_with(|| left.block_id.cmp(&right.block_id)),
        LoomFolderSortMode::NameDesc => right
            .title
            .as_deref()
            .unwrap_or_default()
            .cmp(left.title.as_deref().unwrap_or_default())
            .then_with(|| left.block_id.cmp(&right.block_id)),
        LoomFolderSortMode::CreatedDesc => right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.block_id.cmp(&right.block_id)),
        LoomFolderSortMode::UpdatedDesc => right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.block_id.cmp(&right.block_id)),
        LoomFolderSortMode::Manual => {
            let left_order = order_by_block[&left.block_id];
            let right_order = order_by_block[&right.block_id];
            left_order
                .is_none()
                .cmp(&right_order.is_none())
                .then_with(|| left_order.cmp(&right_order))
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.block_id.cmp(&right.block_id))
        }
    });
    Ok(blocks
        .into_iter()
        .skip(offset as usize)
        .take(limit.clamp(1, 500) as usize)
        .collect())
}

#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(SurrealValue)]
struct TestMetricBindings {
    workspace: RecordId,
    record: RecordId,
    mention_count: i64,
    tag_count: i64,
    backlink_count: i64,
}

#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(SurrealValue)]
struct TestWorkspaceBinding {
    workspace: RecordId,
}

#[cfg(any(test, feature = "surreal-test-support"))]
pub(crate) async fn test_overwrite_loom_block_metrics(
    storage: &super::SurrealStorage,
    workspace_id: &str,
    block_id: &str,
    mention_count: i64,
    tag_count: i64,
    backlink_count: i64,
) -> StorageResult<()> {
    if mention_count < 0 || tag_count < 0 || backlink_count < 0 {
        return Err(StorageError::Validation(
            "loom test metrics must be non-negative",
        ));
    }
    let workspace_id = workspace_id.to_owned();
    let block_id = block_id.to_owned();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        "UPDATE $record SET mention_count = $mention_count, tag_count = $tag_count, backlink_count = $backlink_count WHERE workspace_id = $workspace RETURN AFTER;",
                        TestMetricBindings {
                            workspace: thing("workspaces", workspace_id),
                            record: thing(BLOCKS_TABLE, block_id),
                            mention_count,
                            tag_count,
                            backlink_count,
                        },
                    )
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(map_err)
}

#[cfg(any(test, feature = "surreal-test-support"))]
pub(crate) async fn test_zero_workspace_loom_metrics(
    storage: &super::SurrealStorage,
    workspace_id: &str,
) -> StorageResult<()> {
    let workspace_id = workspace_id.to_owned();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        "UPDATE loom_blocks SET mention_count = 0, tag_count = 0, backlink_count = 0 WHERE workspace_id = $workspace RETURN AFTER;",
                        TestWorkspaceBinding {
                            workspace: thing("workspaces", workspace_id),
                        },
                    )
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(map_err)
}

#[cfg(any(test, feature = "test-utils", feature = "surreal-test-support"))]
#[derive(SurrealValue)]
struct PerfFixtureBlock {
    record: RecordId,
    block_id: String,
    title: String,
    edit_event_id: String,
}

#[cfg(any(test, feature = "test-utils", feature = "surreal-test-support"))]
#[derive(SurrealValue)]
struct PerfFixtureEdge {
    record: RecordId,
    edge_id: String,
    source: RecordId,
    target: RecordId,
    edit_event_id: String,
}

#[cfg(any(test, feature = "test-utils", feature = "surreal-test-support"))]
#[derive(SurrealValue)]
struct PerfFixtureBindings {
    workspace: RecordId,
    blocks: Vec<PerfFixtureBlock>,
    edges: Vec<PerfFixtureEdge>,
    now: Datetime,
}

#[cfg(any(test, feature = "test-utils", feature = "surreal-test-support"))]
pub(crate) async fn test_insert_loom_traversal_perf_fixture(
    storage: &super::SurrealStorage,
    workspace_id: &str,
    total_blocks: usize,
) -> StorageResult<String> {
    if total_blocks == 0 {
        return Err(StorageError::Validation(
            "loom traversal perf fixture requires at least one block",
        ));
    }
    let start_block_id = "perf-block-00000".to_owned();
    let blocks = (0..total_blocks)
        .map(|index| {
            let block_id = format!("perf-block-{index:05}");
            PerfFixtureBlock {
                record: thing(BLOCKS_TABLE, block_id.clone()),
                block_id,
                title: format!("Perf Block {index}"),
                edit_event_id: Uuid::now_v7().to_string(),
            }
        })
        .collect::<Vec<_>>();
    let edges = (1..total_blocks)
        .map(|index| {
            let edge_id = format!("perf-edge-{index:05}");
            PerfFixtureEdge {
                record: thing(EDGES_TABLE, edge_id.clone()),
                edge_id,
                source: thing(BLOCKS_TABLE, format!("perf-block-{:05}", index - 1)),
                target: thing(BLOCKS_TABLE, format!("perf-block-{index:05}")),
                edit_event_id: Uuid::now_v7().to_string(),
            }
        })
        .collect::<Vec<_>>();
    let bindings = PerfFixtureBindings {
        workspace: thing("workspaces", workspace_id.to_owned()),
        blocks,
        edges,
        now: Datetime::from(Utc::now()),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        "BEGIN TRANSACTION; \
                         IF record::exists($workspace) = false { THROW 'HSK-LOOM-WORKSPACE-MISSING'; }; \
                         FOR $block IN $blocks { \
                           CREATE $block.record SET block_id = $block.block_id, workspace_id = $workspace, \
                             content_type = 'note', title = $block.title, pinned = false, favorite = false, \
                             edit_event_id = $block.edit_event_id, last_actor_kind = 'SYSTEM', \
                             created_at = $now, updated_at = $now, backlink_count = 0, mention_count = 0, \
                             tag_count = 0, derived_json = {}, preview_status = 'none'; \
                         }; \
                         FOR $edge IN $edges { \
                           CREATE $edge.record SET edge_id = $edge.edge_id, workspace_id = $workspace, \
                             source_block_id = $edge.source, target_block_id = $edge.target, \
                             edge_type = 'mention', created_by = 'user', edit_event_id = $edge.edit_event_id, \
                             last_actor_kind = 'SYSTEM', created_at = $now; \
                         }; \
                         COMMIT TRANSACTION;",
                        bindings,
                    )
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(|error| {
            if error.to_string().contains("HSK-LOOM-WORKSPACE-MISSING") {
                StorageError::NotFound("workspace")
            } else {
                map_err(error)
            }
        })?;
    Ok(start_block_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::surreal::{SurrealStorage, SurrealStorageConfig};
    use crate::storage::WriteActorKind;

    #[derive(SurrealValue)]
    struct WorkspaceSeed {
        name: String,
    }

    #[derive(SurrealValue)]
    struct RecordBinding {
        record: RecordId,
    }

    #[derive(SurrealValue)]
    struct SearchProjectionRow {
        search_text: String,
    }

    #[derive(SurrealValue)]
    struct MutationIdentityProofRow {
        event_id: String,
        event_sequence: i64,
        payload: JsonValue,
    }

    #[derive(SurrealValue)]
    struct BlockEventLinkProofRow {
        event_ledger_event_id: RecordId,
    }

    #[derive(SurrealValue)]
    struct KnowledgeEntitySearchSeed {
        entity_id: String,
        workspace_id: RecordId,
        entity_kind: String,
        entity_key: String,
        display_name: String,
        detection_provenance: JsonValue,
    }

    #[derive(SurrealValue)]
    struct RichDocumentSearchSeed {
        rich_document_id: String,
        workspace_id: RecordId,
        title: String,
        schema_version: String,
        content_json: JsonValue,
        content_sha256: String,
    }

    #[derive(SurrealValue)]
    struct UserManualSearchSeed {
        page_id: String,
        slug: String,
        title: String,
        page_kind: String,
        body: JsonValue,
        content_hash: String,
        manual_version: String,
    }

    #[derive(SurrealValue)]
    struct WikiPageSearchSeed {
        projection_id: String,
        workspace_id: RecordId,
        projection_kind: String,
        title: String,
        rendered_content: String,
        staleness_hash: String,
    }

    fn metadata(resource_id: &str, timestamp: DateTime<Utc>) -> MutationMetadata {
        MutationMetadata {
            actor_kind: WriteActorKind::System,
            actor_id: Some("loom-store-test".to_owned()),
            job_id: None,
            workflow_id: None,
            edit_event_id: Uuid::now_v7(),
            resource_id: resource_id.to_owned(),
            timestamp,
        }
    }

    async fn open_store(temp: &tempfile::TempDir) -> (SurrealStorageConfig, SurrealStorage) {
        let config = SurrealStorageConfig::for_data_dir(temp.path())
            .expect("configure real embedded Surreal store");
        let store = SurrealStorage::open(config.clone())
            .await
            .expect("open real embedded Surreal store");
        (config, store)
    }

    async fn seed_workspace(store: &SurrealStorage, workspace_id: &str) {
        let workspace_id = workspace_id.to_owned();
        store
            .with_data_operation(move |db| {
                Box::pin(async move {
                    let _: Option<surrealdb::types::Value> = db
                        .upsert_one(
                            "workspaces",
                            &workspace_id,
                            WorkspaceSeed {
                                name: "Loom test workspace".to_owned(),
                            },
                        )
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed workspace");
    }

    fn new_asset(workspace_id: &str, content_hash: &str) -> NewAsset {
        NewAsset {
            workspace_id: workspace_id.to_owned(),
            kind: "image".to_owned(),
            mime: "image/png".to_owned(),
            original_filename: Some(format!("{content_hash}.png")),
            content_hash: content_hash.to_owned(),
            size_bytes: 64,
            width: Some(8),
            height: Some(8),
            classification: "low".to_owned(),
            exportable: true,
            is_proxy_of: None,
            proxy_asset_id: None,
        }
    }

    fn new_block(workspace_id: &str, block_id: &str, title: &str) -> NewLoomBlock {
        let mut derived = LoomBlockDerived::default();
        derived.full_text_index = Some(format!("shared searchable text for {title}"));
        NewLoomBlock {
            block_id: Some(block_id.to_owned()),
            workspace_id: workspace_id.to_owned(),
            content_type: LoomBlockContentType::Note,
            document_id: None,
            asset_id: None,
            title: Some(title.to_owned()),
            original_filename: None,
            content_hash: Some(format!("hash-{block_id}")),
            pinned: false,
            journal_date: None,
            imported_at: None,
            derived,
        }
    }

    async fn create_test_asset(
        store: &SurrealStorage,
        workspace_id: &str,
        asset_id: &str,
        content_hash: &str,
    ) {
        let asset_id = asset_id.to_owned();
        let asset = new_asset(workspace_id, content_hash);
        let write_metadata = metadata(&asset_id, Utc::now());
        store
            .with_storage_operation(move |db| {
                Box::pin(async move { create_asset(&db, asset_id, asset, write_metadata).await })
            })
            .await
            .expect("asset lifecycle operation")
            .expect("create asset");
    }

    async fn create_test_block(
        store: &SurrealStorage,
        workspace_id: &str,
        block_id: &str,
        title: &str,
        timestamp: DateTime<Utc>,
    ) -> LoomBlock {
        let block = new_block(workspace_id, block_id, title);
        let write_metadata = metadata(block_id, timestamp);
        store
            .with_storage_operation(move |db| {
                Box::pin(async move { create_loom_block(&db, block, write_metadata).await })
            })
            .await
            .expect("block lifecycle operation")
            .expect("create block")
    }

    #[tokio::test]
    async fn concurrent_pin_removals_return_their_exact_committed_event_identity() {
        let temp = tempfile::tempdir().expect("create temporary data root");
        let (_, store) = open_store(&temp).await;
        super::super::schema::bootstrap_loom_receipt_test_schema(&store)
            .await
            .expect("bootstrap production Loom receipt schema");
        let workspace_id = "loom-pin-receipt-workspace";
        let block_id = "concurrent-pin";
        seed_workspace(&store, workspace_id).await;
        create_test_block(&store, workspace_id, block_id, "Concurrent pin", Utc::now()).await;

        let seed_workspace_id = workspace_id.to_owned();
        let seed_block_id = block_id.to_owned();
        store
            .with_storage_operation(move |db| {
                Box::pin(async move {
                    update_loom_block(
                        &db,
                        &seed_workspace_id,
                        &seed_block_id,
                        LoomBlockUpdate {
                            pinned: Some(true),
                            pin_order: Some(0),
                            ..LoomBlockUpdate::default()
                        },
                        metadata(&seed_block_id, Utc::now()),
                    )
                    .await
                })
            })
            .await
            .expect("seed pin lifecycle")
            .expect("seed pinned block");

        let remove = |store: SurrealStorage, actor: &'static str| {
            let workspace_id = workspace_id.to_owned();
            let block_id = block_id.to_owned();
            async move {
                store
                    .with_storage_operation(move |db| {
                        let mut write_metadata = metadata(&block_id, Utc::now());
                        write_metadata.actor_id = Some(actor.to_owned());
                        Box::pin(async move {
                            remove_loom_block_pin(&db, &workspace_id, &block_id, write_metadata)
                                .await
                        })
                    })
                    .await
                    .expect("pin removal lifecycle")
                    .expect("remove pin")
            }
        };
        let (first, second) = tokio::join!(
            remove(store.clone(), "concurrent-removal-a"),
            remove(store.clone(), "concurrent-removal-b")
        );

        assert_ne!(first.event.event_id, second.event.event_id);
        assert_ne!(first.event.event_sequence, second.event.event_sequence);
        assert!(!first.block.pinned && first.block.pin_order.is_none());
        assert!(!second.block.pinned && second.block.pin_order.is_none());

        for receipt in [&first, &second] {
            let row = store
                .with_data_operation({
                    let event_id = receipt.event.event_id.clone();
                    move |db| {
                        Box::pin(async move {
                            db.query_first::<MutationIdentityProofRow, _>(
                                "SELECT event_id, event_sequence, payload FROM $record;",
                                RecordBinding {
                                    record: thing("kernel_event_ledger", event_id),
                                },
                            )
                            .await
                        })
                    }
                })
                .await
                .expect("read exact receipt event")
                .expect("receipt event exists");
            assert_eq!(row.event_id, receipt.event.event_id);
            assert_eq!(row.event_sequence, receipt.event.event_sequence);
            assert_eq!(row.payload["operation"], "pin_removed");
            assert_eq!(row.payload["block_id"], block_id);
        }

        let final_receipt = [&first, &second]
            .into_iter()
            .max_by_key(|receipt| receipt.event.event_sequence)
            .expect("one final pin-removal receipt");
        let final_block_link = store
            .with_data_operation({
                let block_id = block_id.to_owned();
                move |db| {
                    Box::pin(async move {
                        db.query_first::<BlockEventLinkProofRow, _>(
                            "SELECT event_ledger_event_id FROM $record;",
                            RecordBinding {
                                record: thing(BLOCKS_TABLE, block_id),
                            },
                        )
                        .await
                    })
                }
            })
            .await
            .expect("read final block EventLedger link")
            .expect("final block exists");
        assert_eq!(
            record_key(
                final_block_link.event_ledger_event_id,
                "kernel_event_ledger"
            )
            .expect("block links the EventLedger table"),
            final_receipt.event.event_id,
            "the final block revision must link the exact last committed removal receipt"
        );

        store.shutdown().await.expect("close embedded store");
    }

    #[tokio::test]
    async fn collection_replacement_is_atomic_and_survives_close_reopen() {
        let temp = tempfile::tempdir().expect("create temporary data root");
        let (config, store) = open_store(&temp).await;
        let workspace_id = "loom-collection-workspace";
        seed_workspace(&store, workspace_id).await;
        create_test_asset(&store, workspace_id, "asset-a", "hash-a").await;
        create_test_asset(&store, workspace_id, "asset-b", "hash-b").await;
        create_test_asset(&store, workspace_id, "asset-c", "hash-c").await;

        let collection_id = "collection-1".to_owned();
        let collection_metadata = metadata(&collection_id, Utc::now());
        let workspace = workspace_id.to_owned();
        let persisted_id = collection_id.clone();
        store
            .with_storage_operation(move |db| {
                Box::pin(async move {
                    create_loom_collection(
                        &db,
                        persisted_id,
                        &workspace,
                        Some("Ordered assets".to_owned()),
                        collection_metadata,
                    )
                    .await
                })
            })
            .await
            .expect("collection lifecycle operation")
            .expect("create collection");

        let workspace = workspace_id.to_owned();
        let collection = collection_id.clone();
        store
            .with_storage_operation(move |db| {
                Box::pin(async move {
                    set_loom_collection_order(
                        &db,
                        &workspace,
                        &collection,
                        &["asset-a".to_owned(), "asset-b".to_owned()],
                    )
                    .await
                })
            })
            .await
            .expect("collection replacement lifecycle")
            .expect("initial collection order");

        // The missing asset violates the record-link assertion after the old
        // members were deleted. The surrounding transaction must restore the
        // prior member set instead of committing an empty/partial collection.
        let workspace = workspace_id.to_owned();
        let collection = collection_id.clone();
        let failed = store
            .with_storage_operation(move |db| {
                Box::pin(async move {
                    set_loom_collection_order(
                        &db,
                        &workspace,
                        &collection,
                        &["asset-c".to_owned(), "missing-asset".to_owned()],
                    )
                    .await
                })
            })
            .await
            .expect("failed collection replacement lifecycle");
        assert!(failed.is_err(), "missing asset must abort the transaction");

        let workspace = workspace_id.to_owned();
        let collection = collection_id.clone();
        let unchanged = store
            .with_storage_operation(move |db| {
                Box::pin(async move { get_loom_collection(&db, &workspace, &collection).await })
            })
            .await
            .expect("read collection lifecycle")
            .expect("read collection after rollback");
        assert_eq!(
            unchanged
                .members
                .iter()
                .map(|member| (member.asset_id.as_str(), member.position))
                .collect::<Vec<_>>(),
            vec![("asset-a", 0), ("asset-b", 1)]
        );

        let workspace = workspace_id.to_owned();
        let collection = collection_id.clone();
        store
            .with_storage_operation(move |db| {
                Box::pin(async move {
                    set_loom_collection_order(
                        &db,
                        &workspace,
                        &collection,
                        &["asset-c".to_owned(), "asset-a".to_owned()],
                    )
                    .await
                })
            })
            .await
            .expect("final collection replacement lifecycle")
            .expect("replace collection order");

        store.shutdown().await.expect("close embedded store");
        drop(store);
        let reopened = SurrealStorage::open(config)
            .await
            .expect("reopen the same embedded store");
        let workspace = workspace_id.to_owned();
        let collection = collection_id.clone();
        let durable = reopened
            .with_storage_operation(move |db| {
                Box::pin(async move { get_loom_collection(&db, &workspace, &collection).await })
            })
            .await
            .expect("reopened collection lifecycle")
            .expect("read durable collection");
        assert_eq!(
            durable
                .members
                .iter()
                .map(|member| (member.asset_id.as_str(), member.position))
                .collect::<Vec<_>>(),
            vec![("asset-c", 0), ("asset-a", 1)]
        );
        reopened.shutdown().await.expect("close reopened store");
    }

    #[tokio::test]
    async fn block_search_projection_update_and_view_ordering_use_real_store() {
        let temp = tempfile::tempdir().expect("create temporary data root");
        let (_, store) = open_store(&temp).await;
        let workspace_id = "loom-search-workspace";
        seed_workspace(&store, workspace_id).await;
        let base = Utc::now();
        create_test_block(&store, workspace_id, "block-a", "Alpha", base).await;
        create_test_block(
            &store,
            workspace_id,
            "block-b",
            "Beta",
            base + chrono::Duration::seconds(1),
        )
        .await;
        create_test_block(
            &store,
            workspace_id,
            "block-c",
            "Gamma",
            base + chrono::Duration::seconds(2),
        )
        .await;

        for (block_id, pin_order, favorite, title, seconds) in [
            ("block-a", 1, false, "Alpha renamed", 3),
            ("block-b", 0, true, "Beta renamed", 4),
        ] {
            let workspace = workspace_id.to_owned();
            let block = block_id.to_owned();
            let update = LoomBlockUpdate {
                title: Some(title.to_owned()),
                pinned: Some(true),
                favorite: Some(favorite),
                pin_order: Some(pin_order),
                ..LoomBlockUpdate::default()
            };
            let write_metadata = metadata(block_id, base + chrono::Duration::seconds(seconds));
            store
                .with_storage_operation(move |db| {
                    Box::pin(async move {
                        update_loom_block(&db, &workspace, &block, update, write_metadata).await
                    })
                })
                .await
                .expect("block update lifecycle")
                .expect("update block");
        }

        let projection = store
            .with_data_operation(|db| {
                Box::pin(async move {
                    db.query_first::<SearchProjectionRow, _>(
                        "SELECT search_text FROM $record;",
                        RecordBinding {
                            record: thing("loom_block_search_index", "block-a"),
                        },
                    )
                    .await
                })
            })
            .await
            .expect("read real search projection")
            .expect("search projection exists");
        assert!(projection.search_text.contains("Alpha renamed"));

        let workspace = workspace_id.to_owned();
        let pins = store
            .with_storage_operation(move |db| {
                Box::pin(async move {
                    query_loom_view(
                        &db,
                        &workspace,
                        LoomViewType::Pins,
                        LoomViewFilters::default(),
                        10,
                        0,
                    )
                    .await
                })
            })
            .await
            .expect("pins view lifecycle")
            .expect("query pins view");
        let LoomViewResponse::Pins { blocks } = pins else {
            panic!("pins query returned a different view variant");
        };
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.block_id.as_str())
                .collect::<Vec<_>>(),
            vec!["block-b", "block-a"]
        );

        let workspace = workspace_id.to_owned();
        let search = store
            .with_storage_operation(move |db| {
                Box::pin(async move {
                    search_loom_blocks(
                        &db,
                        &workspace,
                        "shared searchable",
                        LoomSearchFilters::default(),
                        10,
                        0,
                    )
                    .await
                })
            })
            .await
            .expect("search lifecycle")
            .expect("search blocks");
        assert_eq!(search.len(), 3);
        assert_eq!(search[0].block.block_id, "block-b");
        store.shutdown().await.expect("close embedded store");
    }

    #[tokio::test]
    async fn graph_search_covers_every_declared_source_kind() {
        let temp = tempfile::tempdir().expect("create temporary data root");
        let (_, store) = open_store(&temp).await;
        let workspace_id = "loom-cross-source-workspace";
        seed_workspace(&store, workspace_id).await;
        let base = Utc::now();

        for (index, (block_id, title, content_type)) in [
            (
                "cross-block",
                "cross-source-needle block",
                LoomBlockContentType::Note,
            ),
            (
                "cross-file",
                "cross-source-needle file",
                LoomBlockContentType::File,
            ),
            (
                "cross-tag",
                "cross-source-needle tag",
                LoomBlockContentType::TagHub,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let mut block = new_block(workspace_id, block_id, title);
            block.content_type = content_type;
            let write_metadata = metadata(block_id, base + chrono::Duration::seconds(index as i64));
            store
                .with_storage_operation(move |db| {
                    Box::pin(async move { create_loom_block(&db, block, write_metadata).await })
                })
                .await
                .expect("cross-source block lifecycle")
                .expect("create cross-source block");
        }

        let workspace = workspace_id.to_owned();
        store
            .with_data_operation(move |db| {
                Box::pin(async move {
                    for (entity_id, entity_kind) in [
                        ("cross-symbol", "symbol"),
                        ("cross-work-packet", "work_packet"),
                        ("cross-micro-task", "micro_task"),
                    ] {
                        let _: Option<surrealdb::types::Value> = db
                            .upsert_one(
                                "knowledge_entities",
                                entity_id,
                                KnowledgeEntitySearchSeed {
                                    entity_id: entity_id.to_owned(),
                                    workspace_id: thing("workspaces", workspace.clone()),
                                    entity_kind: entity_kind.to_owned(),
                                    entity_key: format!("cross-source-needle-{entity_kind}"),
                                    display_name: if entity_kind == "symbol" {
                                        "cross-source-needle surrealnav symbol".to_owned()
                                    } else {
                                        format!("cross-source-needle {entity_kind}")
                                    },
                                    detection_provenance: json!({"test": true}),
                                },
                            )
                            .await?;
                    }
                    let _: Option<surrealdb::types::Value> = db
                        .upsert_one(
                            "knowledge_rich_documents",
                            "cross-document",
                            RichDocumentSearchSeed {
                                rich_document_id: "cross-document".to_owned(),
                                workspace_id: thing("workspaces", workspace.clone()),
                                title: "cross-source-needle document".to_owned(),
                                schema_version: "1".to_owned(),
                                content_json: json!({"text": "cross-source-needle"}),
                                content_sha256: "d".repeat(64),
                            },
                        )
                        .await?;
                    let _: Option<surrealdb::types::Value> = db
                        .upsert_one(
                            "user_manual_pages",
                            "cross-manual",
                            UserManualSearchSeed {
                                page_id: "cross-manual".to_owned(),
                                slug: "cross-source-needle-manual".to_owned(),
                                title: "cross-source-needle manual".to_owned(),
                                page_kind: "purpose".to_owned(),
                                body: json!({"text": "cross-source-needle"}),
                                content_hash: "m".repeat(64),
                                manual_version: "1".to_owned(),
                            },
                        )
                        .await?;
                    let _: Option<surrealdb::types::Value> = db
                        .upsert_one(
                            "knowledge_wiki_projections",
                            "cross-wiki",
                            WikiPageSearchSeed {
                                projection_id: "cross-wiki".to_owned(),
                                workspace_id: thing("workspaces", workspace.clone()),
                                projection_kind: "wiki_page".to_owned(),
                                title: "cross-source-needle wiki".to_owned(),
                                rendered_content: "cross-source-needle rendered".to_owned(),
                                staleness_hash: "w".repeat(64),
                            },
                        )
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed every graph-search source");

        let workspace = workspace_id.to_owned();
        let results = store
            .with_storage_operation(move |db| {
                Box::pin(async move {
                    search_loom_graph(
                        &db,
                        &workspace,
                        "cross-source-needle",
                        LoomSearchFilters::default(),
                        20,
                        0,
                    )
                    .await
                })
            })
            .await
            .expect("cross-source search lifecycle")
            .expect("search every source");
        assert_eq!(results.len(), LOOM_GRAPH_SOURCE_ORDER.len());
        let actual: HashSet<&str> = results
            .iter()
            .map(|result| result.source_kind.as_str())
            .collect();
        let expected: HashSet<&str> = LOOM_GRAPH_SOURCE_ORDER
            .iter()
            .map(LoomSearchSourceKind::as_str)
            .collect();
        assert_eq!(actual, expected);

        for source_kind in LOOM_GRAPH_SOURCE_ORDER {
            let workspace = workspace_id.to_owned();
            let filtered = store
                .with_storage_operation(move |db| {
                    Box::pin(async move {
                        search_loom_graph(
                            &db,
                            &workspace,
                            "cross-source-needle",
                            LoomSearchFilters {
                                source_kinds: vec![source_kind],
                                ..LoomSearchFilters::default()
                            },
                            20,
                            0,
                        )
                        .await
                    })
                })
                .await
                .expect("filtered graph-search lifecycle")
                .expect("filter graph-search source");
            assert_eq!(filtered.len(), 1, "source kind {}", source_kind.as_str());
            assert_eq!(filtered[0].source_kind, source_kind);
        }

        let workspace = workspace_id.to_owned();
        let fuzzy = store
            .with_storage_operation(move |db| {
                Box::pin(async move {
                    search_loom_graph(
                        &db,
                        &workspace,
                        "surealnav",
                        LoomSearchFilters {
                            source_kinds: vec![LoomSearchSourceKind::Symbol],
                            ..LoomSearchFilters::default()
                        },
                        10,
                        0,
                    )
                    .await
                })
            })
            .await
            .expect("fuzzy graph-search lifecycle")
            .expect("fuzzy graph-search typo");
        assert_eq!(fuzzy.len(), 1);
        assert_eq!(fuzzy[0].ref_id, "cross-symbol");
        assert!(fuzzy[0].score > 0.0);
        store.shutdown().await.expect("close embedded store");
    }

    #[tokio::test]
    async fn edge_mutations_refresh_metrics_and_graph_traversal() {
        let temp = tempfile::tempdir().expect("create temporary data root");
        let (_, store) = open_store(&temp).await;
        let workspace_id = "loom-graph-workspace";
        seed_workspace(&store, workspace_id).await;
        let base = Utc::now();
        for (index, block_id) in ["node-a", "node-b", "node-c"].into_iter().enumerate() {
            create_test_block(
                &store,
                workspace_id,
                block_id,
                block_id,
                base + chrono::Duration::seconds(index as i64),
            )
            .await;
        }

        for (edge_id, source, target, edge_type) in [
            ("edge-ab", "node-a", "node-b", LoomEdgeType::Mention),
            ("edge-bc", "node-b", "node-c", LoomEdgeType::Tag),
        ] {
            let edge = NewLoomEdge {
                edge_id: Some(edge_id.to_owned()),
                workspace_id: workspace_id.to_owned(),
                source_block_id: source.to_owned(),
                target_block_id: target.to_owned(),
                edge_type,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            };
            let write_metadata = metadata(edge_id, Utc::now());
            store
                .with_storage_operation(move |db| {
                    Box::pin(async move { create_loom_edge(&db, edge, write_metadata).await })
                })
                .await
                .expect("edge create lifecycle")
                .expect("create edge");
        }

        let workspace = workspace_id.to_owned();
        let traversal = store
            .with_storage_operation(move |db| {
                Box::pin(async move { traverse_graph(&db, &workspace, "node-a", 2, &[]).await })
            })
            .await
            .expect("traversal lifecycle")
            .expect("traverse graph");
        assert_eq!(
            traversal
                .iter()
                .map(|(block, depth)| (block.block_id.as_str(), *depth))
                .collect::<Vec<_>>(),
            vec![("node-b", 1), ("node-c", 2)]
        );

        let workspace = workspace_id.to_owned();
        let node_a = store
            .with_storage_operation(move |db| {
                Box::pin(async move { get_loom_block(&db, &workspace, "node-a").await })
            })
            .await
            .expect("node-a read lifecycle")
            .expect("read node-a");
        assert_eq!(node_a.derived.mention_count, 1);
        let workspace = workspace_id.to_owned();
        let node_b = store
            .with_storage_operation(move |db| {
                Box::pin(async move { get_loom_block(&db, &workspace, "node-b").await })
            })
            .await
            .expect("node-b read lifecycle")
            .expect("read node-b");
        assert_eq!(node_b.derived.backlink_count, 1);
        assert_eq!(node_b.derived.tag_count, 1);

        let workspace = workspace_id.to_owned();
        store
            .with_storage_operation(move |db| {
                Box::pin(async move { delete_loom_edge(&db, &workspace, "edge-ab").await })
            })
            .await
            .expect("edge delete lifecycle")
            .expect("delete edge");
        let workspace = workspace_id.to_owned();
        let node_a = store
            .with_storage_operation(move |db| {
                Box::pin(async move { get_loom_block(&db, &workspace, "node-a").await })
            })
            .await
            .expect("node-a reread lifecycle")
            .expect("reread node-a");
        assert_eq!(node_a.derived.mention_count, 0);
        store.shutdown().await.expect("close embedded store");
    }

    #[tokio::test]
    async fn database_test_helpers_use_the_durable_embedded_store() {
        let temp = tempfile::tempdir().expect("create temporary data root");
        let (config, store) = open_store(&temp).await;
        super::super::schema::bootstrap_schema(&store)
            .await
            .expect("bootstrap helper schema");
        let workspace_id = "loom-helper-workspace";
        seed_workspace(&store, workspace_id).await;

        let start = test_insert_loom_traversal_perf_fixture(&store, workspace_id, 4)
            .await
            .expect("insert real traversal fixture");
        assert_eq!(start, "perf-block-00000");
        assert!(matches!(
            test_insert_loom_traversal_perf_fixture(&store, workspace_id, 0).await,
            Err(StorageError::Validation(_))
        ));
        assert!(matches!(
            test_insert_loom_traversal_perf_fixture(&store, "missing-workspace", 1).await,
            Err(StorageError::NotFound("workspace"))
        ));

        test_overwrite_loom_block_metrics(&store, workspace_id, &start, 7, 8, 9)
            .await
            .expect("overwrite metrics in real store");
        let workspace = workspace_id.to_owned();
        let block = store
            .with_storage_operation(move |db| {
                Box::pin(async move { get_loom_block(&db, &workspace, "perf-block-00000").await })
            })
            .await
            .expect("helper block read lifecycle")
            .expect("read helper block");
        assert_eq!(block.derived.mention_count, 7);
        assert_eq!(block.derived.tag_count, 8);
        assert_eq!(block.derived.backlink_count, 9);
        test_zero_workspace_loom_metrics(&store, workspace_id)
            .await
            .expect("zero workspace metrics in real store");
        // PostgreSQL's test hook was intentionally a no-op when the target did
        // not exist; preserve that conformance behavior.
        test_overwrite_loom_block_metrics(&store, workspace_id, "missing-block", 1, 2, 3)
            .await
            .expect("missing metric target remains a no-op");

        store.shutdown().await.expect("close helper store");
        drop(store);
        let reopened = SurrealStorage::open(config)
            .await
            .expect("reopen helper store");
        super::super::schema::bootstrap_schema(&reopened)
            .await
            .expect("reassert helper schema");
        let workspace = workspace_id.to_owned();
        let durable = reopened
            .with_storage_operation(move |db| {
                Box::pin(async move {
                    traverse_graph(
                        &db,
                        &workspace,
                        "perf-block-00000",
                        3,
                        &[LoomEdgeType::Mention],
                    )
                    .await
                })
            })
            .await
            .expect("durable traversal lifecycle")
            .expect("traverse reopened fixture");
        assert_eq!(
            durable
                .iter()
                .map(|(block, depth)| (block.block_id.as_str(), *depth))
                .collect::<Vec<_>>(),
            vec![
                ("perf-block-00001", 1),
                ("perf-block-00002", 2),
                ("perf-block-00003", 3),
            ]
        );
        let workspace = workspace_id.to_owned();
        let start_block = reopened
            .with_storage_operation(move |db| {
                Box::pin(async move { get_loom_block(&db, &workspace, "perf-block-00000").await })
            })
            .await
            .expect("durable metric read lifecycle")
            .expect("read durable helper block");
        assert_eq!(start_block.derived.mention_count, 0);
        assert_eq!(start_block.derived.tag_count, 0);
        assert_eq!(start_block.derived.backlink_count, 0);
        reopened
            .shutdown()
            .await
            .expect("close reopened helper store");
    }
}

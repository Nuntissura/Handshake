//! Collections + contact sheets (MT-018): named, ordered image sets with
//! notes/tags and optional character/sheet links, plus a contact-sheet manifest
//! that snapshots the membership (source asset ids + content hashes) at capture
//! time so a sheet stays reproducible even as collections evolve.
//!
//! legacy source source: `app/backend/library.js` (`createCollection`, `updateCollection`,
//! `addImagesToCollection`, `removeImagesFromCollection`, `listCollectionImages`,
//! `createContactSheet`, `listContactSheets`) and `app/backend/db.js`
//! (`Collection`, `CollectionItem`, `ContactSheet` tables). Schema/behavior
//! intent only -- storage is the single embedded Handshake SurrealDB store,
//! never the legacy SQLite or PostgreSQL layers.
//! MT ids: MT-003 (module boundary), MT-005 (event coverage), MT-018 (this fold-in).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::media::{MediaReviewMetadata, MediaSourceProvenanceRefs};
use super::source_evidence::{normalize_optional_ckc_source_ref, CkcSourceRefKind};
use super::{
    atelier_event_sql, event_family, event_ref_for_text, reject_legacy_runtime_ref,
    search::normalize_tag, AtelierError, AtelierResult, AtelierStore,
};

/// Actor recorded on collection rows written by store-internal paths that have
/// no requesting actor (WP-CKC-posekit-overhaul MT-036 row attribution).
const SYSTEM_COLLECTION_ACTOR: &str = "system";

/// A named, ordered image set. Membership is ordered (`sort_order`) and may be
/// optionally bound to a character and/or a specific sheet version so a
/// collection can capture "this character at this sheet revision".
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Collection {
    pub collection_id: Uuid,
    pub name: String,
    pub notes: String,
    /// Free-form tags (kept as a JSON string array; de-duped and trimmed).
    pub tags: Vec<String>,
    pub character_internal_id: Option<Uuid>,
    pub sheet_version_id: Option<Uuid>,
    pub created_by: String,
    pub updated_by: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// One page of character-scoped collections plus the canonical total, so the
/// API can report `albums_next_offset` from the real row count rather than
/// from the rendered subset.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionPage {
    pub collections: Vec<Collection>,
    pub total_count: i64,
}

#[derive(Clone, Debug, Default)]
pub struct NewCollection {
    pub name: String,
    pub notes: String,
    pub tags: Vec<String>,
    pub character_internal_id: Option<Uuid>,
    pub sheet_version_id: Option<Uuid>,
}

/// One membership row resolved to its underlying media asset.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionMember {
    pub collection_id: Uuid,
    pub asset_id: Uuid,
    pub content_hash: String,
    pub sort_order: i64,
    pub linked_by: String,
    pub updated_by: String,
    pub updated_at_utc: DateTime<Utc>,
    pub added_at_utc: DateTime<Utc>,
}

/// A membership row with the asset fields and the link-scoped provenance the
/// CKC media-album pages render (WP-CKC-posekit-overhaul MT-010/MT-034/MT-035).
/// `source_path_ref` / `source_url_ref` live on the membership, not on the
/// asset: the same asset linked into two albums carries two independent refs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionMemberDetail {
    pub collection_id: Uuid,
    pub asset_id: Uuid,
    pub content_hash: String,
    pub mime: String,
    pub source_provenance: Option<String>,
    pub sort_order: i64,
    pub added_at_utc: DateTime<Utc>,
    pub linked_by: String,
    pub updated_by: String,
    pub updated_at_utc: DateTime<Utc>,
    pub source_path_ref: Option<String>,
    pub source_url_ref: Option<String>,
}

/// One page of album members plus the canonical member count.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionMemberPage {
    pub members: Vec<CollectionMemberDetail>,
    pub total_count: i64,
}

/// Durable receipt written when a membership is unlinked from a collection.
/// The asset row itself is preserved; only the membership goes away, and this
/// receipt keeps the prior order / link refs / attribution auditable.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionItemUnlinkReceipt {
    pub unlink_receipt_id: Uuid,
    pub collection_id: Uuid,
    pub asset_id: Uuid,
    pub prior_sort_order: i64,
    pub prior_source_path_ref: Option<String>,
    pub prior_source_url_ref: Option<String>,
    pub linked_by: String,
    pub member_updated_by: String,
    pub member_updated_at_utc: DateTime<Utc>,
    pub unlinked_by: String,
    pub unlinked_at_utc: DateTime<Utc>,
}

/// Result of appending assets to a collection with optional link-scoped refs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionItemsAdded {
    /// Memberships newly created by this call.
    pub inserted: i64,
    /// Pre-existing memberships whose link-scoped refs were changed by this call.
    pub updated_refs: i64,
}

/// One explicit `(asset, position)` pair of a full album reorder.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionItemReorder {
    pub asset_id: Uuid,
    pub sort_order: i64,
}

/// Edit of the link-scoped provenance on ONE membership. A `set_*` flag with a
/// `None` value clears that ref; a `false` flag leaves the stored ref alone.
#[derive(Clone, Debug, Default)]
pub struct CollectionItemLinkRefUpdate {
    pub set_source_path_ref: bool,
    pub source_path_ref: Option<String>,
    pub set_source_url_ref: bool,
    pub source_url_ref: Option<String>,
}

/// The CKC "notes/tags" save for one media asset: review notes + review status
/// (on `atelier_media_review_metadata`), the full replacement tag set (on
/// `atelier_media_asset_tag`), and optional asset-global source refs (on
/// `atelier_media_source_provenance_ref`), applied as ONE statement.
#[derive(Clone, Debug)]
pub struct MediaNotesTagsUpdate {
    pub asset_id: Uuid,
    /// `None` keeps the stored notes; `Some` replaces them.
    pub notes: Option<String>,
    /// `None` keeps the stored tag set; `Some` replaces it (normalized, de-duped).
    pub tags: Option<Vec<String>>,
    /// Canonical review status token (`unreviewed|review|approved|rejected|deferred`);
    /// `None` keeps the stored status (or `unreviewed` for a new row).
    pub review_status: Option<String>,
    pub source_path_ref: Option<String>,
    pub source_url_ref: Option<String>,
    pub updated_by: String,
}

/// What [`AtelierStore::apply_media_notes_tags`] persisted.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaNotesTagsResult {
    pub metadata: MediaReviewMetadata,
    /// The asset's tag texts after the write, ascending.
    pub tags: Vec<String>,
    pub provenance: Option<MediaSourceProvenanceRefs>,
    pub added_tags: Vec<String>,
    pub removed_tags: Vec<String>,
}

/// A tag attached directly to one media asset.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaAssetTag {
    pub asset_id: Uuid,
    pub tag_id: Uuid,
    pub text: String,
    pub source: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Request to apply collection-level tags/metadata to current member photos.
#[derive(Clone, Debug)]
pub struct CollectionMetadataApplicationRequest {
    pub collection_id: Uuid,
    pub requested_by: String,
    /// Tags to explicitly remove from member photos during this batch.
    pub remove_tags: Vec<String>,
}

/// Durable receipt for a collection-to-photo metadata batch application.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionMetadataApplication {
    pub application_id: Uuid,
    pub collection_id: Uuid,
    pub requested_by: String,
    pub applied_tags: Vec<String>,
    pub removed_tags: Vec<String>,
    pub affected_asset_count: i64,
    pub created_at_utc: DateTime<Utc>,
}

/// A contact sheet: an immutable manifest snapshot of a set of media assets.
/// The manifest captures source asset ids + content hashes at capture time so
/// the sheet is reproducible/auditable even if the source collection changes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactSheet {
    pub sheet_id: Uuid,
    pub name: String,
    /// Origin of the membership: `manual`, `collection`, `intake`, etc.
    pub source_type: String,
    /// Optional source identifier (e.g. the originating collection id as text).
    pub source_id: Option<String>,
    pub tags: Vec<String>,
    pub character_internal_id: Option<Uuid>,
    pub sheet_version_id: Option<Uuid>,
    /// `hsk.atelier.contact_sheet@1`-shaped manifest: {schema, source_type, source_id,
    /// items:[{asset_id, content_hash}], tags, captured_at}.
    pub manifest: serde_json::Value,
    pub image_count: i64,
    pub created_at_utc: DateTime<Utc>,
}

/// Deterministic SVG materialization for a contact-sheet manifest.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactSheetSvgArtifact {
    pub svg_artifact_id: Uuid,
    pub sheet_id: Uuid,
    pub manifest_hash: String,
    pub content_hash: String,
    pub artifact_ref: String,
    pub svg_text: String,
    pub image_count: i64,
    pub created_at_utc: DateTime<Utc>,
}

pub(crate) const CONTACT_SHEET_MANIFEST_SCHEMA: &str = "hsk.atelier.contact_sheet@1";

pub(crate) fn legacy_contact_sheet_manifest_schema() -> String {
    ["c", "kc.contact_sheet@1"].concat()
}

/// New event families contributed by the collections fold-in (extends MT-005).
pub mod collections_event_family {
    pub const COLLECTION_CREATED: &str = "atelier.collection.created";
    pub const COLLECTION_UPDATED: &str = "atelier.collection.updated";
    pub const COLLECTION_IMAGES_ADDED: &str = "atelier.collection.images_added";
    pub const COLLECTION_IMAGES_REMOVED: &str = "atelier.collection.images_removed";
    pub const MEDIA_ASSET_TAGGED: &str = "atelier.collection.media_asset_tagged";
    pub const MEDIA_ASSET_UNTAGGED: &str = "atelier.collection.media_asset_untagged";
    pub const COLLECTION_METADATA_APPLIED: &str = "atelier.collection.metadata_applied_to_images";
    pub const CONTACT_SHEET_CREATED: &str = "atelier.contact_sheet.created";
    pub const CONTACT_SHEET_SVG_RENDERED: &str = "atelier.contact_sheet.svg_rendered";
    /// One CKC media-album membership was unlinked (asset preserved, receipt written).
    pub const MEDIA_ALBUM_ITEM_UNLINKED: &str = "atelier.collection.media_album_item_unlinked";
    /// Link-scoped source refs on one CKC media-album membership were edited.
    pub const MEDIA_ALBUM_ITEM_LINK_REFS_UPDATED: &str =
        "atelier.collection.media_album_item_link_refs_updated";
    /// A CKC media album received an explicit full dense reorder.
    pub const MEDIA_ALBUM_ITEMS_REORDERED: &str = "atelier.collection.media_album_items_reordered";

    /// All collections event families (used by parity/coverage checks).
    pub const ALL: &[&str] = &[
        COLLECTION_CREATED,
        COLLECTION_UPDATED,
        COLLECTION_IMAGES_ADDED,
        COLLECTION_IMAGES_REMOVED,
        MEDIA_ASSET_TAGGED,
        MEDIA_ASSET_UNTAGGED,
        COLLECTION_METADATA_APPLIED,
        CONTACT_SHEET_CREATED,
        CONTACT_SHEET_SVG_RENDERED,
        MEDIA_ALBUM_ITEM_UNLINKED,
        MEDIA_ALBUM_ITEM_LINK_REFS_UPDATED,
        MEDIA_ALBUM_ITEMS_REORDERED,
    ];
}

/// Sentinel thrown inside an album-mutation statement when the membership it
/// targets is gone. The pre-flight read already answered `NotFound` for the
/// common case; this guards the window between that read and the write.
const THROW_ALBUM_MEMBER_MISSING: &str = "ckc_album_member_missing";
/// Sentinel thrown when a reorder's member set no longer matches the album.
const THROW_REORDER_MEMBERSHIP_CHANGED: &str = "ckc_reorder_membership_changed";
/// Sentinel thrown when the media asset a notes/tags save targets is gone.
const THROW_MEDIA_ASSET_MISSING: &str = "ckc_media_asset_missing";

/// Concurrency policy tokens reported on album mutations. The PostgreSQL
/// reference took a `FOR UPDATE` row lock on the collection and read the
/// members inside that lock; the embedded store gives the same guarantee as
/// one optimistic single-statement transaction that re-verifies its
/// preconditions and retries a lost race, so the tokens name that mechanism.
pub const ALBUM_MUTATION_CONCURRENCY_POLICY: &str =
    "collection_single_statement_transaction_snapshot";
pub const ALBUM_REORDER_CONCURRENCY_POLICY: &str =
    "collection_single_statement_transaction_full_dense_membership_verified";

/// `atelier_collection.scoped_name_key` is UNIQUE per `(character scope, name)`;
/// the store reports a duplicate album name as a typed `Conflict` rather than
/// leaking the index violation as a database error.
fn map_scoped_name_conflict(
    error: AtelierError,
    name: &str,
    character_internal_id: Option<Uuid>,
) -> AtelierError {
    let text = error.to_string();
    if text.contains("ux_atelier_collection_scoped_name") {
        return AtelierError::Conflict(match character_internal_id {
            Some(character_internal_id) => format!(
                "collection name {name:?} already exists for character_internal_id={character_internal_id}"
            ),
            None => format!("global collection name {name:?} already exists"),
        });
    }
    error
}

const SURREAL_TRANSACTION_MAX_ATTEMPTS: usize = 10;
const SURREAL_TRANSACTION_BACKOFF_CAP_MS: u64 = 32;

/// Embedded SurrealDB reports a lost optimistic-transaction race on a busy
/// album as a retryable conflict. Two concurrent reorders / links on one album
/// are the MT-056 F5/F9 case; the loser retries with a bounded jittered backoff.
fn is_surreal_retryable_transaction_conflict(error: &AtelierError) -> bool {
    matches!(
        error,
        AtelierError::Database(crate::storage::surreal::SurrealStorageError::Database(source))
            if source
                .to_string()
                .contains("Transaction conflict: Resource busy. This transaction can be retried")
    )
}

fn surreal_transaction_retry_delay(seed: Uuid, failed_attempt: usize) -> Duration {
    let exponential_cap = 1_u64
        .checked_shl(failed_attempt.min(5) as u32)
        .unwrap_or(SURREAL_TRANSACTION_BACKOFF_CAP_MS)
        .min(SURREAL_TRANSACTION_BACKOFF_CAP_MS);
    let seed = seed.as_u128();
    let mut mixed = (seed as u64)
        ^ ((seed >> 64) as u64)
        ^ (failed_attempt as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    mixed ^= mixed >> 30;
    mixed = mixed.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed ^= mixed >> 27;
    mixed = mixed.wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^= mixed >> 31;
    Duration::from_millis(mixed % (exponential_cap + 1))
}

/// Translate the statement-level sentinels (and an exhausted retry budget) of
/// an album mutation into the typed error the HTTP layer maps to 404 / 409.
fn map_album_mutation_error(error: AtelierError, collection_id: Uuid, asset_id: Option<Uuid>) -> AtelierError {
    let text = error.to_string();
    if text.contains(THROW_ALBUM_MEMBER_MISSING) {
        return AtelierError::NotFound(match asset_id {
            Some(asset_id) => format!("media album item collection_id={collection_id} asset_id={asset_id}"),
            None => format!("media album item collection_id={collection_id}"),
        });
    }
    if text.contains(THROW_REORDER_MEMBERSHIP_CHANGED) {
        return AtelierError::Conflict(format!(
            "album membership for collection_id={collection_id} changed while the reorder was being applied; re-read the album and retry"
        ));
    }
    if text.contains(THROW_MEDIA_ASSET_MISSING) {
        return AtelierError::NotFound(match asset_id {
            Some(asset_id) => format!("media asset_id={asset_id}"),
            None => "media asset".to_owned(),
        });
    }
    if is_surreal_retryable_transaction_conflict(&error) {
        return AtelierError::Conflict(format!(
            "collection_id={collection_id} is busy with a concurrent mutation; retry"
        ));
    }
    error
}

fn clean_tags(tags: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for tag in tags {
        let t = tag.trim().to_string();
        if t.is_empty() {
            continue;
        }
        if !seen.iter().any(|existing| existing == &t) {
            seen.push(t);
        }
    }
    seen
}

fn normalize_media_tags(tags: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for tag in tags {
        let normalized = normalize_tag(tag);
        if normalized.is_empty() {
            continue;
        }
        if !seen.iter().any(|existing| existing == &normalized) {
            seen.push(normalized);
        }
    }
    seen
}

fn require_collection_ref_text<'a>(field: &str, value: &'a str) -> AtelierResult<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    reject_legacy_runtime_ref(field, value)?;
    Ok(trimmed)
}

/// Actor written onto `created_by` / `updated_by` / `linked_by`. Same shape as
/// a ref text: non-empty, unpadded, never a machine-local runtime ref.
fn require_collection_actor<'a>(field: &str, value: &'a str) -> AtelierResult<&'a str> {
    require_collection_ref_text(field, value)
}

fn collection_actor_or_system<'a>(field: &str, value: Option<&'a str>) -> AtelierResult<&'a str> {
    match value {
        Some(raw) => require_collection_actor(field, raw),
        None => Ok(SYSTEM_COLLECTION_ACTOR),
    }
}

/// Canonical review-status tokens accepted by `atelier_media_review_metadata`.
const MEDIA_REVIEW_STATUS_TOKENS: &[&str] = &["unreviewed", "review", "approved", "rejected", "deferred"];

fn require_media_review_status(value: &str) -> AtelierResult<&str> {
    if MEDIA_REVIEW_STATUS_TOKENS.contains(&value) {
        Ok(value)
    } else {
        Err(AtelierError::Validation(format!(
            "unsupported review_status: {value}"
        )))
    }
}

/// Split the dense `(asset_id, sort_order)` reorder request the same way the
/// HTTP layer validates it, so a store caller cannot bypass the contract:
/// non-empty, no duplicate assets, no duplicate positions, positions dense
/// from 0.
fn validate_collection_reorder(items: &[CollectionItemReorder]) -> AtelierResult<Vec<Uuid>> {
    if items.is_empty() {
        return Err(AtelierError::Validation(
            "reorder items must not be empty".to_owned(),
        ));
    }
    let mut seen_assets = HashSet::new();
    let mut seen_orders = HashSet::new();
    let mut asset_ids = Vec::with_capacity(items.len());
    for item in items {
        if item.sort_order < 0 {
            return Err(AtelierError::Validation(format!(
                "sort_order for asset_id={} must be >= 0",
                item.asset_id
            )));
        }
        if !seen_assets.insert(item.asset_id) {
            return Err(AtelierError::Validation(format!(
                "duplicate asset_id={} in reorder request",
                item.asset_id
            )));
        }
        if !seen_orders.insert(item.sort_order) {
            return Err(AtelierError::Validation(format!(
                "duplicate sort_order={} in reorder request",
                item.sort_order
            )));
        }
        asset_ids.push(item.asset_id);
    }
    for expected in 0..items.len() as i64 {
        if !seen_orders.contains(&expected) {
            return Err(AtelierError::Validation(format!(
                "reorder sort_order values must be dense from 0; missing {expected}"
            )));
        }
    }
    Ok(asset_ids)
}

fn collection_record(collection_id: Uuid) -> RecordId {
    RecordId::new("atelier_collection", SurrealUuid::from(collection_id))
}

fn media_asset_record(asset_id: Uuid) -> RecordId {
    RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id))
}

fn collection_item_pair_key(collection_id: Uuid, asset_id: Uuid) -> Vec<SurrealUuid> {
    vec![SurrealUuid::from(collection_id), SurrealUuid::from(asset_id)]
}

#[derive(SurrealValue)]
struct CollectionRow {
    collection_id: SurrealUuid,
    name: String,
    notes: String,
    tags_json: Vec<String>,
    character_internal_id: Option<SurrealUuid>,
    sheet_version_id: Option<SurrealUuid>,
    created_by: String,
    updated_by: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl From<CollectionRow> for Collection {
    fn from(row: CollectionRow) -> Self {
        Self {
            collection_id: row.collection_id.into(),
            name: row.name,
            notes: row.notes,
            tags: row.tags_json,
            character_internal_id: row.character_internal_id.map(Into::into),
            sheet_version_id: row.sheet_version_id.map(Into::into),
            created_by: row.created_by,
            updated_by: row.updated_by,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct CollectionMemberRow {
    collection_id: SurrealUuid,
    asset_id: SurrealUuid,
    content_hash: String,
    sort_order: i64,
    linked_by: String,
    updated_by: String,
    updated_at_utc: Datetime,
    added_at_utc: Datetime,
}

impl From<CollectionMemberRow> for CollectionMember {
    fn from(row: CollectionMemberRow) -> Self {
        Self {
            collection_id: row.collection_id.into(),
            asset_id: row.asset_id.into(),
            content_hash: row.content_hash,
            sort_order: row.sort_order,
            linked_by: row.linked_by,
            updated_by: row.updated_by,
            updated_at_utc: row.updated_at_utc.into(),
            added_at_utc: row.added_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct CollectionMemberDetailRow {
    collection_id: SurrealUuid,
    asset_id: SurrealUuid,
    content_hash: String,
    mime: String,
    source_provenance: Option<String>,
    sort_order: i64,
    added_at_utc: Datetime,
    linked_by: String,
    updated_by: String,
    updated_at_utc: Datetime,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
}

impl From<CollectionMemberDetailRow> for CollectionMemberDetail {
    fn from(row: CollectionMemberDetailRow) -> Self {
        Self {
            collection_id: row.collection_id.into(),
            asset_id: row.asset_id.into(),
            content_hash: row.content_hash,
            mime: row.mime,
            source_provenance: row.source_provenance,
            sort_order: row.sort_order,
            added_at_utc: row.added_at_utc.into(),
            linked_by: row.linked_by,
            updated_by: row.updated_by,
            updated_at_utc: row.updated_at_utc.into(),
            source_path_ref: row.source_path_ref,
            source_url_ref: row.source_url_ref,
        }
    }
}

#[derive(SurrealValue)]
struct CollectionItemUnlinkReceiptRow {
    unlink_receipt_id: SurrealUuid,
    collection_id: SurrealUuid,
    asset_id: SurrealUuid,
    prior_sort_order: i64,
    prior_source_path_ref: Option<String>,
    prior_source_url_ref: Option<String>,
    linked_by: String,
    member_updated_by: String,
    member_updated_at_utc: Datetime,
    unlinked_by: String,
    unlinked_at_utc: Datetime,
}

impl From<CollectionItemUnlinkReceiptRow> for CollectionItemUnlinkReceipt {
    fn from(row: CollectionItemUnlinkReceiptRow) -> Self {
        Self {
            unlink_receipt_id: row.unlink_receipt_id.into(),
            collection_id: row.collection_id.into(),
            asset_id: row.asset_id.into(),
            prior_sort_order: row.prior_sort_order,
            prior_source_path_ref: row.prior_source_path_ref,
            prior_source_url_ref: row.prior_source_url_ref,
            linked_by: row.linked_by,
            member_updated_by: row.member_updated_by,
            member_updated_at_utc: row.member_updated_at_utc.into(),
            unlinked_by: row.unlinked_by,
            unlinked_at_utc: row.unlinked_at_utc.into(),
        }
    }
}

/// Per-asset review metadata / tags / asset-global provenance for one page of
/// album members, fetched as three batched statements instead of one round
/// trip per member (MT-056 F1: LIST_CAP members must not mean 3 x LIST_CAP
/// queries).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaAlbumMemberEnrichment {
    pub asset_id: Uuid,
    pub metadata: Option<MediaReviewMetadata>,
    /// Direct tag texts on the asset, ascending.
    pub tags: Vec<String>,
    pub provenance: Option<MediaSourceProvenanceRefs>,
}

#[derive(SurrealValue)]
struct MemberReviewMetadataRow {
    asset_id: SurrealUuid,
    favorite: bool,
    rating: i64,
    frontpage: bool,
    carousel: bool,
    notes: Option<String>,
    review_status: String,
    updated_by: String,
    updated_at_utc: Datetime,
}

impl TryFrom<MemberReviewMetadataRow> for MediaReviewMetadata {
    type Error = AtelierError;

    fn try_from(row: MemberReviewMetadataRow) -> AtelierResult<Self> {
        Ok(Self {
            asset_id: row.asset_id.into(),
            favorite: row.favorite,
            rating: i16::try_from(row.rating).map_err(|_| {
                AtelierError::Internal(format!(
                    "media review rating {} does not fit the i16 contract",
                    row.rating
                ))
            })?,
            frontpage: row.frontpage,
            carousel: row.carousel,
            notes: row.notes,
            review_status: row.review_status,
            updated_by: row.updated_by,
            updated_at_utc: row.updated_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct MemberTagTextRow {
    asset_id: SurrealUuid,
    text: String,
}

#[derive(SurrealValue)]
struct MemberProvenanceRow {
    asset_id: SurrealUuid,
    source_url_ref: Option<String>,
    source_path_ref: Option<String>,
    source_note_ref: Option<String>,
    contact_sheet_ref: Option<String>,
    task_ref: Option<String>,
    run_ref: Option<String>,
    updated_by: String,
    updated_at_utc: Datetime,
}

impl From<MemberProvenanceRow> for MediaSourceProvenanceRefs {
    fn from(row: MemberProvenanceRow) -> Self {
        Self {
            asset_id: row.asset_id.into(),
            source_url_ref: row.source_url_ref,
            source_path_ref: row.source_path_ref,
            source_note_ref: row.source_note_ref,
            contact_sheet_ref: row.contact_sheet_ref,
            task_ref: row.task_ref,
            run_ref: row.run_ref,
            updated_by: row.updated_by,
            updated_at_utc: row.updated_at_utc.into(),
        }
    }
}

/// Snapshot returned by the notes/tags statement: the review row after the
/// write, the asset's tag texts, and the provenance row if one exists.
#[derive(SurrealValue)]
struct MediaNotesTagsSnapshotRow {
    metadata: Vec<MemberReviewMetadataRow>,
    tags: Vec<String>,
    provenance: Vec<MemberProvenanceRow>,
}

#[derive(SurrealValue)]
struct CollectionItemsAddedRow {
    inserted: i64,
    updated_refs: i64,
}

#[derive(SurrealValue)]
struct MediaAssetTagRow {
    asset_id: SurrealUuid,
    tag_id: SurrealUuid,
    text: String,
    source: String,
    created_at_utc: Datetime,
}

impl From<MediaAssetTagRow> for MediaAssetTag {
    fn from(row: MediaAssetTagRow) -> Self {
        Self {
            asset_id: row.asset_id.into(),
            tag_id: row.tag_id.into(),
            text: row.text,
            source: row.source,
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct CollectionMetadataApplicationRow {
    application_id: SurrealUuid,
    collection_id: SurrealUuid,
    requested_by: String,
    applied_tags_json: Vec<String>,
    removed_tags_json: Vec<String>,
    affected_asset_count: i64,
    created_at_utc: Datetime,
}

impl From<CollectionMetadataApplicationRow> for CollectionMetadataApplication {
    fn from(row: CollectionMetadataApplicationRow) -> Self {
        Self {
            application_id: row.application_id.into(),
            collection_id: row.collection_id.into(),
            requested_by: row.requested_by,
            applied_tags: row.applied_tags_json,
            removed_tags: row.removed_tags_json,
            affected_asset_count: row.affected_asset_count,
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct ContactSheetRow {
    sheet_id: SurrealUuid,
    name: String,
    source_type: String,
    source_id: Option<String>,
    tags_json: Vec<String>,
    character_internal_id: Option<SurrealUuid>,
    sheet_version_id: Option<SurrealUuid>,
    manifest: serde_json::Value,
    image_count: i64,
    created_at_utc: Datetime,
}

impl From<ContactSheetRow> for ContactSheet {
    fn from(row: ContactSheetRow) -> Self {
        Self {
            sheet_id: row.sheet_id.into(),
            name: row.name,
            source_type: row.source_type,
            source_id: row.source_id,
            tags: row.tags_json,
            character_internal_id: row.character_internal_id.map(Into::into),
            sheet_version_id: row.sheet_version_id.map(Into::into),
            manifest: row.manifest,
            image_count: row.image_count,
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct ContactSheetSvgArtifactRow {
    svg_artifact_id: SurrealUuid,
    sheet_id: SurrealUuid,
    manifest_hash: String,
    content_hash: String,
    artifact_ref: String,
    svg_text: String,
    image_count: i64,
    created_at_utc: Datetime,
}

impl From<ContactSheetSvgArtifactRow> for ContactSheetSvgArtifact {
    fn from(row: ContactSheetSvgArtifactRow) -> Self {
        Self {
            svg_artifact_id: row.svg_artifact_id.into(),
            sheet_id: row.sheet_id.into(),
            manifest_hash: row.manifest_hash,
            content_hash: row.content_hash,
            artifact_ref: row.artifact_ref,
            svg_text: row.svg_text,
            image_count: row.image_count,
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

fn sha256_ref(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

fn deterministic_uuid(seed: &[u8]) -> Uuid {
    let digest = Sha256::digest(seed);
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn contact_sheet_manifest_items(sheet: &ContactSheet) -> AtelierResult<Vec<(Uuid, String)>> {
    let items = sheet
        .manifest
        .get("items")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            AtelierError::Validation("contact sheet manifest must contain an items array".into())
        })?;
    if items.is_empty() {
        return Err(AtelierError::Validation(
            "contact sheet SVG requires at least one manifest item".into(),
        ));
    }

    let mut resolved = Vec::with_capacity(items.len());
    for item in items {
        let asset_id = item
            .get("asset_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                AtelierError::Validation("contact sheet manifest item missing asset_id".into())
            })?;
        let asset_id = Uuid::parse_str(asset_id).map_err(|err| {
            AtelierError::Validation(format!(
                "contact sheet manifest item asset_id is not a uuid: {err}"
            ))
        })?;
        let content_hash = item
            .get("content_hash")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                AtelierError::Validation("contact sheet manifest item missing content_hash".into())
            })?
            .trim()
            .to_string();
        if content_hash.is_empty() {
            return Err(AtelierError::Validation(
                "contact sheet manifest item content_hash must not be empty".into(),
            ));
        }
        resolved.push((asset_id, content_hash));
    }
    Ok(resolved)
}

fn render_contact_sheet_svg_text(sheet: &ContactSheet) -> AtelierResult<(String, i64)> {
    let items = contact_sheet_manifest_items(sheet)?;
    let image_count = i64::try_from(items.len()).map_err(|_| {
        AtelierError::Validation("contact sheet manifest has too many items".into())
    })?;
    let cols = usize::min(
        4,
        items.len().max(1).isqrt().max(1) + usize::from(items.len() > 1),
    );
    let rows = items.len().div_ceil(cols);
    let tile_w = 220usize;
    let tile_h = 150usize;
    let gap = 16usize;
    let margin = 24usize;
    let width = margin * 2 + cols * tile_w + (cols.saturating_sub(1)) * gap;
    let height = margin * 2 + rows * tile_h + (rows.saturating_sub(1)) * gap + 48;
    let title = escape_xml(&sheet.name);
    let source_id = sheet.source_id.as_deref().unwrap_or("");
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-labelledby=\"title desc\" data-sheet-id=\"{}\" data-source-type=\"{}\">\n",
        sheet.sheet_id,
        escape_xml(&sheet.source_type)
    );
    svg.push_str(&format!("<title id=\"title\">{title}</title>\n"));
    svg.push_str(&format!(
        "<desc id=\"desc\">contact_sheet_id={} source_type={} source_id={} image_count={}</desc>\n",
        sheet.sheet_id,
        escape_xml(&sheet.source_type),
        escape_xml(source_id),
        image_count
    ));
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#f8fafc\"/>\n");
    svg.push_str(&format!(
        "<text x=\"{margin}\" y=\"28\" font-family=\"Arial, sans-serif\" font-size=\"16\" fill=\"#111827\">{title}</text>\n"
    ));
    for (index, (asset_id, content_hash)) in items.iter().enumerate() {
        let col = index % cols;
        let row = index / cols;
        let x = margin + col * (tile_w + gap);
        let y = margin + 24 + row * (tile_h + gap);
        let asset = asset_id.to_string();
        let asset_label = &asset[..8];
        let escaped_hash = escape_xml(content_hash);
        svg.push_str(&format!(
            "<g class=\"contact-sheet-item\" data-index=\"{index}\" data-asset-id=\"{asset}\" data-content-hash=\"{escaped_hash}\">\n"
        ));
        svg.push_str(&format!(
            "<title>asset_id={asset} content_hash={escaped_hash}</title>\n"
        ));
        svg.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{tile_w}\" height=\"{tile_h}\" rx=\"6\" fill=\"#ffffff\" stroke=\"#94a3b8\"/>\n"
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#334155\">#{}</text>\n",
            x + 12,
            y + 24,
            index + 1
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#475569\">asset {}</text>\n",
            x + 12,
            y + 48,
            asset_label
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"9\" fill=\"#64748b\">{}</text>\n",
            x + 12,
            y + 72,
            escaped_hash
        ));
        svg.push_str("</g>\n");
    }
    svg.push_str("</svg>\n");
    Ok((svg, image_count))
}

macro_rules! collection_select {
    () => {
        "collection_id, name, notes, tags_json, \
         IF character_internal_id = NONE { NONE } ELSE { record::id(character_internal_id) } \
           AS character_internal_id, \
         IF sheet_version_id = NONE { NONE } ELSE { record::id(sheet_version_id) } \
           AS sheet_version_id, \
         created_by, updated_by, created_at_utc, updated_at_utc"
    };
}

macro_rules! collection_member_select {
    () => {
        "record::id(collection_id) AS collection_id, record::id(asset_id) AS asset_id, \
         asset_id.content_hash AS content_hash, sort_order, linked_by, updated_by, \
         updated_at_utc, added_at_utc"
    };
}

/// Membership row joined with the asset fields and the link-scoped refs the
/// CKC media-album pages render (`CollectionMemberDetail`).
macro_rules! collection_member_detail_select {
    () => {
        "record::id(collection_id) AS collection_id, record::id(asset_id) AS asset_id, \
         asset_id.content_hash AS content_hash, asset_id.mime AS mime, \
         asset_id.source_provenance AS source_provenance, sort_order, added_at_utc, \
         linked_by, updated_by, updated_at_utc, source_path_ref, source_url_ref"
    };
}

macro_rules! unlink_receipt_select {
    () => {
        "unlink_receipt_id, record::id(collection_id) AS collection_id, \
         record::id(asset_id) AS asset_id, prior_sort_order, prior_source_path_ref, \
         prior_source_url_ref, linked_by, member_updated_by, member_updated_at_utc, \
         unlinked_by, unlinked_at_utc"
    };
}

macro_rules! contact_sheet_select {
    () => {
        "sheet_id, name, source_type, source_id, tags_json, \
         IF character_internal_id = NONE { NONE } ELSE { record::id(character_internal_id) } \
           AS character_internal_id, \
         IF sheet_version_id = NONE { NONE } ELSE { record::id(sheet_version_id) } \
           AS sheet_version_id, \
         manifest, image_count, created_at_utc"
    };
}

macro_rules! svg_artifact_select {
    () => {
        "svg_artifact_id, record::id(sheet_id) AS sheet_id, manifest_hash, content_hash, \
         artifact_ref, svg_text, image_count, created_at_utc"
    };
}

#[derive(Clone, SurrealValue)]
struct CreateCollectionBindings {
    collection_rid: RecordId,
    collection_id: SurrealUuid,
    name: String,
    notes: String,
    tags_json: Vec<String>,
    character_ref: Option<RecordId>,
    sheet_version_ref: Option<RecordId>,
    actor: String,
}

#[derive(SurrealValue)]
struct CharacterCollectionPageBindings {
    character_ref: RecordId,
    limit: i64,
    offset: i64,
}

#[derive(SurrealValue)]
struct CharacterRefBinding {
    character_ref: RecordId,
}

#[derive(SurrealValue)]
struct CollectionMemberPageBindings {
    collection_ref: RecordId,
    limit: i64,
    offset: i64,
}

#[derive(SurrealValue)]
struct AssetRefsBinding {
    asset_refs: Vec<RecordId>,
}

#[derive(Clone, SurrealValue)]
struct UnlinkAlbumItemBindings {
    collection_ref: RecordId,
    member_key: Vec<SurrealUuid>,
    receipt_rid: RecordId,
    unlink_receipt_id: SurrealUuid,
    actor: String,
}

#[derive(Clone, SurrealValue)]
struct UpdateAlbumItemLinkRefsBindings {
    collection_ref: RecordId,
    member_key: Vec<SurrealUuid>,
    set_source_path_ref: bool,
    source_path_ref: Option<String>,
    set_source_url_ref: bool,
    source_url_ref: Option<String>,
    actor: String,
}

#[derive(Clone, SurrealValue)]
struct ReorderItemInput {
    member_key: Vec<SurrealUuid>,
    sort_order: i64,
}

#[derive(Clone, SurrealValue)]
struct ReorderAlbumItemsBindings {
    collection_ref: RecordId,
    expected_asset_ids: Vec<SurrealUuid>,
    items: Vec<ReorderItemInput>,
    actor: String,
}

#[derive(Clone, SurrealValue)]
struct NotesTagCandidate {
    tag_rid: RecordId,
    tag_id: SurrealUuid,
    text: String,
}

#[derive(Clone, SurrealValue)]
struct ApplyMediaNotesTagsBindings {
    asset_ref: RecordId,
    metadata_rid: RecordId,
    provenance_rid: RecordId,
    replace_tags: bool,
    added_tags: Vec<NotesTagCandidate>,
    removed_tags: Vec<String>,
    tag_source: String,
    favorite: bool,
    rating: i64,
    frontpage: bool,
    carousel: bool,
    notes: Option<String>,
    review_status: String,
    write_provenance: bool,
    source_url_ref: Option<String>,
    source_path_ref: Option<String>,
    source_note_ref: Option<String>,
    contact_sheet_ref: Option<String>,
    task_ref: Option<String>,
    run_ref: Option<String>,
    actor: String,
}

#[derive(SurrealValue)]
struct UnlinkReceiptIdBinding {
    unlink_receipt_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct CollectionIdBinding {
    collection_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct CollectionRefBinding {
    collection_ref: RecordId,
}

#[derive(Clone, SurrealValue)]
struct UpdateCollectionBindings {
    collection_rid: RecordId,
    name: Option<String>,
    notes: Option<String>,
    tags_json: Option<Vec<String>>,
    actor: String,
}

#[derive(Clone, SurrealValue)]
struct CollectionItemInput {
    asset_ref: RecordId,
    pair_key: Vec<SurrealUuid>,
    order_offset: i64,
}

#[derive(Clone, SurrealValue)]
struct AddImagesBindings {
    collection_ref: RecordId,
    asset_refs: Vec<RecordId>,
    items: Vec<CollectionItemInput>,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
    actor: String,
}

#[derive(Clone, SurrealValue)]
struct RemoveImageInput {
    asset_ref: RecordId,
    pair_key: Vec<SurrealUuid>,
    receipt_rid: RecordId,
    unlink_receipt_id: SurrealUuid,
}

#[derive(Clone, SurrealValue)]
struct RemoveImagesBindings {
    collection_ref: RecordId,
    asset_refs: Vec<RecordId>,
    items: Vec<RemoveImageInput>,
    actor: String,
}

#[derive(SurrealValue)]
struct AssetIdBinding {
    asset_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct AssetRefBinding {
    asset_ref: RecordId,
}

#[derive(Clone, SurrealValue)]
struct TagMediaAssetBindings {
    asset_ref: RecordId,
    tag_rid: RecordId,
    tag_id: SurrealUuid,
    text: String,
    source: String,
}

#[derive(SurrealValue)]
struct UntagMediaAssetBindings {
    asset_ref: RecordId,
    text: String,
}

#[derive(Clone, SurrealValue)]
struct MetadataTagCandidate {
    tag_rid: RecordId,
    tag_id: SurrealUuid,
    text: String,
}

#[derive(Clone, SurrealValue)]
struct ApplyCollectionMetadataBindings {
    application_rid: RecordId,
    application_id: SurrealUuid,
    collection_ref: RecordId,
    requested_by: String,
    applied_tags: Vec<MetadataTagCandidate>,
    applied_tags_json: Vec<String>,
    removed_tags_json: Vec<String>,
    tag_source: String,
}

#[derive(SurrealValue)]
struct AssetIdsBinding {
    asset_ids: Vec<SurrealUuid>,
}

#[derive(SurrealValue)]
struct AssetHashRow {
    asset_id: SurrealUuid,
    content_hash: String,
}

#[derive(Clone, SurrealValue)]
struct CreateContactSheetBindings {
    sheet_rid: RecordId,
    sheet_id: SurrealUuid,
    name: String,
    source_type: String,
    source_id: Option<String>,
    tags_json: Vec<String>,
    character_ref: Option<RecordId>,
    sheet_version_ref: Option<RecordId>,
    manifest: serde_json::Value,
    image_count: i64,
}

#[derive(SurrealValue)]
struct SheetIdBinding {
    sheet_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct SourceTypeBinding {
    source_type: Option<String>,
}

#[derive(SurrealValue)]
struct FindSvgArtifactBindings {
    sheet_ref: RecordId,
    manifest_hash: String,
}

#[derive(Clone, SurrealValue)]
struct CreateSvgArtifactBindings {
    artifact_rid: RecordId,
    svg_artifact_id: SurrealUuid,
    sheet_ref: RecordId,
    manifest_hash: String,
    content_hash: String,
    artifact_ref: String,
    svg_text: String,
    image_count: i64,
}

#[derive(SurrealValue)]
struct SvgArtifactWriteResult {
    artifact: Vec<ContactSheetSvgArtifactRow>,
    created: bool,
}

const CREATE_COLLECTION_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.collection_rid; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         collection_id: $domain.collection_id, name: $domain.name, notes: $domain.notes, \
         tags_json: $domain.tags_json, character_internal_id: $domain.character_ref, \
         sheet_version_id: $domain.sheet_version_ref, \
         created_by: $domain.actor, updated_by: $domain.actor \
       }; RETURN (SELECT ",
    collection_select!(),
    " FROM $rid); };"
);

const COUNT_CHARACTER_COLLECTIONS_STATEMENT: &str =
    "RETURN count(SELECT id FROM atelier_collection WHERE character_internal_id = $character_ref);";

/// Album pages are ordered by immutable keys (MT-056 F4): `created_at_utc`
/// never moves once written and `collection_id` breaks ties, so two reads of
/// the same offset return the same rows even while other albums are edited.
const LIST_CHARACTER_COLLECTIONS_PAGE_STATEMENT: &str = concat!(
    "SELECT ",
    collection_select!(),
    " FROM atelier_collection WHERE character_internal_id = $character_ref \
      ORDER BY created_at_utc DESC, collection_id ASC LIMIT $limit START $offset;"
);

const COUNT_COLLECTION_MEMBERS_STATEMENT: &str =
    "RETURN count(SELECT id FROM atelier_collection_item WHERE collection_id = $collection_ref);";

/// Member pages are ordered by `sort_order` then `asset_id` (immutable tie
/// break, MT-056 F4).
const LIST_COLLECTION_MEMBER_PAGE_STATEMENT: &str = concat!(
    "SELECT ",
    collection_member_detail_select!(),
    " FROM atelier_collection_item WHERE collection_id = $collection_ref \
      ORDER BY sort_order ASC, asset_id ASC LIMIT $limit START $offset;"
);

const LIST_COLLECTION_MEMBER_ASSET_IDS_STATEMENT: &str =
    "SELECT VALUE record::id(asset_id) FROM atelier_collection_item \
     WHERE collection_id = $collection_ref ORDER BY sort_order ASC, asset_id ASC;";

const MEMBER_REVIEW_METADATA_BATCH_STATEMENT: &str =
    "SELECT record::id(asset_id) AS asset_id, favorite, rating, frontpage, carousel, notes, \
     review_status, updated_by, updated_at_utc FROM atelier_media_review_metadata \
     WHERE asset_id IN $asset_refs;";

const MEMBER_TAGS_BATCH_STATEMENT: &str =
    "SELECT record::id(asset_id) AS asset_id, tag_id.text AS text FROM atelier_media_asset_tag \
     WHERE asset_id IN $asset_refs ORDER BY text ASC;";

const MEMBER_PROVENANCE_BATCH_STATEMENT: &str =
    "SELECT record::id(asset_id) AS asset_id, source_url_ref, source_path_ref, source_note_ref, \
     contact_sheet_ref, task_ref, run_ref, updated_by, updated_at_utc \
     FROM atelier_media_source_provenance_ref WHERE asset_id IN $asset_refs;";

/// Unlink ONE membership: copy the row into the receipt table, delete the
/// membership, bump the collection, all with the event in one statement. The
/// asset row is never touched. A vanished membership throws the sentinel so
/// the caller answers 404 instead of writing a receipt for nothing.
const UNLINK_ALBUM_ITEM_STATEMENT: &str = concat!(
    "RETURN { \
       LET $member_rid = type::record('atelier_collection_item', $domain.member_key); \
       IF !record::exists($member_rid) { THROW 'ckc_album_member_missing'; }; \
       LET $member = (SELECT * FROM ONLY $member_rid); ",
    atelier_event_sql!(),
    " CREATE $domain.receipt_rid CONTENT { \
         unlink_receipt_id: $domain.unlink_receipt_id, collection_id: $member.collection_id, \
         asset_id: $member.asset_id, prior_sort_order: $member.sort_order, \
         prior_source_path_ref: $member.source_path_ref, \
         prior_source_url_ref: $member.source_url_ref, linked_by: $member.linked_by, \
         member_updated_by: $member.updated_by, member_updated_at_utc: $member.updated_at_utc, \
         unlinked_by: $domain.actor \
       }; \
       DELETE $member_rid; \
       UPDATE $domain.collection_ref SET updated_by = $domain.actor, updated_at_utc = time::now(); \
       RETURN (SELECT ",
    unlink_receipt_select!(),
    " FROM $domain.receipt_rid); };"
);

const GET_UNLINK_RECEIPT_STATEMENT: &str = concat!(
    "SELECT ",
    unlink_receipt_select!(),
    " FROM atelier_collection_item_unlink_receipt WHERE unlink_receipt_id = $unlink_receipt_id \
      LIMIT 1;"
);

/// Edit the link-scoped refs of ONE membership. A `set_*` flag false leaves the
/// stored ref alone; true writes the (possibly NONE = cleared) value.
const UPDATE_ALBUM_ITEM_LINK_REFS_STATEMENT: &str = concat!(
    "RETURN { \
       LET $member_rid = type::record('atelier_collection_item', $domain.member_key); \
       IF !record::exists($member_rid) { THROW 'ckc_album_member_missing'; }; ",
    atelier_event_sql!(),
    " UPDATE $member_rid SET \
         source_path_ref = IF $domain.set_source_path_ref { $domain.source_path_ref } \
                           ELSE { source_path_ref }, \
         source_url_ref = IF $domain.set_source_url_ref { $domain.source_url_ref } \
                          ELSE { source_url_ref }, \
         updated_by = $domain.actor, updated_at_utc = time::now(); \
       UPDATE $domain.collection_ref SET updated_by = $domain.actor, updated_at_utc = time::now(); \
       RETURN 1; };"
);

/// Full dense reorder. The membership set is re-verified INSIDE the statement:
/// if a concurrent link/unlink changed it between the caller's pre-flight and
/// this write, the sentinel throws and nothing is written (MT-056 F5/F9).
const REORDER_ALBUM_ITEMS_STATEMENT: &str = concat!(
    "RETURN { \
       LET $current = (SELECT VALUE record::id(asset_id) FROM atelier_collection_item \
                       WHERE collection_id = $domain.collection_ref); \
       IF array::len($current) != array::len($domain.expected_asset_ids) \
          OR array::len(array::complement($domain.expected_asset_ids, $current)) != 0 { \
         THROW 'ckc_reorder_membership_changed'; \
       }; ",
    atelier_event_sql!(),
    " FOR $item IN $domain.items { \
         LET $rid = type::record('atelier_collection_item', $item.member_key); \
         IF !record::exists($rid) { THROW 'ckc_reorder_membership_changed'; }; \
         UPDATE $rid SET sort_order = $item.sort_order, updated_by = $domain.actor, \
                         updated_at_utc = time::now(); \
       }; \
       UPDATE $domain.collection_ref SET updated_by = $domain.actor, updated_at_utc = time::now(); \
       RETURN array::len($domain.items); };"
);

/// The CKC notes/tags save as ONE statement: review metadata upsert (carrying
/// forward favorite/rating/frontpage/carousel), tag set replacement (dictionary
/// rows created on demand, link rows keyed `[asset, tag]`), optional
/// asset-global provenance upsert, then a snapshot of all three. The event
/// fragment records `MEDIA_REVIEW_METADATA_UPDATED`; the per-tag and
/// provenance events are appended after commit by the caller.
const APPLY_MEDIA_NOTES_TAGS_STATEMENT: &str = concat!(
    "RETURN { \
       IF !record::exists($domain.asset_ref) { THROW 'ckc_media_asset_missing'; }; ",
    atelier_event_sql!(),
    " UPSERT $domain.metadata_rid SET asset_id = $domain.asset_ref, \
         favorite = $domain.favorite, rating = $domain.rating, frontpage = $domain.frontpage, \
         carousel = $domain.carousel, notes = $domain.notes, \
         review_status = $domain.review_status, updated_by = $domain.actor, \
         updated_at_utc = time::now(); \
       IF $domain.replace_tags { \
         LET $removed_refs = (SELECT VALUE id FROM atelier_tag WHERE text IN $domain.removed_tags); \
         DELETE atelier_media_asset_tag WHERE asset_id = $domain.asset_ref \
                                          AND tag_id IN $removed_refs; \
         FOR $tag IN $domain.added_tags { \
           LET $existing = (SELECT VALUE id FROM atelier_tag WHERE text = $tag.text LIMIT 1); \
           IF $existing = [] { CREATE $tag.tag_rid CONTENT { tag_id: $tag.tag_id, text: $tag.text }; }; \
           LET $tag_ref = (SELECT VALUE id FROM atelier_tag WHERE text = $tag.text LIMIT 1)[0]; \
           LET $link_rid = type::record('atelier_media_asset_tag', \
                                        [record::id($domain.asset_ref), record::id($tag_ref)]); \
           UPSERT $link_rid SET asset_id = $domain.asset_ref, tag_id = $tag_ref, \
                                source = $domain.tag_source; \
         }; \
       }; \
       IF $domain.write_provenance { \
         UPSERT $domain.provenance_rid SET asset_id = $domain.asset_ref, \
           source_url_ref = $domain.source_url_ref, source_path_ref = $domain.source_path_ref, \
           source_note_ref = $domain.source_note_ref, \
           contact_sheet_ref = $domain.contact_sheet_ref, task_ref = $domain.task_ref, \
           run_ref = $domain.run_ref, updated_by = $domain.actor, updated_at_utc = time::now(); \
       }; \
       RETURN { \
         metadata: (SELECT record::id(asset_id) AS asset_id, favorite, rating, frontpage, \
                           carousel, notes, review_status, updated_by, updated_at_utc \
                    FROM $domain.metadata_rid), \
         tags: (SELECT VALUE tag_id.text FROM atelier_media_asset_tag \
                WHERE asset_id = $domain.asset_ref), \
         provenance: (SELECT record::id(asset_id) AS asset_id, source_url_ref, source_path_ref, \
                             source_note_ref, contact_sheet_ref, task_ref, run_ref, updated_by, \
                             updated_at_utc \
                      FROM $domain.provenance_rid) \
       }; };"
);

const GET_COLLECTION_STATEMENT: &str = concat!(
    "SELECT ",
    collection_select!(),
    " FROM atelier_collection WHERE collection_id = $collection_id LIMIT 1;"
);

const LIST_COLLECTIONS_STATEMENT: &str = concat!(
    "SELECT ",
    collection_select!(),
    " FROM atelier_collection ORDER BY updated_at_utc DESC, collection_id ASC;"
);

const UPDATE_COLLECTION_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.collection_rid; ",
    atelier_event_sql!(),
    " UPDATE $rid SET name = $domain.name ?? name, notes = $domain.notes ?? notes, \
         tags_json = $domain.tags_json ?? tags_json, updated_by = $domain.actor, \
         updated_at_utc = time::now(); \
       RETURN (SELECT ",
    collection_select!(),
    " FROM $rid); };"
);

/// Append memberships. New rows are appended after the current max
/// `sort_order` carrying the link-scoped refs and the actor; an existing
/// membership is left alone unless a ref was supplied that differs from the
/// stored one, in which case only that ref (and `updated_by`) changes and the
/// row counts toward `updated_refs`. Membership rows and the event commit as
/// one statement (MT-056 F10).
const ADD_IMAGES_STATEMENT: &str = concat!(
    "RETURN { \
       LET $existing_refs = (SELECT VALUE asset_id FROM atelier_collection_item \
                             WHERE collection_id = $domain.collection_ref \
                               AND asset_id IN $domain.asset_refs); \
       LET $existing_count = array::len($existing_refs); \
       LET $next_order = (array::max((SELECT VALUE sort_order FROM atelier_collection_item \
                                     WHERE collection_id = $domain.collection_ref)) ?? -1) + 1; ",
    atelier_event_sql!(),
    " FOR $item IN $domain.items { \
         LET $rid = type::record('atelier_collection_item', $item.pair_key); \
         IF !record::exists($rid) { \
           CREATE $rid CONTENT { collection_id: $domain.collection_ref, \
             asset_id: $item.asset_ref, sort_order: $next_order + $item.order_offset, \
             source_path_ref: $domain.source_path_ref, source_url_ref: $domain.source_url_ref, \
             linked_by: $domain.actor, updated_by: $domain.actor }; \
         }; \
       }; \
       LET $updated_refs = IF $domain.source_path_ref != NONE OR $domain.source_url_ref != NONE { \
         array::len((UPDATE atelier_collection_item SET \
             source_path_ref = $domain.source_path_ref ?? source_path_ref, \
             source_url_ref = $domain.source_url_ref ?? source_url_ref, \
             updated_by = $domain.actor, updated_at_utc = time::now() \
           WHERE collection_id = $domain.collection_ref AND asset_id IN $existing_refs \
             AND (($domain.source_path_ref != NONE AND source_path_ref != $domain.source_path_ref) \
                  OR ($domain.source_url_ref != NONE AND source_url_ref != $domain.source_url_ref)) \
           RETURN AFTER)) \
       } ELSE { 0 }; \
       LET $inserted = array::len($domain.items) - $existing_count; \
       IF $inserted > 0 OR $updated_refs > 0 { \
         UPDATE $domain.collection_ref SET updated_by = $domain.actor, updated_at_utc = time::now(); \
       }; \
       RETURN { inserted: $inserted, updated_refs: $updated_refs }; };"
);

/// Remove memberships, writing one unlink receipt per removed row so the prior
/// order / link refs / attribution stay auditable (MT-036 receipt half).
const REMOVE_IMAGES_STATEMENT: &str = concat!(
    "RETURN { \
       LET $removed = count(SELECT id FROM atelier_collection_item \
                            WHERE collection_id = $domain.collection_ref \
                              AND asset_id IN $domain.asset_refs); ",
    atelier_event_sql!(),
    " FOR $item IN $domain.items { \
         LET $rid = type::record('atelier_collection_item', $item.pair_key); \
         IF record::exists($rid) { \
           LET $member = (SELECT * FROM ONLY $rid); \
           CREATE $item.receipt_rid CONTENT { \
             unlink_receipt_id: $item.unlink_receipt_id, collection_id: $member.collection_id, \
             asset_id: $member.asset_id, prior_sort_order: $member.sort_order, \
             prior_source_path_ref: $member.source_path_ref, \
             prior_source_url_ref: $member.source_url_ref, linked_by: $member.linked_by, \
             member_updated_by: $member.updated_by, \
             member_updated_at_utc: $member.updated_at_utc, unlinked_by: $domain.actor \
           }; \
           DELETE $rid; \
         }; \
       }; \
       IF $removed > 0 { \
         UPDATE $domain.collection_ref SET updated_by = $domain.actor, updated_at_utc = time::now(); \
       }; \
       RETURN $removed; };"
);

const LIST_COLLECTION_IMAGES_STATEMENT: &str = concat!(
    "SELECT ",
    collection_member_select!(),
    " FROM atelier_collection_item WHERE collection_id = $collection_ref \
      ORDER BY sort_order ASC, added_at_utc ASC;"
);

const TAG_MEDIA_ASSET_STATEMENT: &str = concat!(
    "RETURN { \
       LET $existing_tag = (SELECT VALUE id FROM atelier_tag \
                            WHERE text = $domain.text LIMIT 1); \
       IF $existing_tag = [] { \
         CREATE $domain.tag_rid CONTENT { tag_id: $domain.tag_id, text: $domain.text }; \
       }; \
       LET $tag_ref = (SELECT VALUE id FROM atelier_tag WHERE text = $domain.text LIMIT 1)[0]; \
       LET $rid = type::record('atelier_media_asset_tag', \
                              [record::id($domain.asset_ref), record::id($tag_ref)]); ",
    atelier_event_sql!(),
    " UPSERT $rid SET asset_id = $domain.asset_ref, tag_id = $tag_ref, source = $domain.source; \
       RETURN (SELECT record::id(asset_id) AS asset_id, record::id(tag_id) AS tag_id, \
                      tag_id.text AS text, source, created_at_utc FROM $rid); };"
);

const UNTAG_MEDIA_ASSET_STATEMENT: &str = "RETURN { \
       LET $tag_refs = (SELECT VALUE id FROM atelier_tag WHERE text = $text); \
       LET $removed = count(SELECT id FROM atelier_media_asset_tag \
                            WHERE asset_id = $asset_ref AND tag_id IN $tag_refs); \
       DELETE atelier_media_asset_tag WHERE asset_id = $asset_ref AND tag_id IN $tag_refs; \
       RETURN $removed > 0; };";

const LIST_MEDIA_ASSET_TAGS_STATEMENT: &str =
    "SELECT record::id(asset_id) AS asset_id, record::id(tag_id) AS tag_id, \
            tag_id.text AS text, source, created_at_utc \
     FROM atelier_media_asset_tag WHERE asset_id = $asset_ref \
     ORDER BY text ASC, tag_id ASC;";

const APPLY_COLLECTION_METADATA_STATEMENT: &str = concat!(
    "RETURN { \
       LET $members = (SELECT VALUE asset_id FROM atelier_collection_item \
                       WHERE collection_id = $domain.collection_ref \
                       ORDER BY sort_order ASC, added_at_utc ASC); \
       FOR $tag IN $domain.applied_tags { \
         LET $existing = (SELECT VALUE id FROM atelier_tag WHERE text = $tag.text LIMIT 1); \
         IF $existing = [] { CREATE $tag.tag_rid CONTENT { tag_id: $tag.tag_id, text: $tag.text }; }; \
       }; \
       LET $applied_tag_refs = (SELECT VALUE id FROM atelier_tag \
                               WHERE text IN $domain.applied_tags_json); \
       FOR $asset_ref IN $members { \
         FOR $tag_ref IN $applied_tag_refs { \
           LET $rid = type::record('atelier_media_asset_tag', \
                                  [record::id($asset_ref), record::id($tag_ref)]); \
           IF !record::exists($rid) { \
             CREATE $rid CONTENT { asset_id: $asset_ref, tag_id: $tag_ref, \
                                   source: $domain.tag_source }; \
           }; \
         }; \
       }; \
       LET $removed_tag_refs = (SELECT VALUE id FROM atelier_tag \
                               WHERE text IN $domain.removed_tags_json); \
       DELETE atelier_media_asset_tag WHERE asset_id IN $members \
                                        AND tag_id IN $removed_tag_refs; ",
    atelier_event_sql!(),
    " CREATE $domain.application_rid CONTENT { \
         application_id: $domain.application_id, collection_id: $domain.collection_ref, \
         requested_by: $domain.requested_by, applied_tags_json: $domain.applied_tags_json, \
         removed_tags_json: $domain.removed_tags_json, \
         affected_asset_count: array::len($members) \
       }; \
       RETURN (SELECT application_id, record::id(collection_id) AS collection_id, requested_by, \
                      applied_tags_json, removed_tags_json, affected_asset_count, created_at_utc \
               FROM $domain.application_rid); };"
);

const ASSET_HASHES_STATEMENT: &str =
    "SELECT asset_id, content_hash FROM atelier_media_asset WHERE asset_id IN $asset_ids;";

const CREATE_CONTACT_SHEET_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.sheet_rid; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { sheet_id: $domain.sheet_id, name: $domain.name, \
         source_type: $domain.source_type, source_id: $domain.source_id, \
         tags_json: $domain.tags_json, character_internal_id: $domain.character_ref, \
         sheet_version_id: $domain.sheet_version_ref, manifest: $domain.manifest, \
         image_count: $domain.image_count \
       }; RETURN (SELECT ",
    contact_sheet_select!(),
    " FROM $rid); };"
);

const GET_CONTACT_SHEET_STATEMENT: &str = concat!(
    "SELECT ",
    contact_sheet_select!(),
    " FROM atelier_contact_sheet WHERE sheet_id = $sheet_id LIMIT 1;"
);

const LIST_CONTACT_SHEETS_STATEMENT: &str = concat!(
    "SELECT ",
    contact_sheet_select!(),
    " FROM atelier_contact_sheet \
      WHERE $source_type = NONE OR source_type = $source_type \
      ORDER BY created_at_utc DESC, sheet_id ASC;"
);

const FIND_SVG_ARTIFACT_STATEMENT: &str = concat!(
    "SELECT ",
    svg_artifact_select!(),
    " FROM atelier_contact_sheet_svg_artifact \
      WHERE sheet_id = $sheet_ref AND manifest_hash = $manifest_hash LIMIT 1;"
);

const CREATE_SVG_ARTIFACT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $created = !record::exists($artifact_rid); \
       IF $created { \
         CREATE $artifact_rid CONTENT { svg_artifact_id: $svg_artifact_id, \
           sheet_id: $sheet_ref, manifest_hash: $manifest_hash, content_hash: $content_hash, \
           artifact_ref: $artifact_ref, svg_text: $svg_text, image_count: $image_count }; \
       }; RETURN { created: $created, artifact: (SELECT ",
    svg_artifact_select!(),
    " FROM $artifact_rid) }; };"
);

impl AtelierStore {
    /// Create a named collection. `name` must be non-empty and is unique. Tags
    /// are trimmed and de-duplicated. Optional character/sheet links are FK
    /// validated by the database.
    pub async fn create_collection(&self, new: &NewCollection) -> AtelierResult<Collection> {
        self.create_collection_inner(new, None).await
    }

    /// Create a named collection attributed to the requesting actor
    /// (`created_by` / `updated_by` and the EventLedger payload).
    pub async fn create_collection_attributed(
        &self,
        new: &NewCollection,
        requested_by: &str,
    ) -> AtelierResult<Collection> {
        self.create_collection_inner(new, Some(requested_by)).await
    }

    async fn create_collection_inner(
        &self,
        new: &NewCollection,
        requested_by: Option<&str>,
    ) -> AtelierResult<Collection> {
        let name = new.name.trim();
        if name.is_empty() {
            return Err(AtelierError::Validation(
                "collection name must not be empty".into(),
            ));
        }
        let actor = collection_actor_or_system("collection actor", requested_by)?;
        let tags = clean_tags(&new.tags);
        let collection_id = Uuid::now_v7();
        let bindings = CreateCollectionBindings {
            collection_rid: collection_record(collection_id),
            collection_id: SurrealUuid::from(collection_id),
            name: name.to_owned(),
            notes: new.notes.clone(),
            tags_json: tags.clone(),
            character_ref: new
                .character_internal_id
                .map(|id| RecordId::new("atelier_character", SurrealUuid::from(id))),
            sheet_version_ref: new
                .sheet_version_id
                .map(|id| RecordId::new("atelier_sheet_version", SurrealUuid::from(id))),
            actor: actor.to_owned(),
        };
        let row: Option<CollectionRow> = self
            .write_with_event(
                CREATE_COLLECTION_STATEMENT,
                bindings,
                collections_event_family::COLLECTION_CREATED,
                "atelier_collection",
                &collection_id.to_string(),
                serde_json::json!({
                    "name": name,
                    "tags": tags,
                    "character_scoped": new.character_internal_id.is_some(),
                    "requested_by": actor,
                }),
            )
            .await
            .map_err(|error| map_scoped_name_conflict(error, name, new.character_internal_id))?;
        row.map(Collection::from).ok_or_else(|| {
            AtelierError::Internal("creating a collection returned no row".to_owned())
        })
    }

    /// One page of the collections bound to a CKC character, newest first,
    /// plus the canonical total so the caller can compute `next_offset` from
    /// the real row count rather than the rendered subset (MT-056 F2/F3).
    pub async fn list_character_collections_page(
        &self,
        character_internal_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> AtelierResult<CollectionPage> {
        if offset < 0 {
            return Err(AtelierError::Validation("offset must be >= 0".to_owned()));
        }
        if limit < 1 {
            return Err(AtelierError::Validation("limit must be >= 1".to_owned()));
        }
        let character_ref = RecordId::new("atelier_character", SurrealUuid::from(character_internal_id));
        let count_bindings = CharacterRefBinding {
            character_ref: character_ref.clone(),
        };
        let total_count: Option<i64> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(COUNT_CHARACTER_COLLECTIONS_STATEMENT, count_bindings)
                        .await
                })
            })
            .await?;
        let page_bindings = CharacterCollectionPageBindings {
            character_ref,
            limit,
            offset,
        };
        let rows: Vec<CollectionRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_CHARACTER_COLLECTIONS_PAGE_STATEMENT, page_bindings)
                        .await
                })
            })
            .await?;
        Ok(CollectionPage {
            collections: rows.into_iter().map(Collection::from).collect(),
            total_count: total_count.unwrap_or(0).max(0),
        })
    }

    /// One page of album members with the asset fields and link-scoped refs,
    /// ordered by `sort_order, asset_id`, plus the canonical member count.
    pub async fn list_collection_member_page(
        &self,
        collection_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> AtelierResult<CollectionMemberPage> {
        if offset < 0 {
            return Err(AtelierError::Validation("offset must be >= 0".to_owned()));
        }
        if limit < 1 {
            return Err(AtelierError::Validation("limit must be >= 1".to_owned()));
        }
        let total_count = self.count_collection_members(collection_id).await?;
        let page_bindings = CollectionMemberPageBindings {
            collection_ref: collection_record(collection_id),
            limit,
            offset,
        };
        let rows: Vec<CollectionMemberDetailRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_COLLECTION_MEMBER_PAGE_STATEMENT, page_bindings)
                        .await
                })
            })
            .await?;
        Ok(CollectionMemberPage {
            members: rows.into_iter().map(CollectionMemberDetail::from).collect(),
            total_count,
        })
    }

    /// Canonical member count of a collection (the number the API reports,
    /// never the size of a rendered page).
    pub async fn count_collection_members(&self, collection_id: Uuid) -> AtelierResult<i64> {
        let bindings = CollectionRefBinding {
            collection_ref: collection_record(collection_id),
        };
        let count: Option<i64> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(COUNT_COLLECTION_MEMBERS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(count.unwrap_or(0).max(0))
    }

    /// Current member asset ids of a collection in album order.
    pub async fn list_collection_member_asset_ids(
        &self,
        collection_id: Uuid,
    ) -> AtelierResult<Vec<Uuid>> {
        let bindings = CollectionRefBinding {
            collection_ref: collection_record(collection_id),
        };
        let ids: Vec<SurrealUuid> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_COLLECTION_MEMBER_ASSET_IDS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(ids.into_iter().map(Into::into).collect())
    }

    /// Review metadata, direct tags and asset-global provenance for a set of
    /// assets in three batched statements. Assets with no rows come back with
    /// `None` / empty tags so the caller can render every member uniformly.
    pub async fn media_album_member_enrichment(
        &self,
        asset_ids: &[Uuid],
    ) -> AtelierResult<Vec<MediaAlbumMemberEnrichment>> {
        if asset_ids.is_empty() {
            return Ok(Vec::new());
        }
        let asset_refs: Vec<RecordId> = asset_ids.iter().copied().map(media_asset_record).collect();
        let review_bindings = AssetRefsBinding {
            asset_refs: asset_refs.clone(),
        };
        let review_rows: Vec<MemberReviewMetadataRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(MEMBER_REVIEW_METADATA_BATCH_STATEMENT, review_bindings)
                        .await
                })
            })
            .await?;
        let tag_bindings = AssetRefsBinding {
            asset_refs: asset_refs.clone(),
        };
        let tag_rows: Vec<MemberTagTextRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(MEMBER_TAGS_BATCH_STATEMENT, tag_bindings)
                        .await
                })
            })
            .await?;
        let provenance_bindings = AssetRefsBinding { asset_refs };
        let provenance_rows: Vec<MemberProvenanceRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(MEMBER_PROVENANCE_BATCH_STATEMENT, provenance_bindings)
                        .await
                })
            })
            .await?;

        let mut metadata_by_asset: HashMap<Uuid, MediaReviewMetadata> = HashMap::new();
        for row in review_rows {
            let metadata: MediaReviewMetadata = row.try_into()?;
            metadata_by_asset.insert(metadata.asset_id, metadata);
        }
        let mut tags_by_asset: HashMap<Uuid, Vec<String>> = HashMap::new();
        for row in tag_rows {
            tags_by_asset
                .entry(row.asset_id.into())
                .or_default()
                .push(row.text);
        }
        let mut provenance_by_asset: HashMap<Uuid, MediaSourceProvenanceRefs> = HashMap::new();
        for row in provenance_rows {
            let provenance: MediaSourceProvenanceRefs = row.into();
            provenance_by_asset.insert(provenance.asset_id, provenance);
        }
        Ok(asset_ids
            .iter()
            .map(|asset_id| {
                let mut tags = tags_by_asset.remove(asset_id).unwrap_or_default();
                tags.sort();
                tags.dedup();
                MediaAlbumMemberEnrichment {
                    asset_id: *asset_id,
                    metadata: metadata_by_asset.remove(asset_id),
                    tags,
                    provenance: provenance_by_asset.remove(asset_id),
                }
            })
            .collect())
    }

    /// Fetch a collection by id.
    pub async fn get_collection(&self, collection_id: Uuid) -> AtelierResult<Collection> {
        let bindings = CollectionIdBinding {
            collection_id: SurrealUuid::from(collection_id),
        };
        let row: Option<CollectionRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_COLLECTION_STATEMENT, bindings).await })
            })
            .await?;
        row.map(Collection::from)
            .ok_or_else(|| AtelierError::NotFound(format!("collection_id={collection_id}")))
    }

    /// List collections (most recently updated first).
    pub async fn list_collections(&self) -> AtelierResult<Vec<Collection>> {
        let rows: Vec<CollectionRow> = self
            .store()
            .with_data_operation(|ctx| {
                Box::pin(async move { ctx.query_values(LIST_COLLECTIONS_STATEMENT, ()).await })
            })
            .await?;
        Ok(rows.into_iter().map(Collection::from).collect())
    }

    /// Update mutable fields of a collection (name, notes, tags). `None` leaves
    /// a field unchanged. Bumps `updated_at_utc`.
    pub async fn update_collection(
        &self,
        collection_id: Uuid,
        name: Option<&str>,
        notes: Option<&str>,
        tags: Option<&[String]>,
    ) -> AtelierResult<Collection> {
        if let Some(n) = name {
            if n.trim().is_empty() {
                return Err(AtelierError::Validation(
                    "collection name must not be empty".into(),
                ));
            }
        }
        self.get_collection(collection_id).await?;
        let tags_cleaned = tags.map(clean_tags);
        let bindings = UpdateCollectionBindings {
            collection_rid: collection_record(collection_id),
            name: name.map(|value| value.trim().to_owned()),
            notes: notes.map(ToOwned::to_owned),
            tags_json: tags_cleaned.clone(),
            actor: SYSTEM_COLLECTION_ACTOR.to_owned(),
        };
        let row: Option<CollectionRow> = self
            .write_with_event(
                UPDATE_COLLECTION_STATEMENT,
                bindings,
                collections_event_family::COLLECTION_UPDATED,
                "atelier_collection",
                &collection_id.to_string(),
                serde_json::json!({
                    "name": name.map(str::trim),
                    "tags": tags_cleaned,
                }),
            )
            .await?;
        row.map(Collection::from)
            .ok_or_else(|| AtelierError::NotFound(format!("collection_id={collection_id}")))
    }

    /// Append media assets to a collection in the given order. Existing
    /// memberships are ignored (idempotent via ON CONFLICT). Returns the number
    /// of newly inserted memberships. Bumps the collection `updated_at_utc`.
    pub async fn add_images_to_collection(
        &self,
        collection_id: Uuid,
        asset_ids: &[Uuid],
    ) -> AtelierResult<i64> {
        Ok(self
            .add_images_to_collection_inner(collection_id, asset_ids, None, None, None)
            .await?
            .inserted)
    }

    /// Append media assets and record the requesting actor on the membership
    /// rows and in the EventLedger payload.
    pub async fn add_images_to_collection_attributed(
        &self,
        collection_id: Uuid,
        asset_ids: &[Uuid],
        requested_by: &str,
    ) -> AtelierResult<i64> {
        Ok(self
            .add_images_to_collection_inner(collection_id, asset_ids, Some(requested_by), None, None)
            .await?
            .inserted)
    }

    /// Append media assets and persist optional link-scoped provenance on the
    /// collection membership (never on the asset identity). Pre-existing
    /// memberships whose stored ref differs from a supplied ref are updated in
    /// place and counted in `updated_refs`.
    pub async fn add_images_to_collection_with_link_refs_attributed(
        &self,
        collection_id: Uuid,
        asset_ids: &[Uuid],
        source_path_ref: Option<&str>,
        source_url_ref: Option<&str>,
        requested_by: &str,
    ) -> AtelierResult<CollectionItemsAdded> {
        self.add_images_to_collection_inner(
            collection_id,
            asset_ids,
            Some(requested_by),
            source_path_ref,
            source_url_ref,
        )
        .await
    }

    async fn add_images_to_collection_inner(
        &self,
        collection_id: Uuid,
        asset_ids: &[Uuid],
        requested_by: Option<&str>,
        source_path_ref: Option<&str>,
        source_url_ref: Option<&str>,
    ) -> AtelierResult<CollectionItemsAdded> {
        let source_path_ref = normalize_optional_ckc_source_ref(
            CkcSourceRefKind::Folder,
            "source_path_ref",
            &source_path_ref.map(ToOwned::to_owned),
        )?;
        let source_url_ref = normalize_optional_ckc_source_ref(
            CkcSourceRefKind::SourceUrl,
            "source_url_ref",
            &source_url_ref.map(ToOwned::to_owned),
        )?;
        let actor = collection_actor_or_system("collection item actor", requested_by)?;
        // Validate the collection exists (clear error vs. an FK violation).
        self.get_collection(collection_id).await?;
        let mut unique = Vec::new();
        for asset_id in asset_ids {
            if !unique.contains(asset_id) {
                unique.push(*asset_id);
            }
        }
        // Reject unknown assets with a typed NotFound naming them, instead of
        // leaking the schema's record-link assertion as a 500.
        if !unique.is_empty() {
            let bindings = AssetIdsBinding {
                asset_ids: unique.iter().copied().map(SurrealUuid::from).collect(),
            };
            let rows: Vec<AssetHashRow> = self
                .store()
                .with_data_operation(move |ctx| {
                    Box::pin(
                        async move { ctx.query_values(ASSET_HASHES_STATEMENT, bindings).await },
                    )
                })
                .await?;
            if rows.len() != unique.len() {
                let existing: HashSet<Uuid> =
                    rows.into_iter().map(|row| row.asset_id.into()).collect();
                let missing: Vec<String> = unique
                    .iter()
                    .filter(|asset_id| !existing.contains(asset_id))
                    .map(Uuid::to_string)
                    .collect();
                return Err(AtelierError::NotFound(format!(
                    "collection media targets missing from atelier_media_asset: {}",
                    missing.join(", ")
                )));
            }
        }
        // Offsets are assigned only to assets that are not members yet so the
        // appended block stays dense; the statement re-checks existence so a
        // concurrent link cannot duplicate a membership.
        let current_members: HashSet<Uuid> = self
            .list_collection_member_asset_ids(collection_id)
            .await?
            .into_iter()
            .collect();
        let collection_ref = collection_record(collection_id);
        let asset_refs: Vec<RecordId> = unique.iter().copied().map(media_asset_record).collect();
        let mut next_offset = 0_i64;
        let items = unique
            .iter()
            .zip(asset_refs.iter())
            .map(|(asset_id, asset_ref)| {
                let order_offset = if current_members.contains(asset_id) {
                    0
                } else {
                    let offset = next_offset;
                    next_offset += 1;
                    offset
                };
                CollectionItemInput {
                    asset_ref: asset_ref.clone(),
                    pair_key: collection_item_pair_key(collection_id, *asset_id),
                    order_offset,
                }
            })
            .collect();
        let bindings = AddImagesBindings {
            collection_ref,
            asset_refs,
            items,
            source_path_ref: source_path_ref.clone(),
            source_url_ref: source_url_ref.clone(),
            actor: actor.to_owned(),
        };
        let aggregate_id = collection_id.to_string();
        let payload = serde_json::json!({
            "requested": asset_ids.len(),
            "unique_requested": unique.len(),
            "source_path_ref": source_path_ref,
            "source_url_ref": source_url_ref,
            "requested_by": actor,
        });
        let result: Option<CollectionItemsAddedRow> = self
            .run_album_mutation_with_retry(collection_id, || {
                self.write_with_event(
                    ADD_IMAGES_STATEMENT,
                    bindings.clone(),
                    collections_event_family::COLLECTION_IMAGES_ADDED,
                    "atelier_collection",
                    &aggregate_id,
                    payload.clone(),
                )
            })
            .await
            .map_err(|error| map_album_mutation_error(error, collection_id, None))?;
        let result = result.ok_or_else(|| {
            AtelierError::Internal("adding collection images returned no count".to_owned())
        })?;
        Ok(CollectionItemsAdded {
            inserted: result.inserted,
            updated_refs: result.updated_refs,
        })
    }

    /// Remove media assets from a collection, writing one unlink receipt per
    /// removed membership (attributed to the system actor). Returns the number
    /// removed. Bumps `updated_at_utc` when anything was removed.
    pub async fn remove_images_from_collection(
        &self,
        collection_id: Uuid,
        asset_ids: &[Uuid],
    ) -> AtelierResult<i64> {
        self.get_collection(collection_id).await?;
        let mut unique = Vec::new();
        for asset_id in asset_ids {
            if !unique.contains(asset_id) {
                unique.push(*asset_id);
            }
        }
        let items: Vec<RemoveImageInput> = unique
            .iter()
            .map(|asset_id| {
                let unlink_receipt_id = Uuid::now_v7();
                RemoveImageInput {
                    asset_ref: media_asset_record(*asset_id),
                    pair_key: collection_item_pair_key(collection_id, *asset_id),
                    receipt_rid: RecordId::new(
                        "atelier_collection_item_unlink_receipt",
                        SurrealUuid::from(unlink_receipt_id),
                    ),
                    unlink_receipt_id: SurrealUuid::from(unlink_receipt_id),
                }
            })
            .collect();
        let unlink_receipt_ids: Vec<Uuid> = items
            .iter()
            .map(|item| item.unlink_receipt_id.into())
            .collect();
        let bindings = RemoveImagesBindings {
            collection_ref: collection_record(collection_id),
            asset_refs: unique.iter().copied().map(media_asset_record).collect(),
            items,
            actor: SYSTEM_COLLECTION_ACTOR.to_owned(),
        };
        let removed: Option<i64> = self
            .write_with_event(
                REMOVE_IMAGES_STATEMENT,
                bindings,
                collections_event_family::COLLECTION_IMAGES_REMOVED,
                "atelier_collection",
                &collection_id.to_string(),
                serde_json::json!({
                    "requested": asset_ids.len(),
                    "unique_requested": unique.len(),
                    "removed_by": SYSTEM_COLLECTION_ACTOR,
                    "unlink_receipt_ids": unlink_receipt_ids,
                }),
            )
            .await?;
        removed.ok_or_else(|| {
            AtelierError::Internal("removing collection images returned no count".to_owned())
        })
    }

    /// Retry an album mutation whose statement lost an optimistic transaction
    /// race (two writers on one album, MT-056 F5/F9). Every other error, and an
    /// exhausted budget, is returned to the caller unchanged.
    async fn run_album_mutation_with_retry<T, F, Fut>(
        &self,
        seed: Uuid,
        mut attempt: F,
    ) -> AtelierResult<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = AtelierResult<T>>,
    {
        let mut failed_attempts = 0_usize;
        loop {
            match attempt().await {
                Ok(value) => return Ok(value),
                Err(error)
                    if is_surreal_retryable_transaction_conflict(&error)
                        && failed_attempts + 1 < SURREAL_TRANSACTION_MAX_ATTEMPTS =>
                {
                    failed_attempts += 1;
                    tokio::time::sleep(surreal_transaction_retry_delay(seed, failed_attempts)).await;
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// Unlink ONE membership from a CKC media album: the membership row is
    /// copied into `atelier_collection_item_unlink_receipt` and deleted, the
    /// collection is bumped, and `MEDIA_ALBUM_ITEM_UNLINKED` is recorded, all
    /// in one statement. The media asset row, its notes, tags and asset-global
    /// provenance are untouched. A non-member is `NotFound`.
    pub async fn unlink_media_album_item(
        &self,
        collection_id: Uuid,
        asset_id: Uuid,
        requested_by: &str,
    ) -> AtelierResult<CollectionItemUnlinkReceipt> {
        let actor = require_collection_actor("collection item actor", requested_by)?;
        self.get_collection(collection_id).await?;
        let unlink_receipt_id = Uuid::now_v7();
        let bindings = UnlinkAlbumItemBindings {
            collection_ref: collection_record(collection_id),
            member_key: collection_item_pair_key(collection_id, asset_id),
            receipt_rid: RecordId::new(
                "atelier_collection_item_unlink_receipt",
                SurrealUuid::from(unlink_receipt_id),
            ),
            unlink_receipt_id: SurrealUuid::from(unlink_receipt_id),
            actor: actor.to_owned(),
        };
        let aggregate_id = collection_id.to_string();
        let payload = serde_json::json!({
            "mutation": "unlink",
            "requested_by": actor,
            "collection_id": collection_id,
            "asset_id": asset_id,
            "removed": 1,
            "removed_by": actor,
            "unlink_receipt_id": unlink_receipt_id,
            "concurrency_policy": ALBUM_MUTATION_CONCURRENCY_POLICY,
            "asset_preserved": true,
        });
        let row: Option<CollectionItemUnlinkReceiptRow> = self
            .run_album_mutation_with_retry(collection_id, || {
                self.write_with_event(
                    UNLINK_ALBUM_ITEM_STATEMENT,
                    bindings.clone(),
                    collections_event_family::MEDIA_ALBUM_ITEM_UNLINKED,
                    "atelier_collection",
                    &aggregate_id,
                    payload.clone(),
                )
            })
            .await
            .map_err(|error| map_album_mutation_error(error, collection_id, Some(asset_id)))?;
        row.map(CollectionItemUnlinkReceipt::from).ok_or_else(|| {
            AtelierError::Internal("unlinking an album item returned no receipt".to_owned())
        })
    }

    /// Fetch one unlink receipt by id.
    pub async fn get_collection_item_unlink_receipt(
        &self,
        unlink_receipt_id: Uuid,
    ) -> AtelierResult<Option<CollectionItemUnlinkReceipt>> {
        let bindings = UnlinkReceiptIdBinding {
            unlink_receipt_id: SurrealUuid::from(unlink_receipt_id),
        };
        let row: Option<CollectionItemUnlinkReceiptRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_UNLINK_RECEIPT_STATEMENT, bindings).await })
            })
            .await?;
        Ok(row.map(Into::into))
    }

    /// Edit the link-scoped provenance on ONE membership. Returns the number
    /// of membership rows updated (always 1; a non-member is `NotFound`).
    pub async fn update_media_album_item_link_refs(
        &self,
        collection_id: Uuid,
        asset_id: Uuid,
        update: &CollectionItemLinkRefUpdate,
        requested_by: &str,
    ) -> AtelierResult<i64> {
        let actor = require_collection_actor("collection item actor", requested_by)?;
        if !update.set_source_path_ref && !update.set_source_url_ref {
            return Err(AtelierError::Validation(
                "at least one link-scoped provenance field must be set or cleared".to_owned(),
            ));
        }
        let source_path_ref = if update.set_source_path_ref {
            normalize_optional_ckc_source_ref(
                CkcSourceRefKind::Folder,
                "source_path_ref",
                &update.source_path_ref,
            )?
        } else {
            None
        };
        let source_url_ref = if update.set_source_url_ref {
            normalize_optional_ckc_source_ref(
                CkcSourceRefKind::SourceUrl,
                "source_url_ref",
                &update.source_url_ref,
            )?
        } else {
            None
        };
        self.get_collection(collection_id).await?;
        let bindings = UpdateAlbumItemLinkRefsBindings {
            collection_ref: collection_record(collection_id),
            member_key: collection_item_pair_key(collection_id, asset_id),
            set_source_path_ref: update.set_source_path_ref,
            source_path_ref: source_path_ref.clone(),
            set_source_url_ref: update.set_source_url_ref,
            source_url_ref: source_url_ref.clone(),
            actor: actor.to_owned(),
        };
        let aggregate_id = collection_id.to_string();
        let payload = serde_json::json!({
            "mutation": "link_ref_edit",
            "requested_by": actor,
            "collection_id": collection_id,
            "asset_id": asset_id,
            "set_source_path_ref": update.set_source_path_ref,
            "source_path_ref": source_path_ref,
            "set_source_url_ref": update.set_source_url_ref,
            "source_url_ref": source_url_ref,
            "concurrency_policy": ALBUM_MUTATION_CONCURRENCY_POLICY,
            "global_asset_provenance_preserved": true,
        });
        let updated: Option<i64> = self
            .run_album_mutation_with_retry(collection_id, || {
                self.write_with_event(
                    UPDATE_ALBUM_ITEM_LINK_REFS_STATEMENT,
                    bindings.clone(),
                    collections_event_family::MEDIA_ALBUM_ITEM_LINK_REFS_UPDATED,
                    "atelier_collection",
                    &aggregate_id,
                    payload.clone(),
                )
            })
            .await
            .map_err(|error| map_album_mutation_error(error, collection_id, Some(asset_id)))?;
        updated.ok_or_else(|| {
            AtelierError::Internal("editing an album item link returned no count".to_owned())
        })
    }

    /// Apply an explicit full dense reorder to a CKC media album. The request
    /// must name every current member exactly once with positions `0..n`; a
    /// set that does not match the album is `Validation` (wrong size) or
    /// `NotFound` (foreign asset), and a set that stops matching while the
    /// write is in flight is `Conflict`. Returns the number of rows reordered.
    pub async fn reorder_media_album_items(
        &self,
        collection_id: Uuid,
        items: &[CollectionItemReorder],
        requested_by: &str,
    ) -> AtelierResult<i64> {
        let actor = require_collection_actor("collection item actor", requested_by)?;
        let asset_ids = validate_collection_reorder(items)?;
        self.get_collection(collection_id).await?;
        let current_members = self.list_collection_member_asset_ids(collection_id).await?;
        if current_members.len() != asset_ids.len() {
            return Err(AtelierError::Validation(format!(
                "reorder must include every current album member for collection_id={collection_id}; current={} requested={}",
                current_members.len(),
                asset_ids.len()
            )));
        }
        let current_set: HashSet<Uuid> = current_members.into_iter().collect();
        let missing: Vec<String> = asset_ids
            .iter()
            .filter(|asset_id| !current_set.contains(asset_id))
            .map(Uuid::to_string)
            .collect();
        if !missing.is_empty() {
            return Err(AtelierError::NotFound(format!(
                "album reorder targets missing from collection_id={collection_id}: {}",
                missing.join(", ")
            )));
        }
        let bindings = ReorderAlbumItemsBindings {
            collection_ref: collection_record(collection_id),
            expected_asset_ids: asset_ids.iter().copied().map(SurrealUuid::from).collect(),
            items: items
                .iter()
                .map(|item| ReorderItemInput {
                    member_key: collection_item_pair_key(collection_id, item.asset_id),
                    sort_order: item.sort_order,
                })
                .collect(),
            actor: actor.to_owned(),
        };
        let aggregate_id = collection_id.to_string();
        let payload = serde_json::json!({
            "mutation": "reorder",
            "requested_by": actor,
            "collection_id": collection_id,
            "requested": items.len(),
            "changed": items.len(),
            "concurrency_policy": ALBUM_REORDER_CONCURRENCY_POLICY,
            "items": items
                .iter()
                .map(|item| serde_json::json!({
                    "asset_id": item.asset_id,
                    "sort_order": item.sort_order,
                }))
                .collect::<Vec<_>>(),
        });
        let changed: Option<i64> = self
            .run_album_mutation_with_retry(collection_id, || {
                self.write_with_event(
                    REORDER_ALBUM_ITEMS_STATEMENT,
                    bindings.clone(),
                    collections_event_family::MEDIA_ALBUM_ITEMS_REORDERED,
                    "atelier_collection",
                    &aggregate_id,
                    payload.clone(),
                )
            })
            .await
            .map_err(|error| map_album_mutation_error(error, collection_id, None))?;
        changed.ok_or_else(|| {
            AtelierError::Internal("reordering album items returned no count".to_owned())
        })
    }

    /// The CKC "notes/tags" save for one media asset. Every input is validated
    /// (including the merged asset-global refs) BEFORE any write, so a rejected
    /// request leaves notes, tags and provenance exactly as they were. The
    /// review-metadata upsert, the tag-set replacement and the optional
    /// provenance upsert then commit in one statement together with the
    /// `MEDIA_REVIEW_METADATA_UPDATED` event; the per-tag and provenance events
    /// are appended after that commit.
    pub async fn apply_media_notes_tags(
        &self,
        update: &MediaNotesTagsUpdate,
    ) -> AtelierResult<MediaNotesTagsResult> {
        let actor = require_collection_actor("media notes actor", &update.updated_by)?;
        let asset_id = update.asset_id;
        let requested_path_ref = normalize_optional_ckc_source_ref(
            CkcSourceRefKind::Folder,
            "source_path_ref",
            &update.source_path_ref,
        )?;
        let requested_url_ref = normalize_optional_ckc_source_ref(
            CkcSourceRefKind::SourceUrl,
            "source_url_ref",
            &update.source_url_ref,
        )?;
        let desired_tags = update.tags.as_deref().map(normalize_media_tags);

        if self.get_media_asset(asset_id).await?.is_none() {
            return Err(AtelierError::NotFound(format!("media asset_id={asset_id}")));
        }
        let existing_metadata = self.get_media_review_metadata(asset_id).await?;
        let existing_tags: Vec<String> = self
            .list_media_asset_tags(asset_id)
            .await?
            .into_iter()
            .map(|tag| tag.text)
            .collect();
        let existing_provenance = self.get_media_source_provenance_refs(asset_id).await?;

        let notes = update
            .notes
            .clone()
            .or_else(|| existing_metadata.as_ref().and_then(|row| row.notes.clone()));
        let review_status = match update.review_status.as_deref() {
            Some(status) => require_media_review_status(status)?.to_owned(),
            None => existing_metadata
                .as_ref()
                .map(|row| row.review_status.clone())
                .unwrap_or_else(|| "unreviewed".to_owned()),
        };
        let (added_tags, removed_tags) = match &desired_tags {
            Some(desired) => (
                desired
                    .iter()
                    .filter(|tag| !existing_tags.contains(tag))
                    .cloned()
                    .collect::<Vec<_>>(),
                existing_tags
                    .iter()
                    .filter(|tag| !desired.contains(tag))
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
            None => (Vec::new(), Vec::new()),
        };

        let write_provenance = requested_path_ref.is_some() || requested_url_ref.is_some();
        let final_url_ref = requested_url_ref
            .clone()
            .or_else(|| existing_provenance.as_ref().and_then(|row| row.source_url_ref.clone()));
        let final_path_ref = requested_path_ref
            .clone()
            .or_else(|| existing_provenance.as_ref().and_then(|row| row.source_path_ref.clone()));
        let source_note_ref = existing_provenance.as_ref().and_then(|row| row.source_note_ref.clone());
        let contact_sheet_ref = existing_provenance.as_ref().and_then(|row| row.contact_sheet_ref.clone());
        let task_ref = existing_provenance.as_ref().and_then(|row| row.task_ref.clone());
        let run_ref = existing_provenance.as_ref().and_then(|row| row.run_ref.clone());
        if write_provenance {
            // The merged row must pass the same rules as a fresh write: a stored
            // ref that drifted invalid cannot be re-persisted alongside the edit.
            normalize_optional_ckc_source_ref(CkcSourceRefKind::SourceUrl, "source_url_ref", &final_url_ref)?;
            normalize_optional_ckc_source_ref(CkcSourceRefKind::Folder, "source_path_ref", &final_path_ref)?;
            for (field, value) in [
                ("source_note_ref", &source_note_ref),
                ("contact_sheet_ref", &contact_sheet_ref),
                ("task_ref", &task_ref),
                ("run_ref", &run_ref),
            ] {
                if let Some(raw) = value.as_deref() {
                    require_collection_ref_text(field, raw)?;
                }
            }
        }

        let tag_candidates: Vec<NotesTagCandidate> = added_tags
            .iter()
            .map(|text| {
                let tag_id = Uuid::now_v7();
                NotesTagCandidate {
                    tag_rid: RecordId::new("atelier_tag", SurrealUuid::from(tag_id)),
                    tag_id: SurrealUuid::from(tag_id),
                    text: text.clone(),
                }
            })
            .collect();
        let bindings = ApplyMediaNotesTagsBindings {
            asset_ref: media_asset_record(asset_id),
            metadata_rid: RecordId::new("atelier_media_review_metadata", SurrealUuid::from(asset_id)),
            provenance_rid: RecordId::new(
                "atelier_media_source_provenance_ref",
                SurrealUuid::from(asset_id),
            ),
            replace_tags: desired_tags.is_some(),
            added_tags: tag_candidates,
            removed_tags: removed_tags.clone(),
            tag_source: actor.to_owned(),
            favorite: existing_metadata.as_ref().map(|row| row.favorite).unwrap_or(false),
            rating: existing_metadata.as_ref().map(|row| i64::from(row.rating)).unwrap_or(0),
            frontpage: existing_metadata.as_ref().map(|row| row.frontpage).unwrap_or(false),
            carousel: existing_metadata.as_ref().map(|row| row.carousel).unwrap_or(false),
            notes: notes.clone(),
            review_status: review_status.clone(),
            write_provenance,
            source_url_ref: final_url_ref.clone(),
            source_path_ref: final_path_ref.clone(),
            source_note_ref: source_note_ref.clone(),
            contact_sheet_ref: contact_sheet_ref.clone(),
            task_ref: task_ref.clone(),
            run_ref: run_ref.clone(),
            actor: actor.to_owned(),
        };
        let snapshot: Option<MediaNotesTagsSnapshotRow> = self
            .write_with_event(
                APPLY_MEDIA_NOTES_TAGS_STATEMENT,
                bindings,
                event_family::MEDIA_REVIEW_METADATA_UPDATED,
                "atelier_media_review_metadata",
                &asset_id.to_string(),
                serde_json::json!({
                    "asset_id": asset_id,
                    "favorite": existing_metadata.as_ref().map(|row| row.favorite).unwrap_or(false),
                    "rating": existing_metadata.as_ref().map(|row| row.rating).unwrap_or(0),
                    "frontpage": existing_metadata.as_ref().map(|row| row.frontpage).unwrap_or(false),
                    "carousel": existing_metadata.as_ref().map(|row| row.carousel).unwrap_or(false),
                    "review_status": review_status,
                    "notes_present": notes.is_some(),
                    "notes_ref": notes.as_deref().map(event_ref_for_text),
                    "requested_by": actor,
                }),
            )
            .await
            .map_err(|error| map_album_mutation_error(error, asset_id, Some(asset_id)))?;
        let snapshot = snapshot.ok_or_else(|| {
            AtelierError::Internal("saving media notes/tags returned no snapshot".to_owned())
        })?;
        let metadata: MediaReviewMetadata = snapshot
            .metadata
            .into_iter()
            .next()
            .ok_or_else(|| {
                AtelierError::Internal("saving media notes/tags returned no review row".to_owned())
            })?
            .try_into()?;
        let mut tags = snapshot.tags;
        tags.sort();
        tags.dedup();
        let provenance = snapshot.provenance.into_iter().next().map(Into::into);

        for text in &removed_tags {
            self.record_event(
                collections_event_family::MEDIA_ASSET_UNTAGGED,
                "atelier_media_asset_tag",
                &event_ref_for_text(&format!("media-asset-untag:{asset_id}:{text}")),
                serde_json::json!({ "asset_id": asset_id, "text": text }),
            )
            .await?;
        }
        for text in &added_tags {
            self.record_event(
                collections_event_family::MEDIA_ASSET_TAGGED,
                "atelier_media_asset_tag",
                &event_ref_for_text(&format!("media-asset-tag:{asset_id}:{text}")),
                serde_json::json!({
                    "asset_id": asset_id,
                    "text": text,
                    "tag_source_ref": event_ref_for_text(actor),
                }),
            )
            .await?;
        }
        if write_provenance {
            self.record_event(
                event_family::MEDIA_SOURCE_PROVENANCE_REFS_SET,
                "atelier_media_asset",
                &asset_id.to_string(),
                serde_json::json!({
                    "asset_id": asset_id,
                    "source_url_ref": final_url_ref,
                    "source_path_ref": final_path_ref,
                    "source_note_ref": source_note_ref,
                    "contact_sheet_ref": contact_sheet_ref,
                    "task_ref": task_ref,
                    "run_ref": run_ref,
                    "updated_by": actor,
                }),
            )
            .await?;
        }

        Ok(MediaNotesTagsResult {
            metadata,
            tags,
            provenance,
            added_tags,
            removed_tags,
        })
    }

    /// List a collection's members in membership order, resolved to their media
    /// asset content hashes.
    pub async fn list_collection_images(
        &self,
        collection_id: Uuid,
    ) -> AtelierResult<Vec<CollectionMember>> {
        let bindings = CollectionRefBinding {
            collection_ref: RecordId::new("atelier_collection", SurrealUuid::from(collection_id)),
        };
        let rows: Vec<CollectionMemberRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_COLLECTION_IMAGES_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(rows.into_iter().map(CollectionMember::from).collect())
    }

    /// Attach one normalized tag directly to a media asset. This is the
    /// per-photo tag surface used by collection batch metadata application.
    pub async fn tag_media_asset(
        &self,
        asset_id: Uuid,
        text: &str,
        source: &str,
    ) -> AtelierResult<MediaAssetTag> {
        let source = require_collection_ref_text("source", source)?;
        let normalized = normalize_tag(text);
        if normalized.is_empty() {
            return Err(AtelierError::Validation(
                "tag text must not be empty".into(),
            ));
        }
        let exists_bindings = AssetIdBinding {
            asset_id: SurrealUuid::from(asset_id),
        };
        let asset_exists: Option<bool> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "RETURN record::exists(type::record('atelier_media_asset', $asset_id));",
                        exists_bindings,
                    )
                    .await
                })
            })
            .await?;
        if !asset_exists.unwrap_or(false) {
            return Err(AtelierError::NotFound(format!("media asset_id={asset_id}")));
        }
        let tag_id = Uuid::now_v7();
        let bindings = TagMediaAssetBindings {
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id)),
            tag_rid: RecordId::new("atelier_tag", SurrealUuid::from(tag_id)),
            tag_id: SurrealUuid::from(tag_id),
            text: normalized.clone(),
            source: source.to_owned(),
        };
        let row: Option<MediaAssetTagRow> = self
            .write_with_event(
                TAG_MEDIA_ASSET_STATEMENT,
                bindings,
                collections_event_family::MEDIA_ASSET_TAGGED,
                "atelier_media_asset_tag",
                &event_ref_for_text(&format!("media-asset-tag:{asset_id}:{normalized}")),
                serde_json::json!({
                    "asset_id": asset_id,
                    "text": normalized,
                    "tag_source_ref": event_ref_for_text(source),
                }),
            )
            .await?;
        row.map(MediaAssetTag::from).ok_or_else(|| {
            AtelierError::Internal("tagging a media asset returned no row".to_owned())
        })
    }

    /// Remove one tag from a media asset. Returns `true` only when a row was
    /// actually removed.
    pub async fn untag_media_asset(&self, asset_id: Uuid, text: &str) -> AtelierResult<bool> {
        let normalized = normalize_tag(text);
        if normalized.is_empty() {
            return Err(AtelierError::Validation(
                "tag text must not be empty".into(),
            ));
        }
        let bindings = UntagMediaAssetBindings {
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id)),
            text: normalized.clone(),
        };
        let removed: Option<bool> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(
                    async move { ctx.query_first(UNTAG_MEDIA_ASSET_STATEMENT, bindings).await },
                )
            })
            .await?;
        if !removed.unwrap_or(false) {
            return Ok(false);
        }
        self.record_event(
            collections_event_family::MEDIA_ASSET_UNTAGGED,
            "atelier_media_asset_tag",
            &event_ref_for_text(&format!("media-asset-untag:{}:{}", asset_id, normalized)),
            serde_json::json!({
                "asset_id": asset_id,
                "text": normalized,
            }),
        )
        .await?;
        Ok(true)
    }

    /// List direct tags on one media asset, ordered by normalized tag text.
    pub async fn list_media_asset_tags(&self, asset_id: Uuid) -> AtelierResult<Vec<MediaAssetTag>> {
        let bindings = AssetRefBinding {
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id)),
        };
        let rows: Vec<MediaAssetTagRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_MEDIA_ASSET_TAGS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(rows.into_iter().map(MediaAssetTag::from).collect())
    }

    /// Apply a collection's tags to every current member photo as a durable
    /// batch. Existing photo tags are additive and preserved; only tags listed
    /// in `remove_tags` are detached.
    pub async fn apply_collection_metadata_to_images(
        &self,
        request: &CollectionMetadataApplicationRequest,
    ) -> AtelierResult<CollectionMetadataApplication> {
        let requested_by = require_collection_ref_text("requested_by", &request.requested_by)?;
        let collection = self.get_collection(request.collection_id).await?;
        let applied_tags = normalize_media_tags(&collection.tags);
        let removed_tags = normalize_media_tags(&request.remove_tags);
        let application_id = Uuid::now_v7();
        let candidates = applied_tags
            .iter()
            .map(|text| {
                let tag_id = Uuid::now_v7();
                MetadataTagCandidate {
                    tag_rid: RecordId::new("atelier_tag", SurrealUuid::from(tag_id)),
                    tag_id: SurrealUuid::from(tag_id),
                    text: text.clone(),
                }
            })
            .collect();
        let bindings = ApplyCollectionMetadataBindings {
            application_rid: RecordId::new(
                "atelier_collection_metadata_application",
                SurrealUuid::from(application_id),
            ),
            application_id: SurrealUuid::from(application_id),
            collection_ref: RecordId::new(
                "atelier_collection",
                SurrealUuid::from(request.collection_id),
            ),
            requested_by: requested_by.to_owned(),
            applied_tags: candidates,
            applied_tags_json: applied_tags.clone(),
            removed_tags_json: removed_tags.clone(),
            tag_source: format!("collection:{}", request.collection_id),
        };
        let row: Option<CollectionMetadataApplicationRow> = self
            .write_with_event(
                APPLY_COLLECTION_METADATA_STATEMENT,
                bindings,
                collections_event_family::COLLECTION_METADATA_APPLIED,
                "atelier_collection",
                &request.collection_id.to_string(),
                serde_json::json!({
                    "application_id": application_id,
                    "requested_by": requested_by,
                    "applied_tags": applied_tags,
                    "removed_tags": removed_tags,
                }),
            )
            .await?;
        row.map(CollectionMetadataApplication::from).ok_or_else(|| {
            AtelierError::Internal(
                "applying collection metadata returned no receipt row".to_owned(),
            )
        })
    }

    /// Capture a contact sheet from an explicit list of media assets, or from a
    /// source collection's ordered membership when `asset_ids` is empty and a
    /// `collection` source is provided. The resulting manifest snapshots source
    /// asset ids + content hashes so the sheet is reproducible/auditable even if
    /// the source collection later changes.
    pub async fn create_contact_sheet(
        &self,
        name: &str,
        source_type: &str,
        source_collection_id: Option<Uuid>,
        asset_ids: &[Uuid],
        tags: &[String],
        character_internal_id: Option<Uuid>,
        sheet_version_id: Option<Uuid>,
    ) -> AtelierResult<ContactSheet> {
        let st = {
            let trimmed = source_type.trim().to_ascii_lowercase();
            if trimmed.is_empty() {
                "manual".to_string()
            } else {
                trimmed
            }
        };

        // Resolve membership: explicit ids win; otherwise pull from a source
        // collection's ordered membership (mirrors legacy source `createContactSheet`).
        let members: Vec<(Uuid, String)> = if !asset_ids.is_empty() {
            let bindings = AssetIdsBinding {
                asset_ids: asset_ids.iter().copied().map(SurrealUuid::from).collect(),
            };
            let rows: Vec<AssetHashRow> = self
                .store()
                .with_data_operation(move |ctx| {
                    Box::pin(
                        async move { ctx.query_values(ASSET_HASHES_STATEMENT, bindings).await },
                    )
                })
                .await?;
            let mut resolved = Vec::with_capacity(asset_ids.len());
            for asset_id in asset_ids {
                let hash = rows
                    .iter()
                    .find(|row| Uuid::from(row.asset_id) == *asset_id)
                    .map(|row| row.content_hash.clone())
                    .ok_or_else(|| AtelierError::NotFound(format!("media asset_id={asset_id}")))?;
                resolved.push((*asset_id, hash));
            }
            resolved
        } else if st == "collection" {
            let cid = source_collection_id.ok_or_else(|| {
                AtelierError::Validation(
                    "source_collection_id is required for a collection-sourced contact sheet"
                        .into(),
                )
            })?;
            let rows = self.list_collection_images(cid).await?;
            rows.into_iter()
                .map(|m| (m.asset_id, m.content_hash))
                .collect()
        } else {
            Vec::new()
        };

        if members.is_empty() {
            return Err(AtelierError::Validation(
                "contact sheet requires asset_ids or a non-empty source collection".into(),
            ));
        }

        let source_id = source_collection_id.map(|c| c.to_string());
        let tags_cleaned = clean_tags(tags);
        let image_count = i64::try_from(members.len()).map_err(|_| {
            AtelierError::Validation("contact sheet image count exceeds i64 range".into())
        })?;

        let manifest = serde_json::json!({
            "schema": CONTACT_SHEET_MANIFEST_SCHEMA,
            "source_type": st,
            "source_id": source_id,
            "items": members
                .iter()
                .map(|(asset_id, content_hash)| serde_json::json!({
                    "asset_id": asset_id,
                    "content_hash": content_hash,
                }))
                .collect::<Vec<_>>(),
            "tags": tags_cleaned,
            "captured_at": Utc::now().to_rfc3339(),
        });

        let final_name = if name.trim().is_empty() {
            format!("contact_sheet_{}", Utc::now().format("%Y%m%dT%H%M%SZ"))
        } else {
            name.trim().to_string()
        };

        let sheet_id = Uuid::now_v7();
        let bindings = CreateContactSheetBindings {
            sheet_rid: RecordId::new("atelier_contact_sheet", SurrealUuid::from(sheet_id)),
            sheet_id: SurrealUuid::from(sheet_id),
            name: final_name.clone(),
            source_type: st.clone(),
            source_id: source_id.clone(),
            tags_json: tags_cleaned,
            character_ref: character_internal_id
                .map(|id| RecordId::new("atelier_character", SurrealUuid::from(id))),
            sheet_version_ref: sheet_version_id
                .map(|id| RecordId::new("atelier_sheet_version", SurrealUuid::from(id))),
            manifest,
            image_count,
        };
        let row: Option<ContactSheetRow> = self
            .write_with_event(
                CREATE_CONTACT_SHEET_STATEMENT,
                bindings,
                collections_event_family::CONTACT_SHEET_CREATED,
                "atelier_contact_sheet",
                &sheet_id.to_string(),
                serde_json::json!({
                    "name": final_name,
                    "source_type": st,
                    "source_id": source_id,
                    "image_count": image_count,
                }),
            )
            .await?;
        row.map(ContactSheet::from).ok_or_else(|| {
            AtelierError::Internal("creating a contact sheet returned no row".to_owned())
        })
    }

    /// Fetch a contact sheet by id (manifest included).
    pub async fn get_contact_sheet(&self, sheet_id: Uuid) -> AtelierResult<ContactSheet> {
        let bindings = SheetIdBinding {
            sheet_id: SurrealUuid::from(sheet_id),
        };
        let row: Option<ContactSheetRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(
                    async move { ctx.query_first(GET_CONTACT_SHEET_STATEMENT, bindings).await },
                )
            })
            .await?;
        row.map(ContactSheet::from)
            .ok_or_else(|| AtelierError::NotFound(format!("contact_sheet sheet_id={sheet_id}")))
    }

    /// List contact sheets, optionally filtered by source type, most recent
    /// first.
    pub async fn list_contact_sheets(
        &self,
        source_type: Option<&str>,
    ) -> AtelierResult<Vec<ContactSheet>> {
        let filter = source_type.map(|s| s.trim().to_ascii_lowercase());
        let bindings = SourceTypeBinding {
            source_type: filter,
        };
        let rows: Vec<ContactSheetRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_CONTACT_SHEETS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(rows.into_iter().map(ContactSheet::from).collect())
    }

    /// Materialize a deterministic SVG artifact from a persisted contact-sheet
    /// manifest. Repeated renders for the same manifest return the existing row
    /// and do not emit duplicate events.
    pub async fn render_contact_sheet_svg_artifact(
        &self,
        sheet_id: Uuid,
    ) -> AtelierResult<ContactSheetSvgArtifact> {
        let sheet = self.get_contact_sheet(sheet_id).await?;
        let manifest_bytes = serde_json::to_vec(&sheet.manifest).map_err(|err| {
            AtelierError::Validation(format!("contact sheet manifest could not be hashed: {err}"))
        })?;
        let manifest_hash = sha256_ref(&manifest_bytes);
        let (svg_text, image_count) = render_contact_sheet_svg_text(&sheet)?;
        let content_hash = sha256_ref(svg_text.as_bytes());
        let artifact_ref = format!(
            "artifact://atelier/contact-sheet-svg/{}",
            content_hash.trim_start_matches("sha256:")
        );
        let existing_bindings = FindSvgArtifactBindings {
            sheet_ref: RecordId::new("atelier_contact_sheet", SurrealUuid::from(sheet_id)),
            manifest_hash: manifest_hash.clone(),
        };
        let existing: Option<ContactSheetSvgArtifactRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(FIND_SVG_ARTIFACT_STATEMENT, existing_bindings)
                        .await
                })
            })
            .await?;
        if let Some(row) = existing {
            return Ok(row.into());
        }

        let svg_artifact_id = deterministic_uuid(
            format!("atelier-contact-sheet-svg:{sheet_id}:{manifest_hash}").as_bytes(),
        );
        let bindings = CreateSvgArtifactBindings {
            artifact_rid: RecordId::new(
                "atelier_contact_sheet_svg_artifact",
                SurrealUuid::from(svg_artifact_id),
            ),
            svg_artifact_id: SurrealUuid::from(svg_artifact_id),
            sheet_ref: RecordId::new("atelier_contact_sheet", SurrealUuid::from(sheet_id)),
            manifest_hash: manifest_hash.clone(),
            content_hash: content_hash.clone(),
            artifact_ref: artifact_ref.clone(),
            svg_text,
            image_count,
        };
        let result: Option<SvgArtifactWriteResult> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(CREATE_SVG_ARTIFACT_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        let result = result.ok_or_else(|| {
            AtelierError::Internal("rendering contact sheet SVG returned no result".to_owned())
        })?;
        let artifact: ContactSheetSvgArtifact = result
            .artifact
            .into_iter()
            .next()
            .ok_or_else(|| {
                AtelierError::NotFound(format!(
                    "contact sheet SVG artifact sheet_id={sheet_id} manifest_hash={manifest_hash}"
                ))
            })?
            .into();
        let created = result.created;

        if created {
            self.record_event(
                collections_event_family::CONTACT_SHEET_SVG_RENDERED,
                "atelier_contact_sheet",
                &sheet_id.to_string(),
                serde_json::json!({
                    "svg_artifact_id": artifact.svg_artifact_id,
                    "manifest_hash": artifact.manifest_hash,
                    "content_hash": artifact.content_hash,
                    "artifact_ref": artifact.artifact_ref,
                    "image_count": artifact.image_count,
                }),
            )
            .await?;
        }

        Ok(artifact)
    }
}

#[cfg(test)]
mod embedded_store_tests {
    use super::*;
    use crate::storage::surreal::{bootstrap_schema, SurrealStorage, SurrealStorageConfig};

    #[derive(SurrealValue)]
    struct SeedAssetBindings {
        asset_rid: RecordId,
        asset_id: SurrealUuid,
        content_hash: String,
        artifact_ref: String,
    }

    async fn seed_media_asset(store: &SurrealStorage, asset_id: Uuid, content_hash: &str) {
        let bindings = SeedAssetBindings {
            asset_rid: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id)),
            asset_id: SurrealUuid::from(asset_id),
            content_hash: content_hash.to_owned(),
            artifact_ref: format!("artifact://atelier/test/{asset_id}"),
        };
        let created: Option<bool> = store
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "RETURN { \
                           CREATE $asset_rid CONTENT { asset_id: $asset_id, \
                             content_hash: $content_hash, mime: 'image/png', byte_len: 1, \
                             artifact_ref: $artifact_ref }; \
                           RETURN true; };",
                        bindings,
                    )
                    .await
                })
            })
            .await
            .expect("seed media asset");
        assert_eq!(created, Some(true));
    }

    #[tokio::test]
    async fn collection_batch_rolls_back_and_reopens_idempotently() {
        let temp = tempfile::tempdir().expect("create temporary collection store");
        let config = SurrealStorageConfig::for_data_dir(temp.path()).expect("configure store");
        let storage = SurrealStorage::open(config).await.expect("open store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap schema first pass");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap schema idempotent second pass");
        let atelier = AtelierStore::new(storage.clone());
        atelier.ensure_schema().await.expect("atelier schema ready");

        let collection = atelier
            .create_collection(&NewCollection {
                name: "collection-reopen-proof".to_owned(),
                notes: "embedded rollback and reopen".to_owned(),
                tags: vec!["hero".to_owned(), " hero ".to_owned()],
                character_internal_id: None,
                sheet_version_id: None,
            })
            .await
            .expect("create collection");
        let retained_asset = Uuid::now_v7();
        let rollback_asset = Uuid::now_v7();
        seed_media_asset(&storage, retained_asset, "sha256:retained").await;
        seed_media_asset(&storage, rollback_asset, "sha256:rollback").await;

        assert_eq!(
            atelier
                .add_images_to_collection(collection.collection_id, &[retained_asset])
                .await
                .expect("add retained asset"),
            1
        );
        assert_eq!(
            atelier
                .add_images_to_collection(collection.collection_id, &[retained_asset])
                .await
                .expect("idempotent membership replay"),
            0
        );
        let events_before_failure = atelier
            .count_events_for_aggregate(
                collections_event_family::COLLECTION_IMAGES_ADDED,
                "atelier_collection",
                &collection.collection_id.to_string(),
            )
            .await
            .expect("count events before rollback proof");

        let missing_asset = Uuid::now_v7();
        assert!(atelier
            .add_images_to_collection(collection.collection_id, &[rollback_asset, missing_asset],)
            .await
            .is_err());
        let members = atelier
            .list_collection_images(collection.collection_id)
            .await
            .expect("list after rejected batch");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].asset_id, retained_asset);
        assert_eq!(
            atelier
                .count_events_for_aggregate(
                    collections_event_family::COLLECTION_IMAGES_ADDED,
                    "atelier_collection",
                    &collection.collection_id.to_string(),
                )
                .await
                .expect("count events after rollback proof"),
            events_before_failure,
            "the failed membership batch and its event must roll back together"
        );

        storage.shutdown().await.expect("close first store");
        let reopened = SurrealStorage::open(
            SurrealStorageConfig::for_data_dir(temp.path()).expect("configure reopened store"),
        )
        .await
        .expect("reopen store");
        bootstrap_schema(&reopened)
            .await
            .expect("bootstrap reopened store idempotently");
        let reopened_atelier = AtelierStore::new(reopened.clone());
        reopened_atelier
            .ensure_schema()
            .await
            .expect("atelier schema remains ready after reopen");
        reopened_atelier
            .ensure_schema()
            .await
            .expect("atelier readiness gate is idempotent after reopen");

        let defined_atelier_tables: std::collections::BTreeSet<String> = reopened
            .with_data_operation(|ctx| {
                Box::pin(async move {
                    ctx.query_values::<String, ()>(
                        "RETURN array::sort(object::keys((INFO FOR DB).tables));",
                        (),
                    )
                    .await
                })
            })
            .await
            .expect("inspect reopened schema")
            .into_iter()
            .filter(|table| table.starts_with("atelier_"))
            .collect();
        let readiness_inventory: std::collections::BTreeSet<String> =
            crate::atelier::ATELIER_TABLES
                .iter()
                .map(|table| (*table).to_owned())
                .collect();
        assert_eq!(
            readiness_inventory, defined_atelier_tables,
            "Atelier readiness must cover every canonical atelier_* table and no stale table"
        );

        let reopened_members = reopened_atelier
            .list_collection_images(collection.collection_id)
            .await
            .expect("list persisted members after reopen");
        assert_eq!(reopened_members.len(), 1);
        assert_eq!(reopened_members[0].asset_id, retained_asset);

        let first_tag = reopened_atelier
            .tag_media_asset(retained_asset, " Hero ", "test://collections-reopen")
            .await
            .expect("tag asset");
        let replayed_tag = reopened_atelier
            .tag_media_asset(retained_asset, "hero", "test://collections-reopen")
            .await
            .expect("idempotent tag replay");
        assert_eq!(first_tag.tag_id, replayed_tag.tag_id);
        assert_eq!(
            reopened_atelier
                .list_media_asset_tags(retained_asset)
                .await
                .expect("list persisted asset tags")
                .len(),
            1
        );
        reopened.shutdown().await.expect("close reopened store");
    }
}

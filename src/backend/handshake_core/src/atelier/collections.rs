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
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{
    atelier_event_sql, event_ref_for_text, reject_legacy_runtime_ref, search::normalize_tag,
    AtelierError, AtelierResult, AtelierStore,
};

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
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
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
    pub added_at_utc: DateTime<Utc>,
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
    ];
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

#[derive(SurrealValue)]
struct CollectionRow {
    collection_id: SurrealUuid,
    name: String,
    notes: String,
    tags_json: Vec<String>,
    character_internal_id: Option<SurrealUuid>,
    sheet_version_id: Option<SurrealUuid>,
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
    added_at_utc: Datetime,
}

impl From<CollectionMemberRow> for CollectionMember {
    fn from(row: CollectionMemberRow) -> Self {
        Self {
            collection_id: row.collection_id.into(),
            asset_id: row.asset_id.into(),
            content_hash: row.content_hash,
            sort_order: row.sort_order,
            added_at_utc: row.added_at_utc.into(),
        }
    }
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
         created_at_utc, updated_at_utc"
    };
}

macro_rules! collection_member_select {
    () => {
        "record::id(collection_id) AS collection_id, record::id(asset_id) AS asset_id, \
         asset_id.content_hash AS content_hash, sort_order, added_at_utc"
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
}

#[derive(Clone, SurrealValue)]
struct RemoveImagesBindings {
    collection_ref: RecordId,
    asset_refs: Vec<RecordId>,
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
         sheet_version_id: $domain.sheet_version_ref \
       }; RETURN (SELECT ",
    collection_select!(),
    " FROM $rid); };"
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
         tags_json = $domain.tags_json ?? tags_json, updated_at_utc = time::now(); \
       RETURN (SELECT ",
    collection_select!(),
    " FROM $rid); };"
);

const ADD_IMAGES_STATEMENT: &str = concat!(
    "RETURN { \
       LET $existing = count(SELECT id FROM atelier_collection_item \
                            WHERE collection_id = $domain.collection_ref \
                              AND asset_id IN $domain.asset_refs); \
       LET $next_order = (array::max((SELECT VALUE sort_order FROM atelier_collection_item \
                                     WHERE collection_id = $domain.collection_ref)) ?? -1) + 1; ",
    atelier_event_sql!(),
    " FOR $item IN $domain.items { \
         LET $rid = type::record('atelier_collection_item', $item.pair_key); \
         IF !record::exists($rid) { \
           CREATE $rid CONTENT { collection_id: $domain.collection_ref, \
             asset_id: $item.asset_ref, sort_order: $next_order + $item.order_offset }; \
         }; \
       }; \
       LET $inserted = array::len($domain.items) - $existing; \
       IF $inserted > 0 { UPDATE $domain.collection_ref SET updated_at_utc = time::now(); }; \
       RETURN $inserted; };"
);

const REMOVE_IMAGES_STATEMENT: &str = concat!(
    "RETURN { \
       LET $removed = count(SELECT id FROM atelier_collection_item \
                            WHERE collection_id = $domain.collection_ref \
                              AND asset_id IN $domain.asset_refs); ",
    atelier_event_sql!(),
    " DELETE atelier_collection_item WHERE collection_id = $domain.collection_ref \
           AND asset_id IN $domain.asset_refs; \
       IF $removed > 0 { UPDATE $domain.collection_ref SET updated_at_utc = time::now(); }; \
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
        let name = new.name.trim();
        if name.is_empty() {
            return Err(AtelierError::Validation(
                "collection name must not be empty".into(),
            ));
        }
        let tags = clean_tags(&new.tags);
        let collection_id = Uuid::now_v7();
        let bindings = CreateCollectionBindings {
            collection_rid: RecordId::new("atelier_collection", SurrealUuid::from(collection_id)),
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
                }),
            )
            .await?;
        row.map(Collection::from).ok_or_else(|| {
            AtelierError::Internal("creating a collection returned no row".to_owned())
        })
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
            collection_rid: RecordId::new("atelier_collection", SurrealUuid::from(collection_id)),
            name: name.map(|value| value.trim().to_owned()),
            notes: notes.map(ToOwned::to_owned),
            tags_json: tags_cleaned.clone(),
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
        // Validate the collection exists (clear error vs. an FK violation).
        self.get_collection(collection_id).await?;
        let mut unique = Vec::new();
        for asset_id in asset_ids {
            if !unique.contains(asset_id) {
                unique.push(*asset_id);
            }
        }
        let collection_ref = RecordId::new("atelier_collection", SurrealUuid::from(collection_id));
        let asset_refs: Vec<RecordId> = unique
            .iter()
            .map(|id| RecordId::new("atelier_media_asset", SurrealUuid::from(*id)))
            .collect();
        let items = unique
            .iter()
            .zip(asset_refs.iter())
            .enumerate()
            .map(|(offset, (asset_id, asset_ref))| CollectionItemInput {
                asset_ref: asset_ref.clone(),
                pair_key: vec![
                    SurrealUuid::from(collection_id),
                    SurrealUuid::from(*asset_id),
                ],
                order_offset: offset as i64,
            })
            .collect();
        let bindings = AddImagesBindings {
            collection_ref,
            asset_refs,
            items,
        };
        let inserted: Option<i64> = self
            .write_with_event(
                ADD_IMAGES_STATEMENT,
                bindings,
                collections_event_family::COLLECTION_IMAGES_ADDED,
                "atelier_collection",
                &collection_id.to_string(),
                serde_json::json!({
                    "requested": asset_ids.len(),
                    "unique_requested": unique.len(),
                }),
            )
            .await?;
        inserted.ok_or_else(|| {
            AtelierError::Internal("adding collection images returned no count".to_owned())
        })
    }

    /// Remove media assets from a collection. Returns the number removed. Bumps
    /// `updated_at_utc` when anything was removed.
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
        let bindings = RemoveImagesBindings {
            collection_ref: RecordId::new("atelier_collection", SurrealUuid::from(collection_id)),
            asset_refs: unique
                .iter()
                .map(|id| RecordId::new("atelier_media_asset", SurrealUuid::from(*id)))
                .collect(),
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
                }),
            )
            .await?;
        removed.ok_or_else(|| {
            AtelierError::Internal("removing collection images returned no count".to_owned())
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

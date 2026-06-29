//! Collections + contact sheets (MT-018): named, ordered image sets with
//! notes/tags and optional character/sheet links, plus a contact-sheet manifest
//! that snapshots the membership (source asset ids + content hashes) at capture
//! time so a sheet stays reproducible even as collections evolve.
//!
//! legacy source source: `app/backend/library.js` (`createCollection`, `updateCollection`,
//! `addImagesToCollection`, `removeImagesFromCollection`, `listCollectionImages`,
//! `createContactSheet`, `listContactSheets`) and `app/backend/db.js`
//! (`Collection`, `CollectionItem`, `ContactSheet` tables). Schema/behavior
//! intent only -- storage is PostgreSQL via sqlx, never the legacy source SQLite layer.
//! MT ids: MT-003 (module boundary), MT-005 (event coverage), MT-018 (this fold-in).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{
    event_ref_for_text, reject_legacy_runtime_ref, search::normalize_tag, AtelierError,
    AtelierResult, AtelierStore,
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

fn optional_collection_item_ref(field: &str, value: Option<&str>) -> AtelierResult<Option<String>> {
    match value {
        Some(raw) => Ok(Some(require_collection_ref_text(field, raw)?.to_owned())),
        None => Ok(None),
    }
}

fn collection_from_row(row: &sqlx::postgres::PgRow) -> Collection {
    let tags_json: serde_json::Value = row.get("tags_json");
    let tags = tags_json
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Collection {
        collection_id: row.get("collection_id"),
        name: row.get("name"),
        notes: row.get("notes"),
        tags,
        character_internal_id: row.get("character_internal_id"),
        sheet_version_id: row.get("sheet_version_id"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn media_asset_tag_from_row(row: &sqlx::postgres::PgRow) -> MediaAssetTag {
    MediaAssetTag {
        asset_id: row.get("asset_id"),
        tag_id: row.get("tag_id"),
        text: row.get("text"),
        source: row.get("source"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn collection_metadata_application_from_row(
    row: &sqlx::postgres::PgRow,
) -> CollectionMetadataApplication {
    let applied_tags_json: serde_json::Value = row.get("applied_tags_json");
    let removed_tags_json: serde_json::Value = row.get("removed_tags_json");
    let applied_tags = applied_tags_json
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let removed_tags = removed_tags_json
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    CollectionMetadataApplication {
        application_id: row.get("application_id"),
        collection_id: row.get("collection_id"),
        requested_by: row.get("requested_by"),
        applied_tags,
        removed_tags,
        affected_asset_count: row.get("affected_asset_count"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn member_from_row(row: &sqlx::postgres::PgRow) -> CollectionMember {
    CollectionMember {
        collection_id: row.get("collection_id"),
        asset_id: row.get("asset_id"),
        content_hash: row.get("content_hash"),
        sort_order: row.get("sort_order"),
        added_at_utc: row.get("added_at_utc"),
    }
}

async fn ensure_tag_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    text: &str,
) -> AtelierResult<(Uuid, String)> {
    let normalized = normalize_tag(text);
    if normalized.is_empty() {
        return Err(AtelierError::Validation(
            "tag text must not be empty".into(),
        ));
    }
    let row = sqlx::query(
        r#"INSERT INTO atelier_tag (text)
           VALUES ($1)
           ON CONFLICT (text) DO UPDATE SET text = EXCLUDED.text
           RETURNING tag_id, text"#,
    )
    .bind(&normalized)
    .fetch_one(&mut **tx)
    .await?;
    Ok((row.get("tag_id"), row.get("text")))
}

fn contact_sheet_from_row(row: &sqlx::postgres::PgRow) -> ContactSheet {
    let tags_json: serde_json::Value = row.get("tags_json");
    let tags = tags_json
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    ContactSheet {
        sheet_id: row.get("sheet_id"),
        name: row.get("name"),
        source_type: row.get("source_type"),
        source_id: row.get("source_id"),
        tags,
        character_internal_id: row.get("character_internal_id"),
        sheet_version_id: row.get("sheet_version_id"),
        manifest: row.get("manifest"),
        image_count: row.get("image_count"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn contact_sheet_svg_artifact_from_row(row: &sqlx::postgres::PgRow) -> ContactSheetSvgArtifact {
    ContactSheetSvgArtifact {
        svg_artifact_id: row.get("svg_artifact_id"),
        sheet_id: row.get("sheet_id"),
        manifest_hash: row.get("manifest_hash"),
        content_hash: row.get("content_hash"),
        artifact_ref: row.get("artifact_ref"),
        svg_text: row.get("svg_text"),
        image_count: row.get("image_count"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn sha256_ref(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
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

impl AtelierStore {
    /// Create a named collection. `name` must be non-empty and is unique. Tags
    /// are trimmed and de-duplicated. Optional character/sheet links are FK
    /// validated by the database.
    pub async fn create_collection(&self, new: &NewCollection) -> AtelierResult<Collection> {
        self.create_collection_inner(new, None).await
    }

    /// Create a named collection and include the requesting actor in the
    /// durable EventLedger payload.
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
        let tags = clean_tags(&new.tags);
        let tags_json = serde_json::Value::from(tags.clone());
        let row = sqlx::query(
            r#"INSERT INTO atelier_collection
                 (name, notes, tags_json, character_internal_id, sheet_version_id)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING collection_id, name, notes, tags_json,
                         character_internal_id, sheet_version_id,
                         created_at_utc, updated_at_utc"#,
        )
        .bind(name)
        .bind(&new.notes)
        .bind(&tags_json)
        .bind(new.character_internal_id)
        .bind(new.sheet_version_id)
        .fetch_one(self.pool())
        .await?;
        let collection = collection_from_row(&row);
        self.record_event(
            collections_event_family::COLLECTION_CREATED,
            "atelier_collection",
            &collection.collection_id.to_string(),
            serde_json::json!({
                "name": collection.name,
                "tags": collection.tags,
                "character_scoped": collection.character_internal_id.is_some(),
                "requested_by": requested_by,
            }),
        )
        .await?;
        Ok(collection)
    }

    /// Fetch a collection by id.
    pub async fn get_collection(&self, collection_id: Uuid) -> AtelierResult<Collection> {
        let row = sqlx::query(
            r#"SELECT collection_id, name, notes, tags_json,
                      character_internal_id, sheet_version_id,
                      created_at_utc, updated_at_utc
               FROM atelier_collection WHERE collection_id = $1"#,
        )
        .bind(collection_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("collection_id={collection_id}")))?;
        Ok(collection_from_row(&row))
    }

    /// List collections (most recently updated first).
    pub async fn list_collections(&self) -> AtelierResult<Vec<Collection>> {
        let rows = sqlx::query(
            r#"SELECT collection_id, name, notes, tags_json,
                      character_internal_id, sheet_version_id,
                      created_at_utc, updated_at_utc
               FROM atelier_collection ORDER BY updated_at_utc DESC"#,
        )
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(collection_from_row).collect())
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
        let tags_cleaned = tags.map(clean_tags);
        let tags_json = tags_cleaned
            .as_ref()
            .map(|t| serde_json::Value::from(t.clone()));
        let row = sqlx::query(
            r#"UPDATE atelier_collection
               SET name           = COALESCE($2, name),
                   notes          = COALESCE($3, notes),
                   tags_json      = COALESCE($4, tags_json),
                   updated_at_utc = NOW()
               WHERE collection_id = $1
               RETURNING collection_id, name, notes, tags_json,
                         character_internal_id, sheet_version_id,
                         created_at_utc, updated_at_utc"#,
        )
        .bind(collection_id)
        .bind(name.map(|s| s.trim()))
        .bind(notes)
        .bind(&tags_json)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("collection_id={collection_id}")))?;
        let collection = collection_from_row(&row);
        self.record_event(
            collections_event_family::COLLECTION_UPDATED,
            "atelier_collection",
            &collection.collection_id.to_string(),
            serde_json::json!({
                "name": collection.name,
                "tags": collection.tags,
            }),
        )
        .await?;
        Ok(collection)
    }

    /// Append media assets to a collection in the given order. Existing
    /// memberships are ignored (idempotent via ON CONFLICT). Returns the number
    /// of newly inserted memberships. Bumps the collection `updated_at_utc`.
    pub async fn add_images_to_collection(
        &self,
        collection_id: Uuid,
        asset_ids: &[Uuid],
    ) -> AtelierResult<i64> {
        self.add_images_to_collection_inner(collection_id, asset_ids, None, None, None)
            .await
    }

    /// Append media assets and include the requesting actor in the durable
    /// EventLedger payload.
    pub async fn add_images_to_collection_attributed(
        &self,
        collection_id: Uuid,
        asset_ids: &[Uuid],
        requested_by: &str,
    ) -> AtelierResult<i64> {
        self.add_images_to_collection_inner(
            collection_id,
            asset_ids,
            Some(requested_by),
            None,
            None,
        )
        .await
    }

    /// Append media assets and persist optional link-scoped provenance on the
    /// collection membership, not on the asset identity.
    pub async fn add_images_to_collection_with_link_refs_attributed(
        &self,
        collection_id: Uuid,
        asset_ids: &[Uuid],
        source_path_ref: Option<&str>,
        source_url_ref: Option<&str>,
        requested_by: &str,
    ) -> AtelierResult<i64> {
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
    ) -> AtelierResult<i64> {
        let source_path_ref = optional_collection_item_ref("source_path_ref", source_path_ref)?;
        let source_url_ref = optional_collection_item_ref("source_url_ref", source_url_ref)?;
        let mut tx = self.pool().begin().await?;
        let exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT collection_id FROM atelier_collection WHERE collection_id = $1 FOR UPDATE",
        )
        .bind(collection_id)
        .fetch_optional(&mut *tx)
        .await?;
        if exists.is_none() {
            tx.rollback().await?;
            return Err(AtelierError::NotFound(format!(
                "collection_id={collection_id}"
            )));
        }

        let mut unique_asset_ids = Vec::new();
        for asset_id in asset_ids {
            if !unique_asset_ids.iter().any(|existing| existing == asset_id) {
                unique_asset_ids.push(*asset_id);
            }
        }
        if !unique_asset_ids.is_empty() {
            let existing: Vec<Uuid> = sqlx::query_scalar(
                "SELECT asset_id FROM atelier_media_asset WHERE asset_id = ANY($1)",
            )
            .bind(&unique_asset_ids)
            .fetch_all(&mut *tx)
            .await?;
            if existing.len() != unique_asset_ids.len() {
                let existing: std::collections::HashSet<Uuid> = existing.into_iter().collect();
                let missing: Vec<String> = unique_asset_ids
                    .iter()
                    .filter(|asset_id| !existing.contains(asset_id))
                    .map(Uuid::to_string)
                    .collect();
                tx.rollback().await?;
                return Err(AtelierError::NotFound(format!(
                    "collection media targets missing from atelier_media_asset: {}",
                    missing.join(", ")
                )));
            }
        }

        let mut next_order: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM atelier_collection_item WHERE collection_id = $1",
        )
        .bind(collection_id)
        .fetch_one(&mut *tx)
        .await?;

        let mut inserted: i64 = 0;
        let mut updated_refs: i64 = 0;
        for asset_id in asset_ids {
            let affected = sqlx::query(
                r#"INSERT INTO atelier_collection_item
                     (collection_id, asset_id, sort_order, source_path_ref, source_url_ref)
                   VALUES ($1, $2, $3, $4, $5)
                   ON CONFLICT (collection_id, asset_id) DO NOTHING"#,
            )
            .bind(collection_id)
            .bind(asset_id)
            .bind(next_order)
            .bind(source_path_ref.as_deref())
            .bind(source_url_ref.as_deref())
            .execute(&mut *tx)
            .await?
            .rows_affected();
            if affected > 0 {
                inserted += 1;
                next_order += 1;
            } else if source_path_ref.is_some() || source_url_ref.is_some() {
                let changed = sqlx::query(
                    r#"UPDATE atelier_collection_item
                       SET source_path_ref = COALESCE($3, source_path_ref),
                           source_url_ref = COALESCE($4, source_url_ref)
                       WHERE collection_id = $1 AND asset_id = $2
                         AND (
                           ($3::text IS NOT NULL AND source_path_ref IS DISTINCT FROM $3::text)
                           OR ($4::text IS NOT NULL AND source_url_ref IS DISTINCT FROM $4::text)
                         )"#,
                )
                .bind(collection_id)
                .bind(asset_id)
                .bind(source_path_ref.as_deref())
                .bind(source_url_ref.as_deref())
                .execute(&mut *tx)
                .await?
                .rows_affected();
                updated_refs += changed as i64;
            }
        }

        if inserted > 0 || updated_refs > 0 {
            sqlx::query(
                "UPDATE atelier_collection SET updated_at_utc = NOW() WHERE collection_id = $1",
            )
            .bind(collection_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        self.record_event(
            collections_event_family::COLLECTION_IMAGES_ADDED,
            "atelier_collection",
            &collection_id.to_string(),
            serde_json::json!({
                "requested": asset_ids.len(),
                "inserted": inserted,
                "updated_refs": updated_refs,
                "requested_by": requested_by,
            }),
        )
        .await?;
        Ok(inserted)
    }

    /// Remove media assets from a collection. Returns the number removed. Bumps
    /// `updated_at_utc` when anything was removed.
    pub async fn remove_images_from_collection(
        &self,
        collection_id: Uuid,
        asset_ids: &[Uuid],
    ) -> AtelierResult<i64> {
        let mut tx = self.pool().begin().await?;
        let mut removed: i64 = 0;
        for asset_id in asset_ids {
            let affected = sqlx::query(
                "DELETE FROM atelier_collection_item WHERE collection_id = $1 AND asset_id = $2",
            )
            .bind(collection_id)
            .bind(asset_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            removed += affected as i64;
        }
        if removed > 0 {
            sqlx::query(
                "UPDATE atelier_collection SET updated_at_utc = NOW() WHERE collection_id = $1",
            )
            .bind(collection_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        self.record_event(
            collections_event_family::COLLECTION_IMAGES_REMOVED,
            "atelier_collection",
            &collection_id.to_string(),
            serde_json::json!({
                "requested": asset_ids.len(),
                "removed": removed,
            }),
        )
        .await?;
        Ok(removed)
    }

    /// List a collection's members in membership order, resolved to their media
    /// asset content hashes.
    pub async fn list_collection_images(
        &self,
        collection_id: Uuid,
    ) -> AtelierResult<Vec<CollectionMember>> {
        let rows = sqlx::query(
            r#"SELECT ci.collection_id, ci.asset_id, ma.content_hash,
                      ci.sort_order, ci.added_at_utc
               FROM atelier_collection_item ci
               JOIN atelier_media_asset ma ON ma.asset_id = ci.asset_id
               WHERE ci.collection_id = $1
               ORDER BY ci.sort_order ASC, ci.added_at_utc ASC"#,
        )
        .bind(collection_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(member_from_row).collect())
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
        let tag = self.ensure_tag(text).await?;
        let asset_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT asset_id FROM atelier_media_asset WHERE asset_id = $1")
                .bind(asset_id)
                .fetch_optional(self.pool())
                .await?;
        if asset_exists.is_none() {
            return Err(AtelierError::NotFound(format!("media asset_id={asset_id}")));
        }
        let row = sqlx::query(
            r#"WITH upserted AS (
                 INSERT INTO atelier_media_asset_tag (asset_id, tag_id, source)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (asset_id, tag_id)
                   DO UPDATE SET source = EXCLUDED.source
                 RETURNING asset_id, tag_id, source, created_at_utc
               )
               SELECT u.asset_id, u.tag_id, t.text, u.source, u.created_at_utc
               FROM upserted u
               JOIN atelier_tag t ON t.tag_id = u.tag_id"#,
        )
        .bind(asset_id)
        .bind(tag.tag_id)
        .bind(source)
        .fetch_one(self.pool())
        .await?;
        let asset_tag = media_asset_tag_from_row(&row);
        self.record_event(
            collections_event_family::MEDIA_ASSET_TAGGED,
            "atelier_media_asset_tag",
            &event_ref_for_text(&format!("media-asset-tag:{}:{}", asset_id, tag.tag_id)),
            serde_json::json!({
                "asset_id": asset_id,
                "tag_id": tag.tag_id,
                "text": asset_tag.text,
                "tag_source_ref": event_ref_for_text(source),
            }),
        )
        .await?;
        Ok(asset_tag)
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
        let removed = sqlx::query(
            r#"DELETE FROM atelier_media_asset_tag mat
               USING atelier_tag t
               WHERE mat.tag_id = t.tag_id
                 AND mat.asset_id = $1
                 AND t.text = $2"#,
        )
        .bind(asset_id)
        .bind(&normalized)
        .execute(self.pool())
        .await?;
        if removed.rows_affected() == 0 {
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
        let rows = sqlx::query(
            r#"SELECT mat.asset_id, mat.tag_id, t.text, mat.source,
                      mat.created_at_utc
               FROM atelier_media_asset_tag mat
               JOIN atelier_tag t ON t.tag_id = mat.tag_id
               WHERE mat.asset_id = $1
               ORDER BY t.text ASC"#,
        )
        .bind(asset_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(media_asset_tag_from_row).collect())
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
        let applied_tags_json = serde_json::Value::from(applied_tags.clone());
        let removed_tags_json = serde_json::Value::from(removed_tags.clone());
        let mut tx = self.pool().begin().await?;

        let member_rows = sqlx::query(
            r#"SELECT asset_id
               FROM atelier_collection_item
               WHERE collection_id = $1
               ORDER BY sort_order ASC, added_at_utc ASC"#,
        )
        .bind(request.collection_id)
        .fetch_all(&mut *tx)
        .await?;
        let member_asset_ids: Vec<Uuid> =
            member_rows.iter().map(|row| row.get("asset_id")).collect();
        let tag_source = format!("collection:{}", request.collection_id);

        let mut applied_tag_ids = Vec::new();
        for tag_text in &applied_tags {
            let (tag_id, _) = ensure_tag_in_tx(&mut tx, tag_text).await?;
            applied_tag_ids.push(tag_id);
        }

        for asset_id in &member_asset_ids {
            for tag_id in &applied_tag_ids {
                sqlx::query(
                    r#"INSERT INTO atelier_media_asset_tag (asset_id, tag_id, source)
                       VALUES ($1, $2, $3)
                       ON CONFLICT (asset_id, tag_id) DO NOTHING"#,
                )
                .bind(asset_id)
                .bind(tag_id)
                .bind(&tag_source)
                .execute(&mut *tx)
                .await?;
            }

            for tag_text in &removed_tags {
                sqlx::query(
                    r#"DELETE FROM atelier_media_asset_tag mat
                       USING atelier_tag t
                       WHERE mat.tag_id = t.tag_id
                         AND mat.asset_id = $1
                         AND t.text = $2"#,
                )
                .bind(asset_id)
                .bind(tag_text)
                .execute(&mut *tx)
                .await?;
            }
        }

        let affected_asset_count = i64::try_from(member_asset_ids.len()).map_err(|_| {
            AtelierError::Validation("collection member count exceeds i64 range".into())
        })?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_collection_metadata_application
                 (collection_id, requested_by, applied_tags_json,
                  removed_tags_json, affected_asset_count)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING application_id, collection_id, requested_by,
                         applied_tags_json, removed_tags_json,
                         affected_asset_count, created_at_utc"#,
        )
        .bind(request.collection_id)
        .bind(requested_by)
        .bind(&applied_tags_json)
        .bind(&removed_tags_json)
        .bind(affected_asset_count)
        .fetch_one(&mut *tx)
        .await?;
        let application = collection_metadata_application_from_row(&row);
        tx.commit().await?;

        self.record_event(
            collections_event_family::COLLECTION_METADATA_APPLIED,
            "atelier_collection",
            &request.collection_id.to_string(),
            serde_json::json!({
                "application_id": application.application_id,
                "requested_by": application.requested_by,
                "affected_asset_count": application.affected_asset_count,
                "applied_tags": &application.applied_tags,
                "removed_tags": &application.removed_tags,
            }),
        )
        .await?;
        Ok(application)
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
            let mut resolved = Vec::with_capacity(asset_ids.len());
            for asset_id in asset_ids {
                let hash: Option<String> = sqlx::query_scalar(
                    "SELECT content_hash FROM atelier_media_asset WHERE asset_id = $1",
                )
                .bind(asset_id)
                .fetch_optional(self.pool())
                .await?;
                let hash = hash
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
        let tags_json = serde_json::Value::from(tags_cleaned.clone());
        let image_count = members.len() as i64;

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

        let row = sqlx::query(
            r#"INSERT INTO atelier_contact_sheet
                 (name, source_type, source_id, tags_json, character_internal_id,
                  sheet_version_id, manifest, image_count)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING sheet_id, name, source_type, source_id, tags_json,
                         character_internal_id, sheet_version_id, manifest,
                         image_count, created_at_utc"#,
        )
        .bind(&final_name)
        .bind(&st)
        .bind(&source_id)
        .bind(&tags_json)
        .bind(character_internal_id)
        .bind(sheet_version_id)
        .bind(&manifest)
        .bind(image_count)
        .fetch_one(self.pool())
        .await?;
        let sheet = contact_sheet_from_row(&row);
        self.record_event(
            collections_event_family::CONTACT_SHEET_CREATED,
            "atelier_contact_sheet",
            &sheet.sheet_id.to_string(),
            serde_json::json!({
                "name": sheet.name,
                "source_type": sheet.source_type,
                "source_id": sheet.source_id,
                "image_count": sheet.image_count,
            }),
        )
        .await?;
        Ok(sheet)
    }

    /// Fetch a contact sheet by id (manifest included).
    pub async fn get_contact_sheet(&self, sheet_id: Uuid) -> AtelierResult<ContactSheet> {
        let row = sqlx::query(
            r#"SELECT sheet_id, name, source_type, source_id, tags_json,
                      character_internal_id, sheet_version_id, manifest,
                      image_count, created_at_utc
               FROM atelier_contact_sheet WHERE sheet_id = $1"#,
        )
        .bind(sheet_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("contact_sheet sheet_id={sheet_id}")))?;
        Ok(contact_sheet_from_row(&row))
    }

    /// List contact sheets, optionally filtered by source type, most recent
    /// first.
    pub async fn list_contact_sheets(
        &self,
        source_type: Option<&str>,
    ) -> AtelierResult<Vec<ContactSheet>> {
        let filter = source_type.map(|s| s.trim().to_ascii_lowercase());
        let rows = sqlx::query(
            r#"SELECT sheet_id, name, source_type, source_id, tags_json,
                      character_internal_id, sheet_version_id, manifest,
                      image_count, created_at_utc
               FROM atelier_contact_sheet
               WHERE $1::text IS NULL OR source_type = $1
               ORDER BY created_at_utc DESC"#,
        )
        .bind(filter)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(contact_sheet_from_row).collect())
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

        let inserted = sqlx::query(
            r#"INSERT INTO atelier_contact_sheet_svg_artifact
                 (sheet_id, manifest_hash, content_hash, artifact_ref, svg_text, image_count)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (sheet_id, manifest_hash) DO NOTHING
               RETURNING svg_artifact_id, sheet_id, manifest_hash, content_hash, artifact_ref,
                         svg_text, image_count, created_at_utc"#,
        )
        .bind(sheet_id)
        .bind(&manifest_hash)
        .bind(&content_hash)
        .bind(&artifact_ref)
        .bind(&svg_text)
        .bind(image_count)
        .fetch_optional(self.pool())
        .await?;

        let (artifact, created) = if let Some(row) = inserted {
            (contact_sheet_svg_artifact_from_row(&row), true)
        } else {
            let row = sqlx::query(
                r#"SELECT svg_artifact_id, sheet_id, manifest_hash, content_hash, artifact_ref,
                          svg_text, image_count, created_at_utc
                   FROM atelier_contact_sheet_svg_artifact
                   WHERE sheet_id = $1 AND manifest_hash = $2"#,
            )
            .bind(sheet_id)
            .bind(&manifest_hash)
            .fetch_optional(self.pool())
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!(
                    "contact sheet SVG artifact sheet_id={sheet_id} manifest_hash={manifest_hash}"
                ))
            })?;
            (contact_sheet_svg_artifact_from_row(&row), false)
        };

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

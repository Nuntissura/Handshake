//! Collections + contact sheets (MT-018): named, ordered image sets with
//! notes/tags and optional character/sheet links, plus a contact-sheet manifest
//! that snapshots the membership (source asset ids + content hashes) at capture
//! time so a sheet stays reproducible even as collections evolve.
//!
//! CKC source: `app/backend/library.js` (`createCollection`, `updateCollection`,
//! `addImagesToCollection`, `removeImagesFromCollection`, `listCollectionImages`,
//! `createContactSheet`, `listContactSheets`) and `app/backend/db.js`
//! (`Collection`, `CollectionItem`, `ContactSheet` tables). Schema/behavior
//! intent only -- storage is PostgreSQL via sqlx, never the CKC SQLite layer.
//! MT ids: MT-003 (module boundary), MT-005 (event coverage), MT-018 (this fold-in).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierError, AtelierResult, AtelierStore};

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
    /// `ckc.contact_sheet@1`-shaped manifest: {schema, source_type, source_id,
    /// items:[{asset_id, content_hash}], tags, captured_at}.
    pub manifest: serde_json::Value,
    pub image_count: i64,
    pub created_at_utc: DateTime<Utc>,
}

const CONTACT_SHEET_MANIFEST_SCHEMA: &str = "ckc.contact_sheet@1";

/// New event families contributed by the collections fold-in (extends MT-005).
pub mod collections_event_family {
    pub const COLLECTION_CREATED: &str = "atelier.collection.created";
    pub const COLLECTION_UPDATED: &str = "atelier.collection.updated";
    pub const COLLECTION_IMAGES_ADDED: &str = "atelier.collection.images_added";
    pub const COLLECTION_IMAGES_REMOVED: &str = "atelier.collection.images_removed";
    pub const CONTACT_SHEET_CREATED: &str = "atelier.contact_sheet.created";

    /// All collections event families (used by parity/coverage checks).
    pub const ALL: &[&str] = &[
        COLLECTION_CREATED,
        COLLECTION_UPDATED,
        COLLECTION_IMAGES_ADDED,
        COLLECTION_IMAGES_REMOVED,
        CONTACT_SHEET_CREATED,
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

fn member_from_row(row: &sqlx::postgres::PgRow) -> CollectionMember {
    CollectionMember {
        collection_id: row.get("collection_id"),
        asset_id: row.get("asset_id"),
        content_hash: row.get("content_hash"),
        sort_order: row.get("sort_order"),
        added_at_utc: row.get("added_at_utc"),
    }
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

impl AtelierStore {
    /// Create a named collection. `name` must be non-empty and is unique. Tags
    /// are trimmed and de-duplicated. Optional character/sheet links are FK
    /// validated by the database.
    pub async fn create_collection(&self, new: &NewCollection) -> AtelierResult<Collection> {
        let name = new.name.trim();
        if name.is_empty() {
            return Err(AtelierError::Validation("collection name must not be empty".into()));
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
                "character_internal_id": collection.character_internal_id,
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
        // Validate the collection exists (clear error vs. an FK violation).
        self.get_collection(collection_id).await?;

        let mut tx = self.pool().begin().await?;
        let mut next_order: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM atelier_collection_item WHERE collection_id = $1",
        )
        .bind(collection_id)
        .fetch_one(&mut *tx)
        .await?;

        let mut inserted: i64 = 0;
        for asset_id in asset_ids {
            let affected = sqlx::query(
                r#"INSERT INTO atelier_collection_item (collection_id, asset_id, sort_order)
                   VALUES ($1, $2, $3)
                   ON CONFLICT (collection_id, asset_id) DO NOTHING"#,
            )
            .bind(collection_id)
            .bind(asset_id)
            .bind(next_order)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            if affected > 0 {
                inserted += 1;
                next_order += 1;
            }
        }

        if inserted > 0 {
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
        // collection's ordered membership (mirrors CKC `createContactSheet`).
        let members: Vec<(Uuid, String)> = if !asset_ids.is_empty() {
            let mut resolved = Vec::with_capacity(asset_ids.len());
            for asset_id in asset_ids {
                let hash: Option<String> = sqlx::query_scalar(
                    "SELECT content_hash FROM atelier_media_asset WHERE asset_id = $1",
                )
                .bind(asset_id)
                .fetch_optional(self.pool())
                .await?;
                let hash = hash.ok_or_else(|| {
                    AtelierError::NotFound(format!("media asset_id={asset_id}"))
                })?;
                resolved.push((*asset_id, hash));
            }
            resolved
        } else if st == "collection" {
            let cid = source_collection_id.ok_or_else(|| {
                AtelierError::Validation(
                    "source_collection_id is required for a collection-sourced contact sheet".into(),
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
}

//! Media assets / DAM (MT-015): media identity, provenance, and content-hash
//! dedup. Bytes live in the ArtifactStore (`artifact_ref`), never on random
//! filesystem paths and never in `.GOV`. Identity is stable across file moves.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierResult, AtelierStore};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaAsset {
    pub asset_id: Uuid,
    pub content_hash: String,
    pub mime: String,
    pub byte_len: i64,
    pub source_provenance: Option<String>,
    pub artifact_ref: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewMediaAsset {
    pub content_hash: String,
    pub mime: String,
    pub byte_len: i64,
    pub source_provenance: Option<String>,
    pub artifact_ref: String,
}

fn asset_from_row(row: &sqlx::postgres::PgRow) -> MediaAsset {
    MediaAsset {
        asset_id: row.get("asset_id"),
        content_hash: row.get("content_hash"),
        mime: row.get("mime"),
        byte_len: row.get("byte_len"),
        source_provenance: row.get("source_provenance"),
        artifact_ref: row.get("artifact_ref"),
        created_at_utc: row.get("created_at_utc"),
    }
}

impl AtelierStore {
    /// Materialize a media asset, deduplicating on `content_hash`. Re-ingesting
    /// identical bytes returns the existing asset (idempotent) rather than
    /// creating a duplicate row.
    pub async fn materialize_media_asset(
        &self,
        new: &NewMediaAsset,
    ) -> AtelierResult<MediaAsset> {
        // Fast path: existing asset by content hash.
        if let Some(existing) = self.get_media_asset_by_hash(&new.content_hash).await? {
            return Ok(existing);
        }
        let row = sqlx::query(
            r#"INSERT INTO atelier_media_asset
                 (content_hash, mime, byte_len, source_provenance, artifact_ref)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (content_hash) DO UPDATE SET content_hash = EXCLUDED.content_hash
               RETURNING asset_id, content_hash, mime, byte_len, source_provenance,
                         artifact_ref, created_at_utc"#,
        )
        .bind(&new.content_hash)
        .bind(&new.mime)
        .bind(new.byte_len)
        .bind(&new.source_provenance)
        .bind(&new.artifact_ref)
        .fetch_one(self.pool())
        .await?;
        let asset = asset_from_row(&row);
        self.record_event(
            event_family::MEDIA_ASSET_MATERIALIZED,
            "atelier_media_asset",
            &asset.content_hash,
            serde_json::json!({
                "asset_id": asset.asset_id,
                "mime": asset.mime,
                "byte_len": asset.byte_len,
            }),
        )
        .await?;
        Ok(asset)
    }

    pub async fn get_media_asset_by_hash(
        &self,
        content_hash: &str,
    ) -> AtelierResult<Option<MediaAsset>> {
        let row = sqlx::query(
            r#"SELECT asset_id, content_hash, mime, byte_len, source_provenance,
                      artifact_ref, created_at_utc
               FROM atelier_media_asset WHERE content_hash = $1"#,
        )
        .bind(content_hash)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(asset_from_row))
    }
}

//! Append-only character sheet versions (MT-012): updates never mutate prior
//! versions; each change is a new version with parent linkage and provenance,
//! preventing silent data loss when models or imports edit a sheet.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierResult, AtelierStore};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetVersion {
    pub version_id: Uuid,
    pub character_internal_id: Uuid,
    pub parent_version_id: Option<Uuid>,
    pub seq: i64,
    pub raw_text: String,
    pub author: String,
    pub tool: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewSheetVersion {
    pub character_internal_id: Uuid,
    pub raw_text: String,
    pub author: String,
    pub tool: Option<String>,
}

fn version_from_row(row: &sqlx::postgres::PgRow) -> SheetVersion {
    SheetVersion {
        version_id: row.get("version_id"),
        character_internal_id: row.get("character_internal_id"),
        parent_version_id: row.get("parent_version_id"),
        seq: row.get("seq"),
        raw_text: row.get("raw_text"),
        author: row.get("author"),
        tool: row.get("tool"),
        created_at_utc: row.get("created_at_utc"),
    }
}

impl AtelierStore {
    /// Append a new sheet version. Computes the next sequence number and links
    /// to the previous head as parent; never overwrites an existing version.
    pub async fn append_sheet_version(
        &self,
        new: &NewSheetVersion,
    ) -> AtelierResult<SheetVersion> {
        let mut tx = self.pool().begin().await?;

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM atelier_sheet_version WHERE character_internal_id = $1",
        )
        .bind(new.character_internal_id)
        .fetch_one(&mut *tx)
        .await?;

        let parent_version_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT version_id FROM atelier_sheet_version WHERE character_internal_id = $1 ORDER BY seq DESC LIMIT 1",
        )
        .bind(new.character_internal_id)
        .fetch_optional(&mut *tx)
        .await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_sheet_version
                 (character_internal_id, parent_version_id, seq, raw_text, author, tool)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING version_id, character_internal_id, parent_version_id, seq,
                         raw_text, author, tool, created_at_utc"#,
        )
        .bind(new.character_internal_id)
        .bind(parent_version_id)
        .bind(next_seq)
        .bind(&new.raw_text)
        .bind(&new.author)
        .bind(&new.tool)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let version = version_from_row(&row);
        self.record_event(
            event_family::SHEET_VERSION_APPENDED,
            "atelier_sheet_version",
            &version.character_internal_id.to_string(),
            serde_json::json!({
                "version_id": version.version_id,
                "seq": version.seq,
                "author": version.author,
            }),
        )
        .await?;
        Ok(version)
    }

    /// The current (highest-seq) sheet version for a character, if any.
    pub async fn latest_sheet_version(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Option<SheetVersion>> {
        let row = sqlx::query(
            r#"SELECT version_id, character_internal_id, parent_version_id, seq,
                      raw_text, author, tool, created_at_utc
               FROM atelier_sheet_version
               WHERE character_internal_id = $1
               ORDER BY seq DESC LIMIT 1"#,
        )
        .bind(character_internal_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(version_from_row))
    }

    /// Full append-only version history (ascending sequence).
    pub async fn sheet_version_history(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Vec<SheetVersion>> {
        let rows = sqlx::query(
            r#"SELECT version_id, character_internal_id, parent_version_id, seq,
                      raw_text, author, tool, created_at_utc
               FROM atelier_sheet_version
               WHERE character_internal_id = $1
               ORDER BY seq ASC"#,
        )
        .bind(character_internal_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(version_from_row).collect())
    }
}

//! Reusable artifact links attached to append-only CKC sheet versions.
//!
//! MT-016 keeps OpenPose and ComfyUI outputs as typed refs linked to a sheet
//! version, instead of copying artifact paths into raw character-sheet text.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{
    event_family, reject_legacy_runtime_ref, sheet_artifact_ref, sheet_version_ref, AtelierError,
    AtelierResult, AtelierStore,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SheetArtifactKind {
    OpenPoseJson,
    OpenPosePng,
    ConditioningPng,
    ComfyRender,
    ComfyReceipt,
}

impl SheetArtifactKind {
    pub fn as_token(self) -> &'static str {
        match self {
            SheetArtifactKind::OpenPoseJson => "openpose_json",
            SheetArtifactKind::OpenPosePng => "openpose_png",
            SheetArtifactKind::ConditioningPng => "conditioning_png",
            SheetArtifactKind::ComfyRender => "comfy_render",
            SheetArtifactKind::ComfyReceipt => "comfy_receipt",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "openpose_json" => Ok(Self::OpenPoseJson),
            "openpose_png" => Ok(Self::OpenPosePng),
            "conditioning_png" => Ok(Self::ConditioningPng),
            "comfy_render" => Ok(Self::ComfyRender),
            "comfy_receipt" => Ok(Self::ComfyReceipt),
            other => Err(AtelierError::Validation(format!(
                "unknown sheet artifact kind token: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewSheetArtifactLink {
    pub character_internal_id: Uuid,
    pub sheet_version_id: Uuid,
    pub artifact_kind: SheetArtifactKind,
    pub artifact_ref: String,
    pub manifest_ref: Option<String>,
    pub source_ref: Option<String>,
    pub label: Option<String>,
    pub reuse_role: Option<String>,
    pub linked_by: String,
    pub metadata: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetArtifactLink {
    pub link_id: Uuid,
    pub character_internal_id: Uuid,
    pub sheet_version_id: Uuid,
    pub sheet_version_ref: String,
    pub typed_ref: String,
    pub artifact_kind: SheetArtifactKind,
    pub artifact_ref: String,
    pub manifest_ref: Option<String>,
    pub source_ref: Option<String>,
    pub label: Option<String>,
    pub reuse_role: Option<String>,
    pub linked_by: String,
    pub metadata: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
    pub detached_at_utc: Option<DateTime<Utc>>,
    pub detached_by: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetArtifactLinkWrite {
    pub link: SheetArtifactLink,
    pub created: bool,
}

fn require_trimmed(field: &str, value: &str) -> AtelierResult<()> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(())
}

fn require_no_whitespace(field: &str, value: &str) -> AtelierResult<()> {
    if value.chars().any(char::is_whitespace) {
        return Err(AtelierError::Validation(format!(
            "{field} must not contain whitespace"
        )));
    }
    Ok(())
}

fn validate_optional_ref(field: &str, value: Option<&str>) -> AtelierResult<()> {
    if let Some(value) = value {
        reject_legacy_runtime_ref(field, value)?;
        require_no_whitespace(field, value)?;
    }
    Ok(())
}

fn validate_optional_trimmed(field: &str, value: Option<&str>) -> AtelierResult<()> {
    if let Some(value) = value {
        require_trimmed(field, value)?;
    }
    Ok(())
}

fn validate_optional_reuse_role(value: Option<&str>) -> AtelierResult<()> {
    if let Some(value) = value {
        require_trimmed("reuse_role", value)?;
        if !value.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-')
        }) || !value
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        {
            return Err(AtelierError::Validation(
                "reuse_role must match [a-z0-9][a-z0-9._-]*".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_new_sheet_artifact_link(new: &NewSheetArtifactLink) -> AtelierResult<()> {
    reject_legacy_runtime_ref("artifact_ref", &new.artifact_ref)?;
    require_no_whitespace("artifact_ref", &new.artifact_ref)?;
    validate_optional_ref("manifest_ref", new.manifest_ref.as_deref())?;
    validate_optional_ref("source_ref", new.source_ref.as_deref())?;
    validate_optional_trimmed("label", new.label.as_deref())?;
    validate_optional_reuse_role(new.reuse_role.as_deref())?;
    require_trimmed("linked_by", &new.linked_by)?;
    if !new.metadata.is_object() {
        return Err(AtelierError::Validation(
            "metadata must be a JSON object".to_string(),
        ));
    }
    Ok(())
}

fn sheet_artifact_link_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<SheetArtifactLink> {
    let link_id: Uuid = row.get("link_id");
    let character_internal_id: Uuid = row.get("character_internal_id");
    let sheet_version_id: Uuid = row.get("sheet_version_id");
    let artifact_kind_token: String = row.get("artifact_kind");
    Ok(SheetArtifactLink {
        link_id,
        character_internal_id,
        sheet_version_id,
        sheet_version_ref: sheet_version_ref(character_internal_id, sheet_version_id),
        typed_ref: sheet_artifact_ref(link_id),
        artifact_kind: SheetArtifactKind::from_token(&artifact_kind_token)?,
        artifact_ref: row.get("artifact_ref"),
        manifest_ref: row.get("manifest_ref"),
        source_ref: row.get("source_ref"),
        label: row.get("label"),
        reuse_role: row.get("reuse_role"),
        linked_by: row.get("linked_by"),
        metadata: row.get("metadata"),
        created_at_utc: row.get("created_at_utc"),
        detached_at_utc: row.get("detached_at_utc"),
        detached_by: row.get("detached_by"),
    })
}

impl AtelierStore {
    pub async fn link_sheet_artifact(
        &self,
        new: &NewSheetArtifactLink,
    ) -> AtelierResult<SheetArtifactLink> {
        Ok(self.link_sheet_artifact_with_status(new).await?.link)
    }

    pub async fn link_sheet_artifact_with_status(
        &self,
        new: &NewSheetArtifactLink,
    ) -> AtelierResult<SheetArtifactLinkWrite> {
        validate_new_sheet_artifact_link(new)?;
        let mut tx = self.pool().begin().await?;
        let sheet_character_internal_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT character_internal_id FROM atelier_sheet_version WHERE version_id = $1",
        )
        .bind(new.sheet_version_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(sheet_character_internal_id) = sheet_character_internal_id else {
            tx.rollback().await?;
            return Err(AtelierError::NotFound(format!(
                "sheet version version_id={}",
                new.sheet_version_id
            )));
        };
        if sheet_character_internal_id != new.character_internal_id {
            tx.rollback().await?;
            return Err(AtelierError::Validation(format!(
                "sheet_version_id={} does not belong to character_internal_id={}",
                new.sheet_version_id, new.character_internal_id
            )));
        }

        let inserted_row = sqlx::query(
            r#"INSERT INTO atelier_sheet_artifact_link
                 (character_internal_id, sheet_version_id, artifact_kind, artifact_ref,
                  manifest_ref, source_ref, label, reuse_role, linked_by, metadata)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               ON CONFLICT (sheet_version_id, artifact_kind, artifact_ref)
               WHERE detached_at_utc IS NULL
               DO NOTHING
               RETURNING link_id, character_internal_id, sheet_version_id, artifact_kind,
                         artifact_ref, manifest_ref, source_ref, label, reuse_role,
                         linked_by, metadata, created_at_utc, detached_at_utc, detached_by"#,
        )
        .bind(new.character_internal_id)
        .bind(new.sheet_version_id)
        .bind(new.artifact_kind.as_token())
        .bind(&new.artifact_ref)
        .bind(&new.manifest_ref)
        .bind(&new.source_ref)
        .bind(&new.label)
        .bind(&new.reuse_role)
        .bind(&new.linked_by)
        .bind(&new.metadata)
        .fetch_optional(&mut *tx)
        .await?;

        let (link, inserted) = if let Some(row) = inserted_row {
            (sheet_artifact_link_from_row(&row)?, true)
        } else {
            let row = sqlx::query(
                r#"SELECT link_id, character_internal_id, sheet_version_id, artifact_kind,
                          artifact_ref, manifest_ref, source_ref, label, reuse_role,
                          linked_by, metadata, created_at_utc, detached_at_utc, detached_by
                   FROM atelier_sheet_artifact_link
                   WHERE sheet_version_id = $1
                     AND artifact_kind = $2
                     AND artifact_ref = $3
                     AND detached_at_utc IS NULL"#,
            )
            .bind(new.sheet_version_id)
            .bind(new.artifact_kind.as_token())
            .bind(&new.artifact_ref)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                AtelierError::Conflict(format!(
                    "active sheet artifact link disappeared for sheet_version_id={} artifact_ref={}",
                    new.sheet_version_id, new.artifact_ref
                ))
            })?;
            (sheet_artifact_link_from_row(&row)?, false)
        };
        if inserted {
            self.record_event_in_tx(
                &mut tx,
                event_family::SHEET_ARTIFACT_LINKED,
                "atelier_sheet_artifact_link",
                &link.link_id.to_string(),
                serde_json::json!({
                    "link_id": link.link_id,
                    "typed_ref": link.typed_ref,
                    "sheet_version_ref": link.sheet_version_ref,
                    "artifact_kind": link.artifact_kind.as_token(),
                    "artifact_ref": link.artifact_ref,
                    "reuse_role": link.reuse_role,
                    "linked_by": link.linked_by,
                }),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(SheetArtifactLinkWrite {
            link,
            created: inserted,
        })
    }

    pub async fn get_sheet_artifact(&self, link_id: Uuid) -> AtelierResult<SheetArtifactLink> {
        let row = sqlx::query(
            r#"SELECT link_id, character_internal_id, sheet_version_id, artifact_kind,
                      artifact_ref, manifest_ref, source_ref, label, reuse_role,
                      linked_by, metadata, created_at_utc, detached_at_utc, detached_by
               FROM atelier_sheet_artifact_link
               WHERE link_id = $1
                 AND detached_at_utc IS NULL"#,
        )
        .bind(link_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("sheet artifact link_id={link_id}")))?;
        sheet_artifact_link_from_row(&row)
    }

    pub async fn list_sheet_artifacts(
        &self,
        sheet_version_id: Uuid,
    ) -> AtelierResult<Vec<SheetArtifactLink>> {
        let rows = sqlx::query(
            r#"SELECT link_id, character_internal_id, sheet_version_id, artifact_kind,
                      artifact_ref, manifest_ref, source_ref, label, reuse_role,
                      linked_by, metadata, created_at_utc, detached_at_utc, detached_by
               FROM atelier_sheet_artifact_link
               WHERE sheet_version_id = $1
                 AND detached_at_utc IS NULL
               ORDER BY created_at_utc, link_id"#,
        )
        .bind(sheet_version_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(sheet_artifact_link_from_row).collect()
    }

    pub async fn detach_sheet_artifact(
        &self,
        link_id: Uuid,
        detached_by: &str,
    ) -> AtelierResult<SheetArtifactLink> {
        require_trimmed("detached_by", detached_by)?;
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"UPDATE atelier_sheet_artifact_link
               SET detached_at_utc = NOW(),
                   detached_by = $2
               WHERE link_id = $1
                 AND detached_at_utc IS NULL
               RETURNING link_id, character_internal_id, sheet_version_id, artifact_kind,
                         artifact_ref, manifest_ref, source_ref, label, reuse_role,
                         linked_by, metadata, created_at_utc, detached_at_utc, detached_by"#,
        )
        .bind(link_id)
        .bind(detached_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("sheet artifact link_id={link_id}")))?;
        let link = sheet_artifact_link_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            event_family::SHEET_ARTIFACT_DETACHED,
            "atelier_sheet_artifact_link",
            &link.link_id.to_string(),
            serde_json::json!({
                "link_id": link.link_id,
                "typed_ref": link.typed_ref,
                "sheet_version_ref": link.sheet_version_ref,
                "artifact_kind": link.artifact_kind.as_token(),
                "artifact_ref": link.artifact_ref,
                "detached_by": link.detached_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(link)
    }
}

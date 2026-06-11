//! Filesystem health diagnostics (MT-023).
//!
//! This module preserves the legacy health-check intent as read-only
//! diagnostics over governed PostgreSQL state. It records health snapshots and
//! findings, but it never resyncs, deletes, repairs, or creates media rows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::fs;
use uuid::Uuid;

use super::{AtelierError, AtelierResult, AtelierStore};
use crate::storage::artifacts::{
    artifact_root_rel, artifact_store_root, read_artifact_manifest, resolve_workspace_root,
    validate_artifact_content_hash, ArtifactLayer,
};

pub mod filesystem_health_event_family {
    pub const CHECK_RECORDED: &str = "atelier.filesystem_health.check_recorded";
    pub const ALL: &[&str] = &[CHECK_RECORDED];
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemHealthFindingKind {
    MissingOriginal,
    MissingThumbnail,
    InboxPending,
    UntrackedOriginal,
    SidecarVisibilityAnomaly,
}

impl FilesystemHealthFindingKind {
    pub fn as_token(self) -> &'static str {
        match self {
            FilesystemHealthFindingKind::MissingOriginal => "missing_original",
            FilesystemHealthFindingKind::MissingThumbnail => "missing_thumbnail",
            FilesystemHealthFindingKind::InboxPending => "inbox_pending",
            FilesystemHealthFindingKind::UntrackedOriginal => "untracked_original",
            FilesystemHealthFindingKind::SidecarVisibilityAnomaly => "sidecar_visibility_anomaly",
        }
    }

    fn from_token(value: &str) -> AtelierResult<Self> {
        match value {
            "missing_original" => Ok(FilesystemHealthFindingKind::MissingOriginal),
            "missing_thumbnail" => Ok(FilesystemHealthFindingKind::MissingThumbnail),
            "inbox_pending" => Ok(FilesystemHealthFindingKind::InboxPending),
            "untracked_original" => Ok(FilesystemHealthFindingKind::UntrackedOriginal),
            "sidecar_visibility_anomaly" => {
                Ok(FilesystemHealthFindingKind::SidecarVisibilityAnomaly)
            }
            other => Err(AtelierError::Validation(format!(
                "unsupported filesystem health finding kind: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilesystemHealthCheck {
    pub check_id: Uuid,
    pub requested_by: String,
    pub scope_label: Option<String>,
    pub summary: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilesystemHealthFinding {
    pub finding_id: Uuid,
    pub check_id: Uuid,
    pub finding_kind: FilesystemHealthFindingKind,
    pub target_type: String,
    pub target_id: String,
    pub details: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilesystemHealthReport {
    pub check: FilesystemHealthCheck,
    pub findings: Vec<FilesystemHealthFinding>,
}

#[derive(Clone, Debug)]
pub struct FilesystemHealthCheckRequest {
    pub requested_by: String,
    pub scope_label: Option<String>,
}

#[derive(Clone, Debug)]
struct PendingFilesystemHealthFinding {
    finding_kind: FilesystemHealthFindingKind,
    target_type: &'static str,
    target_id: String,
    details: serde_json::Value,
}

fn check_from_row(row: &sqlx::postgres::PgRow) -> FilesystemHealthCheck {
    FilesystemHealthCheck {
        check_id: row.get("check_id"),
        requested_by: row.get("requested_by"),
        scope_label: row.get("scope_label"),
        summary: row.get("summary"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn finding_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<FilesystemHealthFinding> {
    let finding_kind: String = row.get("finding_kind");
    Ok(FilesystemHealthFinding {
        finding_id: row.get("finding_id"),
        check_id: row.get("check_id"),
        finding_kind: FilesystemHealthFindingKind::from_token(&finding_kind)?,
        target_type: row.get("target_type"),
        target_id: row.get("target_id"),
        details: row.get("details"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn require_requested_by(requested_by: &str) -> AtelierResult<&str> {
    let requested_by = requested_by.trim();
    if requested_by.is_empty() {
        return Err(AtelierError::Validation(
            "filesystem health requested_by must not be empty".into(),
        ));
    }
    Ok(requested_by)
}

fn normalize_scope_label(scope_label: &Option<String>) -> AtelierResult<Option<String>> {
    match scope_label.as_deref() {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else if trimmed != raw {
                Err(AtelierError::Validation(
                    "filesystem health scope_label must not be padded".into(),
                ))
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
    }
}

fn parse_native_artifact_payload_ref(artifact_ref: &str) -> Result<(ArtifactLayer, Uuid), String> {
    let body = artifact_ref
        .strip_prefix("artifact://")
        .ok_or_else(|| "artifact_ref missing artifact:// scheme".to_string())?;
    let parts: Vec<&str> = body.split('/').collect();
    if parts.len() != 5
        || parts[0] != ".handshake"
        || parts[1] != "artifacts"
        || parts[4] != "payload"
    {
        return Err(
            "artifact_ref must point to artifact://.handshake/artifacts/<layer>/<uuid>/payload"
                .to_string(),
        );
    }
    let layer = match parts[2] {
        "L1" => ArtifactLayer::L1,
        "L2" => ArtifactLayer::L2,
        "L3" => ArtifactLayer::L3,
        "L4" => ArtifactLayer::L4,
        other => return Err(format!("unsupported ArtifactStore layer: {other}")),
    };
    let artifact_id = Uuid::parse_str(parts[3])
        .map_err(|err| format!("invalid ArtifactStore artifact id: {err}"))?;
    Ok((layer, artifact_id))
}

fn expected_artifact_manifest_ref(layer: ArtifactLayer, artifact_id: Uuid) -> String {
    format!(
        "artifact://{}/artifact.json",
        artifact_root_rel(layer, artifact_id)
    )
}

fn normalized_sha256_hex(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed != value {
        return None;
    }
    let hex = trimmed.strip_prefix("sha256:").unwrap_or(trimmed);
    if hex.len() == 64 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(hex.to_ascii_lowercase())
    } else {
        None
    }
}

fn artifact_payload_health_issue(
    artifact_ref: &str,
    expected_content_hash: Option<&str>,
    expected_byte_len: Option<i64>,
    expected_mime: Option<&str>,
    expected_manifest_ref: Option<&str>,
) -> Option<String> {
    if artifact_ref.to_ascii_lowercase().contains(".gov") {
        return Some("artifact_ref points at forbidden .GOV path".to_string());
    }
    let (layer, artifact_id) = match parse_native_artifact_payload_ref(artifact_ref) {
        Ok(parsed) => parsed,
        Err(err) => return Some(format!("invalid native ArtifactStore payload ref: {err}")),
    };
    if let Some(actual_manifest_ref) = expected_manifest_ref {
        let expected = expected_artifact_manifest_ref(layer, artifact_id);
        if actual_manifest_ref != expected {
            return Some(format!(
                "ArtifactStore manifest ref mismatch: expected {expected}"
            ));
        }
    }
    let workspace_root = match resolve_workspace_root() {
        Ok(root) => root,
        Err(err) => return Some(format!("ArtifactStore root unavailable: {err}")),
    };
    let manifest = match read_artifact_manifest(&workspace_root, layer, artifact_id) {
        Ok(manifest) => manifest,
        Err(err) => return Some(format!("ArtifactStore manifest validation failed: {err}")),
    };
    if manifest.artifact_id != artifact_id || manifest.layer != layer {
        return Some("ArtifactStore manifest identity mismatch".to_string());
    }
    if let Err(err) = validate_artifact_content_hash(&workspace_root, layer, artifact_id) {
        return Some(format!(
            "ArtifactStore content hash validation failed: {err}"
        ));
    }
    if let Some(expected_hash) = expected_content_hash {
        let Some(expected_hash) = normalized_sha256_hex(expected_hash) else {
            return Some("row content_hash is not a valid sha256 value".to_string());
        };
        if !manifest.content_hash.eq_ignore_ascii_case(&expected_hash) {
            return Some("row content_hash does not match ArtifactStore manifest".to_string());
        }
    }
    if let Some(expected_byte_len) = expected_byte_len {
        if expected_byte_len <= 0 || manifest.size_bytes != expected_byte_len as u64 {
            return Some("row byte_len does not match ArtifactStore manifest".to_string());
        }
    }
    if let Some(expected_mime) = expected_mime {
        if expected_mime.trim().is_empty() || manifest.mime != expected_mime {
            return Some("row mime does not match ArtifactStore manifest".to_string());
        }
    }
    None
}

impl AtelierStore {
    pub async fn run_filesystem_health_check(
        &self,
        request: &FilesystemHealthCheckRequest,
    ) -> AtelierResult<FilesystemHealthReport> {
        let requested_by = require_requested_by(&request.requested_by)?;
        let scope_label = normalize_scope_label(&request.scope_label)?;

        let mut pending = Vec::new();
        collect_missing_originals(self, &mut pending).await?;
        collect_missing_thumbnails(self, &mut pending).await?;
        collect_inbox_pending(self, &mut pending).await?;
        collect_untracked_originals(self, &mut pending).await?;
        collect_sidecar_visibility_anomalies(self, &mut pending).await?;
        let sidecars_checked: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM atelier_media_sidecar")
                .fetch_one(self.pool())
                .await?;

        let summary = filesystem_health_summary(&pending, sidecars_checked);
        let check_id = Uuid::now_v7();
        let mut tx = self.pool().begin().await?;
        let check_row = sqlx::query(
            r#"INSERT INTO atelier_filesystem_health_check
                 (check_id, requested_by, scope_label, summary)
               VALUES ($1, $2, $3, $4)
               RETURNING check_id, requested_by, scope_label, summary, created_at_utc"#,
        )
        .bind(check_id)
        .bind(requested_by)
        .bind(&scope_label)
        .bind(&summary)
        .fetch_one(&mut *tx)
        .await?;
        let check = check_from_row(&check_row);

        let mut findings = Vec::with_capacity(pending.len());
        for finding in pending {
            let finding_id = Uuid::now_v7();
            let row = sqlx::query(
                r#"INSERT INTO atelier_filesystem_health_finding
                     (finding_id, check_id, finding_kind, target_type, target_id, details)
                   VALUES ($1, $2, $3, $4, $5, $6)
                   RETURNING finding_id, check_id, finding_kind, target_type,
                             target_id, details, created_at_utc"#,
            )
            .bind(finding_id)
            .bind(check.check_id)
            .bind(finding.finding_kind.as_token())
            .bind(finding.target_type)
            .bind(&finding.target_id)
            .bind(&finding.details)
            .fetch_one(&mut *tx)
            .await?;
            findings.push(finding_from_row(&row)?);
        }

        self.record_event_in_tx(
            &mut tx,
            filesystem_health_event_family::CHECK_RECORDED,
            "atelier_filesystem_health_check",
            &check.check_id.to_string(),
            serde_json::json!({
                "check_id": check.check_id,
                "requested_by": requested_by,
                "scope_label": scope_label,
                "summary": check.summary,
                "finding_count": findings.len(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(FilesystemHealthReport { check, findings })
    }

    pub async fn list_filesystem_health_findings(
        &self,
        check_id: Uuid,
    ) -> AtelierResult<Vec<FilesystemHealthFinding>> {
        let rows = sqlx::query(
            r#"SELECT finding_id, check_id, finding_kind, target_type,
                      target_id, details, created_at_utc
               FROM atelier_filesystem_health_finding
               WHERE check_id = $1
               ORDER BY finding_kind, target_type, target_id, finding_id"#,
        )
        .bind(check_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(finding_from_row).collect()
    }
}

async fn collect_missing_originals(
    store: &AtelierStore,
    findings: &mut Vec<PendingFilesystemHealthFinding>,
) -> AtelierResult<()> {
    let rows = sqlx::query(
        r#"SELECT asset_id, content_hash, mime, byte_len, artifact_ref,
                  artifact_manifest->>'validation_state' AS validation_state
           FROM atelier_media_asset"#,
    )
    .fetch_all(store.pool())
    .await?;
    for row in rows {
        let asset_id: Uuid = row.get("asset_id");
        let artifact_ref: String = row.get("artifact_ref");
        let content_hash: String = row.get("content_hash");
        let mime: String = row.get("mime");
        let byte_len: i64 = row.get("byte_len");
        let validation_state = row.get::<Option<String>, _>("validation_state");
        let issue = validation_state
            .as_deref()
            .filter(|state| *state == "invalid_legacy_artifact_ref")
            .map(|state| format!("artifact_manifest validation_state={state}"))
            .or_else(|| {
                artifact_payload_health_issue(
                    &artifact_ref,
                    Some(&content_hash),
                    Some(byte_len),
                    Some(&mime),
                    None,
                )
            });
        let Some(issue) = issue else {
            continue;
        };
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::MissingOriginal,
            target_type: "atelier_media_asset",
            target_id: asset_id.to_string(),
            details: serde_json::json!({
                "asset_id": asset_id,
                "content_hash": content_hash,
                "artifact_ref": artifact_ref,
                "validation_state": validation_state,
                "artifact_issue": issue,
            }),
        });
    }
    Ok(())
}

async fn collect_missing_thumbnails(
    store: &AtelierStore,
    findings: &mut Vec<PendingFilesystemHealthFinding>,
) -> AtelierResult<()> {
    let rows = sqlx::query(
        r#"SELECT a.asset_id, a.content_hash
           FROM atelier_media_asset a
           WHERE NOT EXISTS (
               SELECT 1
               FROM atelier_media_sidecar s
               WHERE s.sidecar_asset_id = a.asset_id
                 AND s.hidden_from_gallery
           )
             AND NOT EXISTS (
               SELECT 1
               FROM atelier_media_derivative d
               WHERE d.asset_id = a.asset_id
                 AND d.derivative_kind = 'thumbnail'
                 AND d.status = 'generated'
                 AND d.artifact_ref IS NOT NULL
           )"#,
    )
    .fetch_all(store.pool())
    .await?;
    for row in rows {
        let asset_id: Uuid = row.get("asset_id");
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::MissingThumbnail,
            target_type: "atelier_media_asset",
            target_id: asset_id.to_string(),
            details: serde_json::json!({
                "asset_id": asset_id,
                "content_hash": row.get::<String, _>("content_hash"),
                "required_derivative_kind": "thumbnail",
            }),
        });
    }
    let rows = sqlx::query(
        r#"SELECT d.derivative_id, d.asset_id, d.derivative_kind, d.artifact_ref,
                  d.artifact_manifest_ref, d.mime, d.byte_len, a.content_hash
           FROM atelier_media_derivative d
           JOIN atelier_media_asset a ON a.asset_id = d.asset_id
           WHERE d.derivative_kind = 'thumbnail'
             AND d.status = 'generated'
             AND d.artifact_ref IS NOT NULL"#,
    )
    .fetch_all(store.pool())
    .await?;
    for row in rows {
        let artifact_ref: String = row.get("artifact_ref");
        let artifact_manifest_ref: Option<String> = row.get("artifact_manifest_ref");
        let mime: Option<String> = row.get("mime");
        let byte_len: Option<i64> = row.get("byte_len");
        let issue = artifact_payload_health_issue(
            &artifact_ref,
            None,
            byte_len,
            mime.as_deref(),
            artifact_manifest_ref.as_deref(),
        );
        let Some(issue) = issue else {
            continue;
        };
        let derivative_id: Uuid = row.get("derivative_id");
        let asset_id: Uuid = row.get("asset_id");
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::MissingThumbnail,
            target_type: "atelier_media_derivative",
            target_id: derivative_id.to_string(),
            details: serde_json::json!({
                "derivative_id": derivative_id,
                "asset_id": asset_id,
                "content_hash": row.get::<String, _>("content_hash"),
                "derivative_kind": row.get::<String, _>("derivative_kind"),
                "artifact_ref": artifact_ref,
                "artifact_manifest_ref": artifact_manifest_ref,
                "mime": mime,
                "byte_len": byte_len,
                "artifact_issue": issue,
            }),
        });
    }
    Ok(())
}

async fn collect_inbox_pending(
    store: &AtelierStore,
    findings: &mut Vec<PendingFilesystemHealthFinding>,
) -> AtelierResult<()> {
    let rows = sqlx::query(
        r#"SELECT item_id, batch_id, source_path, file_name, lane
           FROM atelier_intake_item
           WHERE lane IN ('new', 'deferred')"#,
    )
    .fetch_all(store.pool())
    .await?;
    for row in rows {
        let item_id: Uuid = row.get("item_id");
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::InboxPending,
            target_type: "atelier_intake_item",
            target_id: item_id.to_string(),
            details: serde_json::json!({
                "item_id": item_id,
                "batch_id": row.get::<Uuid, _>("batch_id"),
                "source_path": row.get::<String, _>("source_path"),
                "file_name": row.get::<String, _>("file_name"),
                "lane": row.get::<String, _>("lane"),
            }),
        });
    }
    Ok(())
}

async fn collect_untracked_originals(
    store: &AtelierStore,
    findings: &mut Vec<PendingFilesystemHealthFinding>,
) -> AtelierResult<()> {
    let rows = sqlx::query(
        r#"SELECT i.item_id, i.batch_id, i.source_path, i.file_name, i.content_hash
           FROM atelier_intake_item i
           WHERE i.content_hash IS NULL
              OR NOT EXISTS (
                SELECT 1
                FROM atelier_media_asset a
                WHERE a.content_hash = regexp_replace(lower(i.content_hash), '^sha256:', '')
              )"#,
    )
    .fetch_all(store.pool())
    .await?;
    for row in rows {
        let item_id: Uuid = row.get("item_id");
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::UntrackedOriginal,
            target_type: "atelier_intake_item",
            target_id: item_id.to_string(),
            details: serde_json::json!({
                "item_id": item_id,
                "batch_id": row.get::<Uuid, _>("batch_id"),
                "source_path": row.get::<String, _>("source_path"),
                "file_name": row.get::<String, _>("file_name"),
                "content_hash": row.get::<Option<String>, _>("content_hash"),
            }),
        });
    }
    collect_untracked_artifactstore_payloads(store, findings).await?;
    Ok(())
}

async fn collect_untracked_artifactstore_payloads(
    store: &AtelierStore,
    findings: &mut Vec<PendingFilesystemHealthFinding>,
) -> AtelierResult<()> {
    let workspace_root = resolve_workspace_root().map_err(|err| {
        AtelierError::Validation(format!("ArtifactStore root unavailable: {err}"))
    })?;
    let artifact_store = artifact_store_root(&workspace_root);
    if !artifact_store.exists() {
        return Ok(());
    }
    for layer in [
        ArtifactLayer::L1,
        ArtifactLayer::L2,
        ArtifactLayer::L3,
        ArtifactLayer::L4,
    ] {
        let layer_dir = artifact_store.join(layer.as_str());
        let Ok(entries) = fs::read_dir(&layer_dir) else {
            continue;
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let artifact_id = match entry
                .file_name()
                .to_str()
                .and_then(|value| Uuid::parse_str(value).ok())
            {
                Some(artifact_id) => artifact_id,
                None => continue,
            };
            if artifact_payload_health_issue(
                &format!(
                    "artifact://{}/payload",
                    artifact_root_rel(layer, artifact_id)
                ),
                None,
                None,
                None,
                None,
            )
            .is_some()
            {
                continue;
            }
            let manifest = match read_artifact_manifest(&workspace_root, layer, artifact_id) {
                Ok(manifest) => manifest,
                Err(_) => continue,
            };
            let content_hash = manifest.content_hash.to_ascii_lowercase();
            let artifact_ref = format!(
                "artifact://{}/payload",
                artifact_root_rel(layer, artifact_id)
            );
            let artifact_manifest_ref = format!(
                "artifact://{}/artifact.json",
                artifact_root_rel(layer, artifact_id)
            );
            let tracked: bool = sqlx::query_scalar(
                r#"SELECT
                       EXISTS (
                           SELECT 1
                           FROM atelier_media_asset
                           WHERE artifact_ref = $1
                              OR lower(regexp_replace(content_hash, '^sha256:', '')) = $2
                       )
                       OR EXISTS (
                           SELECT 1
                           FROM atelier_intake_item
                           WHERE source_path = $1
                              OR lower(regexp_replace(COALESCE(content_hash, ''), '^sha256:', '')) = $2
                       )
                       OR EXISTS (
                           SELECT 1
                           FROM atelier_media_derivative
                           WHERE artifact_ref = $1
                              OR artifact_manifest_ref = $3
                       )"#,
            )
            .bind(&artifact_ref)
            .bind(&content_hash)
            .bind(&artifact_manifest_ref)
            .fetch_one(store.pool())
            .await?;
            if tracked {
                continue;
            }
            findings.push(PendingFilesystemHealthFinding {
                finding_kind: FilesystemHealthFindingKind::UntrackedOriginal,
                target_type: "artifact_store_payload",
                target_id: artifact_ref.clone(),
                details: serde_json::json!({
                    "artifact_ref": artifact_ref,
                    "artifact_id": artifact_id,
                    "layer": layer.as_str(),
                    "content_hash": content_hash,
                    "mime": manifest.mime,
                    "size_bytes": manifest.size_bytes,
                    "payload_exists": true,
                    "manifest_exists": true,
                }),
            });
        }
    }
    Ok(())
}

async fn collect_sidecar_visibility_anomalies(
    store: &AtelierStore,
    findings: &mut Vec<PendingFilesystemHealthFinding>,
) -> AtelierResult<()> {
    let rows = sqlx::query(
        r#"SELECT sidecar_id, parent_asset_id, sidecar_asset_id, relation_kind,
                  hidden_from_gallery, searchable_by_relation
           FROM atelier_media_sidecar
           WHERE hidden_from_gallery IS DISTINCT FROM TRUE
              OR searchable_by_relation IS DISTINCT FROM TRUE"#,
    )
    .fetch_all(store.pool())
    .await?;
    for row in rows {
        let sidecar_id: Uuid = row.get("sidecar_id");
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::SidecarVisibilityAnomaly,
            target_type: "atelier_media_sidecar",
            target_id: sidecar_id.to_string(),
            details: serde_json::json!({
                "sidecar_id": sidecar_id,
                "parent_asset_id": row.get::<Uuid, _>("parent_asset_id"),
                "sidecar_asset_id": row.get::<Uuid, _>("sidecar_asset_id"),
                "relation_kind": row.get::<String, _>("relation_kind"),
                "hidden_from_gallery": row.get::<bool, _>("hidden_from_gallery"),
                "searchable_by_relation": row.get::<bool, _>("searchable_by_relation"),
            }),
        });
    }
    Ok(())
}

fn filesystem_health_summary(
    findings: &[PendingFilesystemHealthFinding],
    sidecars_checked: i64,
) -> serde_json::Value {
    let count = |kind: FilesystemHealthFindingKind| -> usize {
        findings
            .iter()
            .filter(|finding| finding.finding_kind == kind)
            .count()
    };
    serde_json::json!({
        "missing_originals_count": count(FilesystemHealthFindingKind::MissingOriginal),
        "missing_thumbnails_count": count(FilesystemHealthFindingKind::MissingThumbnail),
        "inbox_pending_count": count(FilesystemHealthFindingKind::InboxPending),
        "untracked_originals_count": count(FilesystemHealthFindingKind::UntrackedOriginal),
        "sidecar_visibility_anomalies_count": count(FilesystemHealthFindingKind::SidecarVisibilityAnomaly),
        "sidecars_checked_count": sidecars_checked,
        "auto_resync": false,
        "auto_delete": false,
    })
}

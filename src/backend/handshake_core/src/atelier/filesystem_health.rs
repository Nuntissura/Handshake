//! Filesystem health diagnostics (MT-023).
//!
//! This module preserves the legacy health-check intent as read-only
//! diagnostics over governed durable state. It records health snapshots and
//! findings, but it never resyncs, deletes, repairs, or creates media rows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{atelier_event_sql, AtelierError, AtelierResult, AtelierStore};
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

/// One `atelier_filesystem_health_check` row as the store returns it.
#[derive(SurrealValue)]
struct HealthCheckRow {
    check_id: SurrealUuid,
    requested_by: String,
    scope_label: Option<String>,
    summary: serde_json::Value,
    created_at_utc: Datetime,
}

impl From<HealthCheckRow> for FilesystemHealthCheck {
    fn from(row: HealthCheckRow) -> Self {
        FilesystemHealthCheck {
            check_id: row.check_id.into(),
            requested_by: row.requested_by,
            scope_label: row.scope_label,
            summary: row.summary,
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

/// One `atelier_filesystem_health_finding` row as the store returns it.
#[derive(SurrealValue)]
struct HealthFindingRow {
    finding_id: SurrealUuid,
    check_id: SurrealUuid,
    finding_kind: String,
    target_type: String,
    target_id: String,
    details: serde_json::Value,
    created_at_utc: Datetime,
}

impl TryFrom<HealthFindingRow> for FilesystemHealthFinding {
    type Error = AtelierError;

    fn try_from(row: HealthFindingRow) -> AtelierResult<Self> {
        Ok(FilesystemHealthFinding {
            finding_id: row.finding_id.into(),
            check_id: row.check_id.into(),
            finding_kind: FilesystemHealthFindingKind::from_token(&row.finding_kind)?,
            target_type: row.target_type,
            target_id: row.target_id,
            details: row.details,
            created_at_utc: row.created_at_utc.into(),
        })
    }
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

/// One finding travelling to [`RECORD_HEALTH_CHECK_STATEMENT`].
#[derive(Clone, SurrealValue)]
struct HealthFindingInsert {
    finding_id: SurrealUuid,
    finding_kind: String,
    target_type: String,
    target_id: String,
    details: serde_json::Value,
}

#[derive(Clone, SurrealValue)]
struct RecordHealthCheckBindings {
    check_rid: RecordId,
    check_id: SurrealUuid,
    requested_by: String,
    scope_label: Option<String>,
    summary: serde_json::Value,
    findings: Vec<HealthFindingInsert>,
}

/// The check row, every finding row, and the recorded event commit together
/// in one atomic statement (the former single transaction).
const RECORD_HEALTH_CHECK_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.check_rid; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         check_id: $domain.check_id, \
         requested_by: $domain.requested_by, \
         scope_label: $domain.scope_label, \
         summary: $domain.summary \
       }; \
       FOR $f IN $domain.findings { \
         CREATE type::record('atelier_filesystem_health_finding', $f.finding_id) CONTENT { \
           finding_id: $f.finding_id, \
           check_id: $rid, \
           finding_kind: $f.finding_kind, \
           target_type: $f.target_type, \
           target_id: $f.target_id, \
           details: $f.details \
         }; \
       }; \
       RETURN { \
         check: (SELECT check_id, requested_by, scope_label, summary, created_at_utc FROM $rid), \
         findings: (SELECT finding_id, record::id(check_id) AS check_id, finding_kind, \
                           target_type, target_id, details, created_at_utc \
                    FROM atelier_filesystem_health_finding WHERE check_id = $rid \
                    ORDER BY finding_kind ASC, target_type ASC, target_id ASC, finding_id ASC) \
       }; };"
);

/// The outcome object [`RECORD_HEALTH_CHECK_STATEMENT`] returns.
#[derive(SurrealValue)]
struct RecordHealthCheckOutcome {
    check: Vec<HealthCheckRow>,
    findings: Vec<HealthFindingRow>,
}

#[derive(SurrealValue)]
struct CheckRefBinding {
    check_ref: RecordId,
}

const LIST_HEALTH_FINDINGS_STATEMENT: &str =
    "SELECT finding_id, record::id(check_id) AS check_id, finding_kind, target_type, \
            target_id, details, created_at_utc \
     FROM atelier_filesystem_health_finding \
     WHERE check_id = $check_ref \
     ORDER BY finding_kind ASC, target_type ASC, target_id ASC, finding_id ASC;";

#[derive(SurrealValue)]
struct NoBindings {}

const COUNT_SIDECARS_STATEMENT: &str = "RETURN count(SELECT id FROM atelier_media_sidecar);";

/// Media-asset fields the missing-original sweep reads. The manifest
/// validation state is projected out of the flexible manifest object.
#[derive(SurrealValue)]
struct MissingOriginalRow {
    asset_id: SurrealUuid,
    content_hash: String,
    mime: String,
    byte_len: i64,
    artifact_ref: String,
    validation_state: Option<String>,
}

const MISSING_ORIGINALS_STATEMENT: &str =
    "SELECT asset_id, content_hash, mime, byte_len, artifact_ref, \
            artifact_manifest.validation_state AS validation_state \
     FROM atelier_media_asset;";

#[derive(SurrealValue)]
struct AssetHashRow {
    asset_id: SurrealUuid,
    content_hash: String,
}

/// Assets with neither a hiding sidecar nor a generated thumbnail.
const ASSETS_MISSING_THUMBNAIL_STATEMENT: &str =
    "SELECT asset_id, content_hash FROM atelier_media_asset \
     WHERE (SELECT VALUE id FROM atelier_media_sidecar \
            WHERE sidecar_asset_id = $parent.id AND hidden_from_gallery = true) = [] \
       AND (SELECT VALUE id FROM atelier_media_derivative \
            WHERE asset_id = $parent.id AND derivative_kind = 'thumbnail' \
              AND status = 'generated' AND artifact_ref != NONE) = [];";

#[derive(SurrealValue)]
struct GeneratedThumbnailRow {
    derivative_id: SurrealUuid,
    asset_id: SurrealUuid,
    derivative_kind: String,
    artifact_ref: Option<String>,
    artifact_manifest_ref: Option<String>,
    mime: Option<String>,
    byte_len: Option<i64>,
    content_hash: String,
}

/// Generated thumbnails joined to their parent asset's content hash via the
/// record link.
const GENERATED_THUMBNAILS_STATEMENT: &str =
    "SELECT derivative_id, record::id(asset_id) AS asset_id, derivative_kind, artifact_ref, \
            artifact_manifest_ref, mime, byte_len, asset_id.content_hash AS content_hash \
     FROM atelier_media_derivative \
     WHERE derivative_kind = 'thumbnail' AND status = 'generated' AND artifact_ref != NONE;";

#[derive(SurrealValue)]
struct IntakeItemRow {
    item_id: SurrealUuid,
    batch_id: SurrealUuid,
    source_path: String,
    file_name: String,
    lane: String,
}

const INBOX_PENDING_STATEMENT: &str =
    "SELECT item_id, record::id(batch_id) AS batch_id, source_path, file_name, lane \
     FROM atelier_intake_item WHERE lane IN ['pending', 'deferred'];";

#[derive(SurrealValue)]
struct UntrackedIntakeItemRow {
    item_id: SurrealUuid,
    batch_id: SurrealUuid,
    source_path: String,
    file_name: String,
    content_hash: Option<String>,
}

/// Intake items whose content hash never materialised into a media asset.
const UNTRACKED_ORIGINALS_STATEMENT: &str =
    "SELECT item_id, record::id(batch_id) AS batch_id, source_path, file_name, content_hash \
     FROM atelier_intake_item \
     WHERE content_hash = NONE \
        OR (SELECT VALUE id FROM atelier_media_asset \
            WHERE content_hash = string::replace(string::lowercase($parent.content_hash ?? ''), 'sha256:', '')) = [];";

#[derive(SurrealValue)]
struct TrackedPayloadBindings {
    artifact_ref: String,
    content_hash: String,
    artifact_manifest_ref: String,
}

const PAYLOAD_TRACKED_STATEMENT: &str =
    "RETURN count(SELECT id FROM atelier_media_asset \
              WHERE artifact_ref = $artifact_ref \
                 OR string::lowercase(string::replace(content_hash, 'sha256:', '')) = $content_hash) > 0 \
        OR count(SELECT id FROM atelier_intake_item \
              WHERE source_path = $artifact_ref \
                 OR string::lowercase(string::replace(content_hash ?? '', 'sha256:', '')) = $content_hash) > 0 \
        OR count(SELECT id FROM atelier_media_derivative \
              WHERE artifact_ref = $artifact_ref \
                 OR artifact_manifest_ref = $artifact_manifest_ref) > 0;";

#[derive(SurrealValue)]
struct SidecarAnomalyRow {
    sidecar_id: SurrealUuid,
    parent_asset_id: SurrealUuid,
    sidecar_asset_id: SurrealUuid,
    relation_kind: String,
    hidden_from_gallery: bool,
    searchable_by_relation: bool,
}

const SIDECAR_ANOMALIES_STATEMENT: &str =
    "SELECT sidecar_id, record::id(parent_asset_id) AS parent_asset_id, \
            record::id(sidecar_asset_id) AS sidecar_asset_id, relation_kind, \
            hidden_from_gallery, searchable_by_relation \
     FROM atelier_media_sidecar \
     WHERE hidden_from_gallery != true OR searchable_by_relation != true;";

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
        let sidecars_checked: i64 = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(COUNT_SIDECARS_STATEMENT, NoBindings {})
                        .await
                })
            })
            .await?
            .unwrap_or_default();

        let summary = filesystem_health_summary(&pending, sidecars_checked);
        let check_id = Uuid::now_v7();
        let finding_inserts: Vec<HealthFindingInsert> = pending
            .iter()
            .map(|finding| HealthFindingInsert {
                finding_id: SurrealUuid::from(Uuid::now_v7()),
                finding_kind: finding.finding_kind.as_token().to_owned(),
                target_type: finding.target_type.to_owned(),
                target_id: finding.target_id.clone(),
                details: finding.details.clone(),
            })
            .collect();
        let bindings = RecordHealthCheckBindings {
            check_rid: RecordId::new(
                "atelier_filesystem_health_check",
                SurrealUuid::from(check_id),
            ),
            check_id: SurrealUuid::from(check_id),
            requested_by: requested_by.to_owned(),
            scope_label: scope_label.clone(),
            summary: summary.clone(),
            findings: finding_inserts,
        };
        let outcome: Option<RecordHealthCheckOutcome> = self
            .write_with_event(
                RECORD_HEALTH_CHECK_STATEMENT,
                bindings,
                filesystem_health_event_family::CHECK_RECORDED,
                "atelier_filesystem_health_check",
                &check_id.to_string(),
                serde_json::json!({
                    "check_id": check_id,
                    "requested_by": requested_by,
                    "scope_label": scope_label,
                    "summary": summary,
                    "finding_count": pending.len(),
                }),
            )
            .await?;
        let outcome = outcome.ok_or_else(|| {
            AtelierError::Internal(
                "recording a filesystem health check returned no outcome".to_owned(),
            )
        })?;
        let check: FilesystemHealthCheck = outcome
            .check
            .into_iter()
            .next()
            .ok_or_else(|| {
                AtelierError::Internal(
                    "recording a filesystem health check returned no check row".to_owned(),
                )
            })?
            .into();
        let findings = outcome
            .findings
            .into_iter()
            .map(FilesystemHealthFinding::try_from)
            .collect::<AtelierResult<Vec<_>>>()?;
        Ok(FilesystemHealthReport { check, findings })
    }

    pub async fn list_filesystem_health_findings(
        &self,
        check_id: Uuid,
    ) -> AtelierResult<Vec<FilesystemHealthFinding>> {
        let bindings = CheckRefBinding {
            check_ref: RecordId::new(
                "atelier_filesystem_health_check",
                SurrealUuid::from(check_id),
            ),
        };
        let rows: Vec<HealthFindingRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_HEALTH_FINDINGS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(FilesystemHealthFinding::try_from)
            .collect()
    }
}

async fn collect_missing_originals(
    store: &AtelierStore,
    findings: &mut Vec<PendingFilesystemHealthFinding>,
) -> AtelierResult<()> {
    let rows: Vec<MissingOriginalRow> = store
        .store()
        .with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_values(MISSING_ORIGINALS_STATEMENT, NoBindings {})
                    .await
            })
        })
        .await?;
    for row in rows {
        let asset_id: Uuid = row.asset_id.into();
        let issue = row
            .validation_state
            .as_deref()
            .filter(|state| *state == "invalid_legacy_artifact_ref")
            .map(|state| format!("artifact_manifest validation_state={state}"))
            .or_else(|| {
                artifact_payload_health_issue(
                    &row.artifact_ref,
                    Some(&row.content_hash),
                    Some(row.byte_len),
                    Some(&row.mime),
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
                "content_hash": row.content_hash,
                "artifact_ref": row.artifact_ref,
                "validation_state": row.validation_state,
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
    let rows: Vec<AssetHashRow> = store
        .store()
        .with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_values(ASSETS_MISSING_THUMBNAIL_STATEMENT, NoBindings {})
                    .await
            })
        })
        .await?;
    for row in rows {
        let asset_id: Uuid = row.asset_id.into();
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::MissingThumbnail,
            target_type: "atelier_media_asset",
            target_id: asset_id.to_string(),
            details: serde_json::json!({
                "asset_id": asset_id,
                "content_hash": row.content_hash,
                "required_derivative_kind": "thumbnail",
            }),
        });
    }
    let rows: Vec<GeneratedThumbnailRow> = store
        .store()
        .with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_values(GENERATED_THUMBNAILS_STATEMENT, NoBindings {})
                    .await
            })
        })
        .await?;
    for row in rows {
        let Some(artifact_ref) = row.artifact_ref.clone() else {
            continue;
        };
        let issue = artifact_payload_health_issue(
            &artifact_ref,
            None,
            row.byte_len,
            row.mime.as_deref(),
            row.artifact_manifest_ref.as_deref(),
        );
        let Some(issue) = issue else {
            continue;
        };
        let derivative_id: Uuid = row.derivative_id.into();
        let asset_id: Uuid = row.asset_id.into();
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::MissingThumbnail,
            target_type: "atelier_media_derivative",
            target_id: derivative_id.to_string(),
            details: serde_json::json!({
                "derivative_id": derivative_id,
                "asset_id": asset_id,
                "content_hash": row.content_hash,
                "derivative_kind": row.derivative_kind,
                "artifact_ref": artifact_ref,
                "artifact_manifest_ref": row.artifact_manifest_ref,
                "mime": row.mime,
                "byte_len": row.byte_len,
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
    let rows: Vec<IntakeItemRow> = store
        .store()
        .with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_values(INBOX_PENDING_STATEMENT, NoBindings {})
                    .await
            })
        })
        .await?;
    for row in rows {
        let item_id: Uuid = row.item_id.into();
        let batch_id: Uuid = row.batch_id.into();
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::InboxPending,
            target_type: "atelier_intake_item",
            target_id: item_id.to_string(),
            details: serde_json::json!({
                "item_id": item_id,
                "batch_id": batch_id,
                "source_path": row.source_path,
                "file_name": row.file_name,
                "lane": row.lane,
            }),
        });
    }
    Ok(())
}

async fn collect_untracked_originals(
    store: &AtelierStore,
    findings: &mut Vec<PendingFilesystemHealthFinding>,
) -> AtelierResult<()> {
    let rows: Vec<UntrackedIntakeItemRow> = store
        .store()
        .with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_values(UNTRACKED_ORIGINALS_STATEMENT, NoBindings {})
                    .await
            })
        })
        .await?;
    for row in rows {
        let item_id: Uuid = row.item_id.into();
        let batch_id: Uuid = row.batch_id.into();
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::UntrackedOriginal,
            target_type: "atelier_intake_item",
            target_id: item_id.to_string(),
            details: serde_json::json!({
                "item_id": item_id,
                "batch_id": batch_id,
                "source_path": row.source_path,
                "file_name": row.file_name,
                "content_hash": row.content_hash,
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
            let bindings = TrackedPayloadBindings {
                artifact_ref: artifact_ref.clone(),
                content_hash: content_hash.clone(),
                artifact_manifest_ref,
            };
            let tracked: Option<bool> = store
                .store()
                .with_data_operation(move |ctx| {
                    Box::pin(
                        async move { ctx.query_first(PAYLOAD_TRACKED_STATEMENT, bindings).await },
                    )
                })
                .await?;
            if tracked.unwrap_or(false) {
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
    let rows: Vec<SidecarAnomalyRow> = store
        .store()
        .with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_values(SIDECAR_ANOMALIES_STATEMENT, NoBindings {})
                    .await
            })
        })
        .await?;
    for row in rows {
        let sidecar_id: Uuid = row.sidecar_id.into();
        let parent_asset_id: Uuid = row.parent_asset_id.into();
        let sidecar_asset_id: Uuid = row.sidecar_asset_id.into();
        findings.push(PendingFilesystemHealthFinding {
            finding_kind: FilesystemHealthFindingKind::SidecarVisibilityAnomaly,
            target_type: "atelier_media_sidecar",
            target_id: sidecar_id.to_string(),
            details: serde_json::json!({
                "sidecar_id": sidecar_id,
                "parent_asset_id": parent_asset_id,
                "sidecar_asset_id": sidecar_asset_id,
                "relation_kind": row.relation_kind,
                "hidden_from_gallery": row.hidden_from_gallery,
                "searchable_by_relation": row.searchable_by_relation,
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

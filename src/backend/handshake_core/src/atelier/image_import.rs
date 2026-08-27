//! Clipboard and URL image import (MT-025).
//!
//! Clipboard import only accepts bytes already captured into ArtifactStore and
//! materializes them through the media store. URL import records a governed
//! fetch request after SSRF and media-downloader capability preflight; actual
//! network fetch remains a Workflow-Engine responsibility.

use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::downloader::{
    validate_media_downloader_capability_grant, MEDIA_DOWNLOADER_BATCH_PROTOCOL_ID,
};
use super::{
    atelier_event_sql, event_family, event_ref_for_text, reject_legacy_runtime_ref,
    uuid_from_record_link, AtelierError, AtelierResult, AtelierStore, NewMediaAsset,
};

const URL_IMPORT_REDIRECT_POLICY: &str = "disabled_until_revalidated";

#[derive(Clone, Debug)]
pub struct ClipboardImageImportRequest {
    pub idempotency_key: String,
    pub mime: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub artifact_ref: String,
    pub source_application: Option<String>,
    pub requested_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UrlImageImportRequest {
    pub idempotency_key: String,
    pub source_url: String,
    pub expected_mime: Option<String>,
    pub source_label: Option<String>,
    pub capability_profile_id: String,
    pub capability_grant_ref: String,
    pub requested_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageImportRecord {
    pub import_id: Uuid,
    pub idempotency_key: String,
    pub source_kind: String,
    pub status: String,
    pub requested_by: String,
    pub normalized_url: Option<String>,
    pub source_url_hash: String,
    pub source_host: Option<String>,
    pub source_label: Option<String>,
    pub expected_mime: Option<String>,
    pub capability_profile_id: Option<String>,
    pub capability_grant_ref: Option<String>,
    pub required_capabilities: serde_json::Value,
    pub asset_id: Option<Uuid>,
    pub artifact_ref: Option<String>,
    pub source_provenance: String,
    pub preflight: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

struct UrlPreflight {
    normalized_url: String,
    source_host: String,
    source_url_hash: String,
}

fn canonical_sha256_content_hash(content_hash: &str) -> AtelierResult<String> {
    let hash = content_hash.trim();
    if hash != content_hash {
        return Err(AtelierError::Validation(
            "content_hash must not be padded".into(),
        ));
    }
    let hex = hash.strip_prefix("sha256:").unwrap_or(hash);
    if hex.len() != 64 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(AtelierError::Validation(
            "content_hash must be sha256:<64 hex> or bare 64 hex".into(),
        ));
    }
    Ok(hex.to_ascii_lowercase())
}

/// One `atelier_image_import_request` row as the store returns it. The asset
/// link stays raw so a missing link maps cleanly to `None`.
#[derive(SurrealValue)]
struct ImageImportRow {
    import_id: SurrealUuid,
    idempotency_key: String,
    source_kind: String,
    status: String,
    requested_by: String,
    normalized_url: Option<String>,
    source_url_hash: Option<String>,
    source_host: Option<String>,
    source_label: Option<String>,
    expected_mime: Option<String>,
    capability_profile_id: Option<String>,
    capability_grant_ref: Option<String>,
    required_capabilities: serde_json::Value,
    asset_id: Option<RecordId>,
    artifact_ref: Option<String>,
    source_provenance: String,
    preflight: serde_json::Value,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl TryFrom<ImageImportRow> for ImageImportRecord {
    type Error = AtelierError;

    fn try_from(row: ImageImportRow) -> AtelierResult<Self> {
        let asset_id = row
            .asset_id
            .as_ref()
            .map(|link| uuid_from_record_link("asset_id", link))
            .transpose()?;
        Ok(ImageImportRecord {
            import_id: row.import_id.into(),
            idempotency_key: row.idempotency_key,
            source_kind: row.source_kind,
            status: row.status,
            requested_by: row.requested_by,
            normalized_url: row.normalized_url,
            source_url_hash: row.source_url_hash.unwrap_or_default(),
            source_host: row.source_host,
            source_label: row.source_label,
            expected_mime: row.expected_mime,
            capability_profile_id: row.capability_profile_id,
            capability_grant_ref: row.capability_grant_ref,
            required_capabilities: row.required_capabilities,
            asset_id,
            artifact_ref: row.artifact_ref,
            source_provenance: row.source_provenance,
            preflight: row.preflight,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        })
    }
}

fn require_non_empty<'a>(field: &str, value: &'a str) -> AtelierResult<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(trimmed)
}

fn validate_image_mime(field: &str, mime: &str) -> AtelierResult<()> {
    let mime = require_non_empty(field, mime)?;
    match mime {
        "image/png" | "image/jpeg" | "image/webp" => Ok(()),
        other => Err(AtelierError::Validation(format!(
            "{field} must be image/png, image/jpeg, or image/webp, got {other:?}"
        ))),
    }
}

fn source_application_token(value: Option<&str>) -> AtelierResult<String> {
    let token = value.unwrap_or("unknown");
    let token = require_non_empty("source_application", token)?;
    reject_legacy_runtime_ref("source_application", token)?;
    Ok(token.to_string())
}

fn validate_optional_label(field: &str, value: &Option<String>) -> AtelierResult<()> {
    if let Some(value) = value {
        require_non_empty(field, value)?;
        reject_legacy_runtime_ref(field, value)?;
    }
    Ok(())
}

fn url_hash(normalized_url: &str) -> String {
    format!(
        "sha256:{}",
        hex::encode(Sha256::digest(normalized_url.as_bytes()))
    )
}

fn ipv4_component_is_numeric_like(component: &str) -> bool {
    let Some(hex) = component
        .strip_prefix("0x")
        .or_else(|| component.strip_prefix("0X"))
    else {
        return !component.is_empty() && component.chars().all(|ch| ch.is_ascii_digit());
    };
    !hex.is_empty() && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn ipv4_component_has_legacy_marker(component: &str) -> bool {
    component.starts_with("0x")
        || component.starts_with("0X")
        || (component.len() > 1 && component.starts_with('0'))
}

fn ambiguous_ipv4_numeric_literal(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.is_empty()
        || parts.len() > 4
        || !parts
            .iter()
            .all(|component| ipv4_component_is_numeric_like(component))
    {
        return false;
    }
    parts.len() != 4
        || parts
            .iter()
            .any(|component| ipv4_component_has_legacy_marker(component))
        || host.parse::<Ipv4Addr>().is_err()
}

fn redacted_url_for_storage(url: &Url) -> String {
    let mut display_url = url.clone();
    display_url.set_path("/path-redacted");
    display_url.set_query(None);
    display_url.set_fragment(None);
    display_url.as_str().to_string()
}

fn ip_is_blocked(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(addr) => ipv4_is_blocked(addr),
        IpAddr::V6(addr) => ipv6_is_blocked(addr),
    }
}

fn ipv4_is_blocked(addr: Ipv4Addr) -> bool {
    addr.is_private()
        || addr.is_loopback()
        || addr.is_link_local()
        || addr.is_unspecified()
        || addr.is_broadcast()
        || addr.is_multicast()
}

fn ipv6_is_blocked(addr: Ipv6Addr) -> bool {
    if let Some(mapped) = addr.to_ipv4_mapped() {
        return ipv4_is_blocked(mapped);
    }
    addr.is_loopback()
        || addr.is_unspecified()
        || addr.is_unique_local()
        || addr.is_unicast_link_local()
        || addr.is_multicast()
}

fn preflight_url(raw: &str) -> AtelierResult<UrlPreflight> {
    let raw = require_non_empty("source_url", raw)?;
    let url = Url::parse(raw)
        .map_err(|err| AtelierError::Validation(format!("source_url parse error: {err}")))?;
    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(AtelierError::Validation(
                "source_url must use http or https".to_string(),
            ));
        }
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AtelierError::Validation(
            "source_url credentials are forbidden".to_string(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| AtelierError::Validation("source_url missing host".to_string()))?;
    let source_host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    let host_for_ip = source_host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(source_host.as_str());
    if source_host.is_empty()
        || host_for_ip == "localhost"
        || host_for_ip.ends_with(".localhost")
        || host_for_ip.ends_with(".local")
    {
        return Err(AtelierError::Validation(
            "source_url blocked by SSRF preflight".to_string(),
        ));
    }
    if ambiguous_ipv4_numeric_literal(host_for_ip) {
        return Err(AtelierError::Validation(
            "source_url blocked by SSRF preflight".to_string(),
        ));
    }
    if let Ok(ip) = host_for_ip.parse::<IpAddr>() {
        if ip_is_blocked(ip) {
            return Err(AtelierError::Validation(
                "source_url blocked by SSRF preflight".to_string(),
            ));
        }
    }

    let mut fetch_url = url.clone();
    fetch_url.set_fragment(None);
    let fetch_url = fetch_url.as_str().to_string();
    let normalized_url = redacted_url_for_storage(&url);
    Ok(UrlPreflight {
        source_url_hash: url_hash(&fetch_url),
        normalized_url,
        source_host,
    })
}

fn idempotency_mismatch() -> AtelierError {
    AtelierError::Validation(
        "idempotency_key is already bound to a different image import request".to_string(),
    )
}

fn require_matching_url_replay(
    existing: &ImageImportRecord,
    input: &UrlImageImportRequest,
    preflight: &UrlPreflight,
    required_capabilities_json: &serde_json::Value,
    source_provenance: &str,
) -> AtelierResult<()> {
    if existing.source_kind != "url" {
        return Err(AtelierError::Validation(
            "idempotency_key is already bound to a different image import source".to_string(),
        ));
    }
    let matches = existing.status == "queued"
        && existing.requested_by == input.requested_by
        && existing.normalized_url.as_deref() == Some(preflight.normalized_url.as_str())
        && existing.source_url_hash == preflight.source_url_hash
        && existing.source_host.as_deref() == Some(preflight.source_host.as_str())
        && existing.source_label == input.source_label
        && existing.expected_mime == input.expected_mime
        && existing.capability_profile_id.as_deref() == Some(input.capability_profile_id.as_str())
        && existing.capability_grant_ref.as_deref() == Some(input.capability_grant_ref.as_str())
        && existing.required_capabilities == *required_capabilities_json
        && existing.source_provenance == source_provenance
        && existing.asset_id.is_none()
        && existing.artifact_ref.is_none();
    if matches {
        Ok(())
    } else {
        Err(idempotency_mismatch())
    }
}

const IMAGE_IMPORT_BY_KEY_STATEMENT: &str = concat!(
    "SELECT ",
    "import_id, idempotency_key, source_kind, status, requested_by, normalized_url, \
     source_url_hash, source_host, source_label, expected_mime, capability_profile_id, \
     capability_grant_ref, required_capabilities, asset_id, artifact_ref, \
     source_provenance, preflight, created_at_utc, updated_at_utc",
    " FROM atelier_image_import_request WHERE idempotency_key = $idempotency_key LIMIT 1;"
);

#[derive(SurrealValue)]
struct IdempotencyKeyBinding {
    idempotency_key: String,
}

#[derive(SurrealValue)]
struct MediaAssetIdBinding {
    asset_id: SurrealUuid,
}

/// The media-asset fields the clipboard replay check compares.
#[derive(SurrealValue)]
struct MediaAssetReplayRow {
    content_hash: String,
    mime: String,
    byte_len: i64,
}

const MEDIA_ASSET_REPLAY_STATEMENT: &str =
    "SELECT content_hash, mime, byte_len FROM atelier_media_asset \
     WHERE asset_id = $asset_id LIMIT 1;";

#[derive(Clone, SurrealValue)]
struct CreateClipboardImportBindings {
    record_id: RecordId,
    import_id: SurrealUuid,
    idempotency_key: String,
    requested_by: String,
    asset_ref: RecordId,
    artifact_ref: String,
    source_provenance: String,
    preflight: serde_json::Value,
}

const CREATE_CLIPBOARD_IMPORT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.record_id; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         import_id: $domain.import_id, \
         idempotency_key: $domain.idempotency_key, \
         source_kind: 'clipboard', \
         status: 'materialized', \
         requested_by: $domain.requested_by, \
         required_capabilities: [], \
         asset_id: $domain.asset_ref, \
         artifact_ref: $domain.artifact_ref, \
         source_provenance: $domain.source_provenance, \
         preflight: $domain.preflight \
       }; \
       RETURN (SELECT ",
    "import_id, idempotency_key, source_kind, status, requested_by, normalized_url, \
     source_url_hash, source_host, source_label, expected_mime, capability_profile_id, \
     capability_grant_ref, required_capabilities, asset_id, artifact_ref, \
     source_provenance, preflight, created_at_utc, updated_at_utc",
    " FROM $rid); };"
);

#[derive(Clone, SurrealValue)]
struct CreateUrlImportBindings {
    record_id: RecordId,
    import_id: SurrealUuid,
    idempotency_key: String,
    requested_by: String,
    normalized_url: String,
    source_url_hash: String,
    source_host: String,
    source_label: Option<String>,
    expected_mime: Option<String>,
    capability_profile_id: String,
    capability_grant_ref: String,
    required_capabilities: serde_json::Value,
    source_provenance: String,
    preflight: serde_json::Value,
}

const CREATE_URL_IMPORT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.record_id; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         import_id: $domain.import_id, \
         idempotency_key: $domain.idempotency_key, \
         source_kind: 'url', \
         status: 'queued', \
         requested_by: $domain.requested_by, \
         normalized_url: $domain.normalized_url, \
         source_url_hash: $domain.source_url_hash, \
         source_host: $domain.source_host, \
         source_label: $domain.source_label, \
         expected_mime: $domain.expected_mime, \
         capability_profile_id: $domain.capability_profile_id, \
         capability_grant_ref: $domain.capability_grant_ref, \
         required_capabilities: $domain.required_capabilities, \
         source_provenance: $domain.source_provenance, \
         preflight: $domain.preflight \
       }; \
       RETURN (SELECT ",
    "import_id, idempotency_key, source_kind, status, requested_by, normalized_url, \
     source_url_hash, source_host, source_label, expected_mime, capability_profile_id, \
     capability_grant_ref, required_capabilities, asset_id, artifact_ref, \
     source_provenance, preflight, created_at_utc, updated_at_utc",
    " FROM $rid); };"
);

fn is_idempotency_key_conflict(error: &AtelierError) -> bool {
    let text = error.to_string();
    text.contains("uq_atelier_image_import_request_1") || text.contains("already contains")
}

impl AtelierStore {
    async fn image_import_by_key(
        &self,
        idempotency_key: &str,
    ) -> AtelierResult<Option<ImageImportRecord>> {
        let bindings = IdempotencyKeyBinding {
            idempotency_key: idempotency_key.to_owned(),
        };
        let row: Option<ImageImportRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(IMAGE_IMPORT_BY_KEY_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        row.map(ImageImportRecord::try_from).transpose()
    }

    async fn require_matching_clipboard_replay(
        &self,
        existing: &ImageImportRecord,
        input: &ClipboardImageImportRequest,
        content_hash: &str,
        source_provenance: &str,
    ) -> AtelierResult<()> {
        if existing.source_kind != "clipboard" {
            return Err(AtelierError::Validation(
                "idempotency_key is already bound to a different image import source".to_string(),
            ));
        }
        let Some(asset_id) = existing.asset_id else {
            return Err(idempotency_mismatch());
        };
        let bindings = MediaAssetIdBinding {
            asset_id: SurrealUuid::from(asset_id),
        };
        let asset: Option<MediaAssetReplayRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(MEDIA_ASSET_REPLAY_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        let asset =
            asset.ok_or_else(|| AtelierError::NotFound(format!("media asset_id={asset_id}")))?;
        let matches = existing.status == "materialized"
            && existing.requested_by == input.requested_by
            && existing.normalized_url.is_none()
            && existing.source_url_hash.is_empty()
            && existing.source_host.is_none()
            && existing.source_label.is_none()
            && existing.expected_mime.is_none()
            && existing.capability_profile_id.is_none()
            && existing.capability_grant_ref.is_none()
            && existing.required_capabilities == serde_json::json!([])
            && existing.artifact_ref.as_deref() == Some(input.artifact_ref.as_str())
            && existing.source_provenance == source_provenance
            && asset.content_hash == content_hash
            && asset.mime == input.mime
            && asset.byte_len == input.byte_len;
        if matches {
            Ok(())
        } else {
            Err(idempotency_mismatch())
        }
    }

    pub async fn import_clipboard_image(
        &self,
        input: &ClipboardImageImportRequest,
    ) -> AtelierResult<ImageImportRecord> {
        require_non_empty("idempotency_key", &input.idempotency_key)?;
        require_non_empty("requested_by", &input.requested_by)?;
        validate_image_mime("mime", &input.mime)?;
        let content_hash = canonical_sha256_content_hash(&input.content_hash)?;
        let source_application = source_application_token(input.source_application.as_deref())?;
        let source_provenance = format!("clipboard:{source_application}");

        if let Some(existing) = self.image_import_by_key(&input.idempotency_key).await? {
            self.require_matching_clipboard_replay(
                &existing,
                input,
                &content_hash,
                &source_provenance,
            )
            .await?;
            return Ok(existing);
        }

        let asset = self
            .materialize_media_asset(&NewMediaAsset {
                content_hash: content_hash.clone(),
                mime: input.mime.clone(),
                byte_len: input.byte_len,
                source_provenance: Some(source_provenance.clone()),
                artifact_ref: input.artifact_ref.clone(),
            })
            .await?;

        let preflight = serde_json::json!({
            "source_application": source_application,
            "artifact_store_binding_verified": true,
        });
        let import_id = Uuid::now_v7();
        let bindings = CreateClipboardImportBindings {
            record_id: RecordId::new("atelier_image_import_request", SurrealUuid::from(import_id)),
            import_id: SurrealUuid::from(import_id),
            idempotency_key: input.idempotency_key.clone(),
            requested_by: input.requested_by.clone(),
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset.asset_id)),
            artifact_ref: input.artifact_ref.clone(),
            source_provenance: source_provenance.clone(),
            preflight: preflight.clone(),
        };
        let written: AtelierResult<Option<ImageImportRow>> = self
            .write_with_event(
                CREATE_CLIPBOARD_IMPORT_STATEMENT,
                bindings,
                event_family::IMAGE_IMPORT_RECORDED,
                "atelier_image_import_request",
                &import_id.to_string(),
                serde_json::json!({
                    "import_id": import_id,
                    "source_kind": "clipboard",
                    "status": "materialized",
                    "requested_by": input.requested_by,
                    "asset_id": asset.asset_id,
                    "artifact_ref": input.artifact_ref,
                    "source_provenance": source_provenance,
                    "source_application": source_application,
                }),
            )
            .await;
        match written {
            Ok(Some(row)) => row.try_into(),
            Ok(None) => Err(AtelierError::Internal(
                "recording a clipboard image import returned no row".to_owned(),
            )),
            Err(error) if is_idempotency_key_conflict(&error) => {
                // A concurrent writer claimed the key first; validate against
                // its record exactly as the replay path does.
                let existing = self
                    .image_import_by_key(&input.idempotency_key)
                    .await?
                    .ok_or(error)?;
                self.require_matching_clipboard_replay(
                    &existing,
                    input,
                    &content_hash,
                    &source_provenance,
                )
                .await?;
                Ok(existing)
            }
            Err(error) => Err(error),
        }
    }

    pub async fn record_url_image_import(
        &self,
        input: &UrlImageImportRequest,
    ) -> AtelierResult<ImageImportRecord> {
        require_non_empty("idempotency_key", &input.idempotency_key)?;
        require_non_empty("requested_by", &input.requested_by)?;
        require_non_empty("capability_profile_id", &input.capability_profile_id)?;
        require_non_empty("capability_grant_ref", &input.capability_grant_ref)?;
        validate_optional_label("source_label", &input.source_label)?;
        if let Some(mime) = &input.expected_mime {
            validate_image_mime("expected_mime", mime)?;
        }

        let preflight = preflight_url(&input.source_url)?;
        let required_capabilities = validate_media_downloader_capability_grant(
            MEDIA_DOWNLOADER_BATCH_PROTOCOL_ID,
            &input.capability_profile_id,
            &input.capability_grant_ref,
        )?;
        let required_capabilities_json = serde_json::json!(required_capabilities);
        let source_provenance = format!("url:{}", preflight.source_url_hash);
        let preflight_json = serde_json::json!({
            "network_fetch_allowed": false,
            "requires_fetch_worker_revalidation": true,
            "redirect_policy": URL_IMPORT_REDIRECT_POLICY,
            "ssrf_preflight": "passed",
        });

        if let Some(existing) = self.image_import_by_key(&input.idempotency_key).await? {
            require_matching_url_replay(
                &existing,
                input,
                &preflight,
                &required_capabilities_json,
                &source_provenance,
            )?;
            return Ok(existing);
        }

        let import_id = Uuid::now_v7();
        let bindings = CreateUrlImportBindings {
            record_id: RecordId::new("atelier_image_import_request", SurrealUuid::from(import_id)),
            import_id: SurrealUuid::from(import_id),
            idempotency_key: input.idempotency_key.clone(),
            requested_by: input.requested_by.clone(),
            normalized_url: preflight.normalized_url.clone(),
            source_url_hash: preflight.source_url_hash.clone(),
            source_host: preflight.source_host.clone(),
            source_label: input.source_label.clone(),
            expected_mime: input.expected_mime.clone(),
            capability_profile_id: input.capability_profile_id.clone(),
            capability_grant_ref: input.capability_grant_ref.clone(),
            required_capabilities: required_capabilities_json.clone(),
            source_provenance: source_provenance.clone(),
            preflight: preflight_json,
        };
        let written: AtelierResult<Option<ImageImportRow>> = self
            .write_with_event(
                CREATE_URL_IMPORT_STATEMENT,
                bindings,
                event_family::IMAGE_IMPORT_RECORDED,
                "atelier_image_import_request",
                &import_id.to_string(),
                serde_json::json!({
                    "import_id": import_id,
                    "source_kind": "url",
                    "status": "queued",
                    "requested_by": input.requested_by,
                    "source_url_ref": preflight.source_url_hash,
                    "source_host": preflight.source_host,
                    "source_label_ref": input.source_label.as_ref().map(|value| event_ref_for_text(value)),
                    "expected_mime": input.expected_mime,
                    "capability_profile_id": input.capability_profile_id,
                    "capability_grant_ref_ref": event_ref_for_text(&input.capability_grant_ref),
                    "required_capabilities": required_capabilities_json,
                    "network_fetch_allowed": false,
                    "requires_fetch_worker_revalidation": true,
                    "redirect_policy": URL_IMPORT_REDIRECT_POLICY,
                }),
            )
            .await;
        match written {
            Ok(Some(row)) => row.try_into(),
            Ok(None) => Err(AtelierError::Internal(
                "recording a url image import returned no row".to_owned(),
            )),
            Err(error) if is_idempotency_key_conflict(&error) => {
                let existing = self
                    .image_import_by_key(&input.idempotency_key)
                    .await?
                    .ok_or(error)?;
                require_matching_url_replay(
                    &existing,
                    input,
                    &preflight,
                    &required_capabilities_json,
                    &source_provenance,
                )?;
                Ok(existing)
            }
            Err(error) => Err(error),
        }
    }
}

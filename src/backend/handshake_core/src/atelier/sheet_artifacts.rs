//! Reusable artifact links attached to append-only CKC sheet versions.
//!
//! MT-016 keeps OpenPose and ComfyUI outputs as typed refs linked to a sheet
//! version, instead of copying artifact paths into raw character-sheet text.
//!
//! SurrealDB port (WP-CKC-posekit-overhaul). Table `atelier_sheet_artifact_link`
//! in `storage/surreal/schema.surql`:
//!
//! * the PostgreSQL partial UNIQUE over active links
//!   (`WHERE detached_at_utc IS NULL`) is the stored discriminator
//!   `active_link_key` (`active:<version>|<kind>|<ref>` while attached,
//!   `detached:<link_id>` afterwards) under a plain UNIQUE index;
//! * the composite FK `(character_internal_id, sheet_version_id)` is the
//!   `sheet_version_id` ASSERT that the version belongs to `$this.character_internal_id`;
//! * the SQL function `atelier_is_native_portable_ref()` is
//!   [`validate_native_portable_ref`] below, evaluated before any statement;
//! * detach never deletes: it stamps `detached_at_utc` + `detached_by` and the
//!   row leaves the active set.
//!
//! Schema deviation from the reference branch, coded against here: the schema
//! requires `artifact_ref` AND `manifest_ref` to start with `artifact://`
//! (PostgreSQL only required a native portable ref), so both are checked in
//! Rust to surface a typed validation error instead of a store error.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::refs::{sheet_artifact_ref, sheet_version_ref};
use super::{atelier_event_sql, AtelierError, AtelierResult, AtelierStore};

/// Event families owned by this module (pattern: `collections_event_family`);
/// the module owner appends them to `event_family::ALL`.
pub mod sheet_artifact_event_family {
    /// A reusable artifact ref was attached to a sheet version (MT-016).
    pub const SHEET_ARTIFACT_LINKED: &str = "atelier.sheet.artifact_linked";
    /// An attached artifact ref was soft-detached (MT-016).
    pub const SHEET_ARTIFACT_DETACHED: &str = "atelier.sheet.artifact_detached";

    pub const ALL: &[&str] = &[SHEET_ARTIFACT_LINKED, SHEET_ARTIFACT_DETACHED];
}

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

fn authority_host(lower: &str) -> Option<&str> {
    let (scheme, rest) = lower.split_once("://")?;
    let mut scheme_chars = scheme.chars();
    let scheme_ok = scheme_chars
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase())
        && scheme_chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '+' | '.' | '-'));
    if !scheme_ok {
        return None;
    }
    let authority = rest
        .split(|ch| matches!(ch, '/' | '?' | '#'))
        .next()
        .unwrap_or_default();
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    if let Some(bracketed) = host_port.strip_prefix('[') {
        return bracketed.split_once(']').map(|(host, _)| host);
    }
    host_port.split(':').next()
}

/// Rust port of the reference migration's `atelier_is_native_portable_ref()`
/// SQL predicate (0340), applied to every ref column before the statement runs.
///
/// Rejects: padding or empty, any whitespace, backslashes, `.gov` anywhere,
/// SQLite refs (`sqlite:` scheme, `.sqlite`/`.sqlite3`/`.db` suffixes or path
/// segments, probed before any `?`/`#` suffix), drive letters (`c:` at the start
/// or after a `/`), `file:` schemes, `//` and `/` and `~/` prefixes,
/// `%userprofile%`, `..` segments, `electron:`/`/electron/`, the tokens `ckc`,
/// `castkit`, `electron` between separators, direct-LLM schemes
/// (`llm|openai|anthropic|ollama|model-server|model_server`), loopback or
/// unspecified authorities (`localhost`, `127.*`, `0.0.0.0`, `::1`), bare
/// loopback prefixes, and `//localhost/`.
///
/// Empty/padded/whitespace failures are [`AtelierError::Validation`] (the
/// caller sent a malformed value); every other match is
/// [`AtelierError::ForbiddenStorage`] (the caller pointed at storage Handshake
/// refuses to reference), mirroring [`super::reject_legacy_runtime_ref`].
pub fn validate_native_portable_ref(field: &str, candidate: &str) -> AtelierResult<()> {
    if candidate.is_empty() || candidate.trim() != candidate {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    if candidate.chars().any(char::is_whitespace) {
        return Err(AtelierError::Validation(format!(
            "{field} must not contain whitespace"
        )));
    }
    let forbidden = |reason: &str| {
        Err(AtelierError::ForbiddenStorage(format!(
            "{field} must be a Handshake-native portable ref ({reason}), not SQLite/Electron/CKC/CastKit/localhost/direct-LLM/.GOV/machine-local storage"
        )))
    };
    if candidate.contains('\\') {
        return forbidden("backslash");
    }
    let v = candidate.to_ascii_lowercase().replace('\\', "/");
    if v.contains(".gov") {
        return forbidden(".gov");
    }
    let sqlite_probe = v
        .split(|ch| matches!(ch, '?' | '#'))
        .next()
        .unwrap_or(v.as_str());
    if v.starts_with("sqlite:")
        || sqlite_probe.ends_with(".sqlite")
        || sqlite_probe.contains(".sqlite/")
        || sqlite_probe.ends_with(".sqlite3")
        || sqlite_probe.contains(".sqlite3/")
        || sqlite_probe.ends_with(".db")
        || sqlite_probe.contains(".db/")
    {
        return forbidden("sqlite");
    }
    let bytes = v.as_bytes();
    let has_drive_prefix = bytes.len() >= 2 && bytes[0].is_ascii_lowercase() && bytes[1] == b':';
    let has_embedded_drive = bytes
        .windows(3)
        .any(|window| window[0] == b'/' && window[1].is_ascii_lowercase() && window[2] == b':');
    if has_drive_prefix || has_embedded_drive {
        return forbidden("drive letter");
    }
    if v.starts_with("file:")
        || v.contains("file://")
        || v.starts_with("//")
        || v.starts_with('/')
        || v.starts_with("~/")
        || v.contains("%userprofile%")
    {
        return forbidden("machine-local path");
    }
    if v.starts_with("../") || v.contains("/../") || v.ends_with("/..") {
        return forbidden("parent-directory segment");
    }
    if v.starts_with("electron:") || v.contains("/electron/") {
        return forbidden("electron");
    }
    if v
        .split(|ch| matches!(ch, '/' | ':' | '.' | '?' | '#' | '&' | '=' | '@'))
        .any(|segment| matches!(segment, "ckc" | "castkit" | "electron"))
    {
        return forbidden("legacy runtime token");
    }
    if ["llm:", "openai:", "anthropic:", "ollama:", "model-server:", "model_server:"]
        .iter()
        .any(|scheme| v.starts_with(scheme))
    {
        return forbidden("direct-LLM scheme");
    }
    if let Some(host) = authority_host(&v) {
        let host = host.trim_start_matches('[').trim_end_matches(']');
        if host == "localhost"
            || host.starts_with("127.")
            || host == "0.0.0.0"
            || host == "::1"
            || matches!(
                host,
                "llm" | "openai" | "anthropic" | "ollama" | "model-server" | "model_server"
            )
        {
            return forbidden("loopback or direct-LLM authority");
        }
    }
    if v.contains("//localhost/")
        || v.starts_with("localhost:")
        || v.starts_with("localhost/")
        || v.starts_with("127.")
        || v.starts_with("0.0.0.0")
        || v.starts_with("[::1]")
        || v.starts_with("::1")
    {
        return forbidden("bare loopback");
    }
    Ok(())
}

fn require_trimmed(field: &str, value: &str) -> AtelierResult<()> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(())
}

/// `artifact_ref` and `manifest_ref` must name an ArtifactStore payload or
/// manifest (`artifact://...`): the schema asserts the prefix.
fn require_artifact_scheme(field: &str, value: &str) -> AtelierResult<()> {
    if !value.starts_with("artifact://") {
        return Err(AtelierError::Validation(format!(
            "{field} must be an ArtifactStore ref starting with artifact://"
        )));
    }
    Ok(())
}

fn validate_optional_ref(field: &str, value: Option<&str>) -> AtelierResult<()> {
    if let Some(value) = value {
        validate_native_portable_ref(field, value)?;
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
    validate_native_portable_ref("artifact_ref", &new.artifact_ref)?;
    require_artifact_scheme("artifact_ref", &new.artifact_ref)?;
    validate_optional_ref("manifest_ref", new.manifest_ref.as_deref())?;
    if let Some(manifest_ref) = new.manifest_ref.as_deref() {
        require_artifact_scheme("manifest_ref", manifest_ref)?;
    }
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

/// One `atelier_sheet_artifact_link` row as the store returns it; record links
/// are reduced to their uuid keys in the select list.
#[derive(SurrealValue)]
struct SheetArtifactLinkRow {
    link_id: SurrealUuid,
    character_internal_id: SurrealUuid,
    sheet_version_id: SurrealUuid,
    artifact_kind: String,
    artifact_ref: String,
    manifest_ref: Option<String>,
    source_ref: Option<String>,
    label: Option<String>,
    reuse_role: Option<String>,
    linked_by: String,
    metadata: serde_json::Value,
    created_at_utc: Datetime,
    detached_at_utc: Option<Datetime>,
    detached_by: Option<String>,
}

impl TryFrom<SheetArtifactLinkRow> for SheetArtifactLink {
    type Error = AtelierError;

    fn try_from(row: SheetArtifactLinkRow) -> AtelierResult<Self> {
        let link_id: Uuid = row.link_id.into();
        let character_internal_id: Uuid = row.character_internal_id.into();
        let sheet_version_id: Uuid = row.sheet_version_id.into();
        Ok(SheetArtifactLink {
            link_id,
            character_internal_id,
            sheet_version_id,
            sheet_version_ref: sheet_version_ref(character_internal_id, sheet_version_id),
            typed_ref: sheet_artifact_ref(link_id),
            artifact_kind: SheetArtifactKind::from_token(&row.artifact_kind)?,
            artifact_ref: row.artifact_ref,
            manifest_ref: row.manifest_ref,
            source_ref: row.source_ref,
            label: row.label,
            reuse_role: row.reuse_role,
            linked_by: row.linked_by,
            metadata: row.metadata,
            created_at_utc: row.created_at_utc.into(),
            detached_at_utc: row.detached_at_utc.map(Into::into),
            detached_by: row.detached_by,
        })
    }
}

#[derive(SurrealValue)]
struct SheetVersionOwnerRow {
    character_internal_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct VersionIdBinding {
    version_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct ActiveLinkLookupBindings {
    sheet_version: RecordId,
    artifact_kind: String,
    artifact_ref: String,
}

#[derive(SurrealValue)]
struct LinkIdBinding {
    link_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct SheetVersionRefBinding {
    sheet_version: RecordId,
}

#[derive(Clone, SurrealValue)]
struct LinkSheetArtifactBindings {
    link_record: RecordId,
    link_id: SurrealUuid,
    character: RecordId,
    sheet_version: RecordId,
    artifact_kind: String,
    artifact_ref: String,
    manifest_ref: Option<String>,
    source_ref: Option<String>,
    label: Option<String>,
    reuse_role: Option<String>,
    linked_by: String,
    metadata: serde_json::Value,
}

#[derive(Clone, SurrealValue)]
struct DetachSheetArtifactBindings {
    link_record: RecordId,
    detached_by: String,
}

macro_rules! link_columns {
    () => {
        "link_id, record::id(character_internal_id) AS character_internal_id, \
         record::id(sheet_version_id) AS sheet_version_id, artifact_kind, artifact_ref, \
         manifest_ref, source_ref, label, reuse_role, linked_by, metadata, created_at_utc, \
         detached_at_utc, detached_by"
    };
}

const SELECT_SHEET_VERSION_OWNER: &str =
    "SELECT record::id(character_internal_id) AS character_internal_id \
     FROM atelier_sheet_version WHERE version_id = $version_id LIMIT 1;";

const SELECT_ACTIVE_LINK_BY_REF: &str = concat!(
    "SELECT ",
    link_columns!(),
    " FROM atelier_sheet_artifact_link WHERE sheet_version_id = $sheet_version \
       AND artifact_kind = $artifact_kind AND artifact_ref = $artifact_ref \
       AND detached_at_utc IS NONE LIMIT 1;"
);

const SELECT_ACTIVE_LINK_BY_ID: &str = concat!(
    "SELECT ",
    link_columns!(),
    " FROM atelier_sheet_artifact_link WHERE link_id = $link_id \
       AND detached_at_utc IS NONE LIMIT 1;"
);

const LIST_ACTIVE_LINKS_FOR_VERSION: &str = concat!(
    "SELECT ",
    link_columns!(),
    " FROM atelier_sheet_artifact_link WHERE sheet_version_id = $sheet_version \
       AND detached_at_utc IS NONE ORDER BY created_at_utc ASC, link_id ASC;"
);

/// Create the link row and its `SHEET_ARTIFACT_LINKED` event in one statement.
/// A concurrent attach of the same `(version, kind, ref)` loses on the
/// `active_link_key` UNIQUE index and the caller re-reads the winner.
const LINK_SHEET_ARTIFACT_STATEMENT: &str = concat!(
    "RETURN { CREATE $domain.link_record CONTENT { \
       link_id: $domain.link_id, \
       character_internal_id: $domain.character, \
       sheet_version_id: $domain.sheet_version, \
       artifact_kind: $domain.artifact_kind, \
       artifact_ref: $domain.artifact_ref, \
       manifest_ref: $domain.manifest_ref, \
       source_ref: $domain.source_ref, \
       label: $domain.label, \
       reuse_role: $domain.reuse_role, \
       linked_by: $domain.linked_by, \
       metadata: $domain.metadata \
     } RETURN NONE; ",
    atelier_event_sql!(),
    " RETURN (SELECT ",
    link_columns!(),
    " FROM $domain.link_record)[0]; };"
);

/// The `THROW` token the detach statement raises when the link is missing or
/// already detached; mapped to [`AtelierError::NotFound`] (detach is
/// active-only, so a second detach neither rewrites the row nor emits a
/// second event).
const SHEET_ARTIFACT_NOT_ACTIVE_THROW: &str = "HSK-SHEET-ARTIFACT-NOT-ACTIVE";

/// Soft-detach and record `SHEET_ARTIFACT_DETACHED` in one statement. The
/// active check precedes the UPDATE, so a THROW leaves nothing written.
const DETACH_SHEET_ARTIFACT_STATEMENT: &str = concat!(
    "RETURN { LET $current = (SELECT detached_at_utc FROM $domain.link_record)[0]; \
     IF $current = NONE OR $current.detached_at_utc != NONE { THROW 'HSK-SHEET-ARTIFACT-NOT-ACTIVE'; }; \
     UPDATE $domain.link_record SET detached_at_utc = time::now(), detached_by = $domain.detached_by RETURN NONE; ",
    atelier_event_sql!(),
    " RETURN (SELECT ",
    link_columns!(),
    " FROM $domain.link_record)[0]; };"
);

fn is_active_link_conflict(error: &AtelierError) -> bool {
    let text = error.to_string();
    text.contains("already contains") || text.contains("ux_atelier_sheet_artifact_link_active_ref")
}

impl AtelierStore {
    pub async fn link_sheet_artifact(
        &self,
        new: &NewSheetArtifactLink,
    ) -> AtelierResult<SheetArtifactLink> {
        Ok(self.link_sheet_artifact_with_status(new).await?.link)
    }

    async fn find_active_sheet_artifact(
        &self,
        sheet_version_id: Uuid,
        artifact_kind: SheetArtifactKind,
        artifact_ref: &str,
    ) -> AtelierResult<Option<SheetArtifactLink>> {
        let bindings = ActiveLinkLookupBindings {
            sheet_version: RecordId::new(
                "atelier_sheet_version",
                SurrealUuid::from(sheet_version_id),
            ),
            artifact_kind: artifact_kind.as_token().to_owned(),
            artifact_ref: artifact_ref.to_owned(),
        };
        let row: Option<SheetArtifactLinkRow> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(SELECT_ACTIVE_LINK_BY_REF, bindings).await })
            })
            .await?;
        row.map(SheetArtifactLink::try_from).transpose()
    }

    /// Attach a reusable artifact ref to a sheet version. Idempotent on
    /// `(sheet_version_id, artifact_kind, artifact_ref)` over ACTIVE links: a
    /// repeat attach returns the existing link with `created = false` and emits
    /// no second event.
    pub async fn link_sheet_artifact_with_status(
        &self,
        new: &NewSheetArtifactLink,
    ) -> AtelierResult<SheetArtifactLinkWrite> {
        validate_new_sheet_artifact_link(new)?;
        let owner: Option<SheetVersionOwnerRow> = self
            .with_data({
                let bindings = VersionIdBinding {
                    version_id: SurrealUuid::from(new.sheet_version_id),
                };
                move |ctx| {
                    Box::pin(async move { ctx.query_first(SELECT_SHEET_VERSION_OWNER, bindings).await })
                }
            })
            .await?;
        let Some(owner) = owner else {
            return Err(AtelierError::NotFound(format!(
                "sheet version version_id={}",
                new.sheet_version_id
            )));
        };
        let sheet_character_internal_id: Uuid = owner.character_internal_id.into();
        if sheet_character_internal_id != new.character_internal_id {
            return Err(AtelierError::Validation(format!(
                "sheet_version_id={} does not belong to character_internal_id={}",
                new.sheet_version_id, new.character_internal_id
            )));
        }

        if let Some(existing) = self
            .find_active_sheet_artifact(new.sheet_version_id, new.artifact_kind, &new.artifact_ref)
            .await?
        {
            return Ok(SheetArtifactLinkWrite {
                link: existing,
                created: false,
            });
        }

        let link_id = Uuid::now_v7();
        let bindings = LinkSheetArtifactBindings {
            link_record: RecordId::new("atelier_sheet_artifact_link", SurrealUuid::from(link_id)),
            link_id: SurrealUuid::from(link_id),
            character: RecordId::new(
                "atelier_character",
                SurrealUuid::from(new.character_internal_id),
            ),
            sheet_version: RecordId::new(
                "atelier_sheet_version",
                SurrealUuid::from(new.sheet_version_id),
            ),
            artifact_kind: new.artifact_kind.as_token().to_owned(),
            artifact_ref: new.artifact_ref.clone(),
            manifest_ref: new.manifest_ref.clone(),
            source_ref: new.source_ref.clone(),
            label: new.label.clone(),
            reuse_role: new.reuse_role.clone(),
            linked_by: new.linked_by.clone(),
            metadata: new.metadata.clone(),
        };
        let typed_ref = sheet_artifact_ref(link_id);
        let written = self
            .write_with_event(
                LINK_SHEET_ARTIFACT_STATEMENT,
                bindings,
                sheet_artifact_event_family::SHEET_ARTIFACT_LINKED,
                "atelier_sheet_artifact_link",
                &link_id.to_string(),
                serde_json::json!({
                    "link_id": link_id,
                    "typed_ref": typed_ref,
                    "sheet_version_ref": sheet_version_ref(new.character_internal_id, new.sheet_version_id),
                    "artifact_kind": new.artifact_kind.as_token(),
                    "artifact_ref": &new.artifact_ref,
                    "reuse_role": &new.reuse_role,
                    "linked_by": &new.linked_by,
                }),
            )
            .await;
        let row: Option<SheetArtifactLinkRow> = match written {
            Ok(row) => row,
            Err(error) if is_active_link_conflict(&error) => {
                let existing = self
                    .find_active_sheet_artifact(
                        new.sheet_version_id,
                        new.artifact_kind,
                        &new.artifact_ref,
                    )
                    .await?
                    .ok_or_else(|| {
                        AtelierError::Conflict(format!(
                            "active sheet artifact link disappeared for sheet_version_id={} artifact_ref={}",
                            new.sheet_version_id, new.artifact_ref
                        ))
                    })?;
                return Ok(SheetArtifactLinkWrite {
                    link: existing,
                    created: false,
                });
            }
            Err(error) => return Err(error),
        };
        let link: SheetArtifactLink = row
            .ok_or_else(|| {
                AtelierError::Internal("linking a sheet artifact returned no row".to_owned())
            })?
            .try_into()?;
        Ok(SheetArtifactLinkWrite {
            link,
            created: true,
        })
    }

    /// Resolve one ACTIVE typed ref; a detached link is not found.
    pub async fn get_sheet_artifact(&self, link_id: Uuid) -> AtelierResult<SheetArtifactLink> {
        let bindings = LinkIdBinding {
            link_id: SurrealUuid::from(link_id),
        };
        let row: Option<SheetArtifactLinkRow> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(SELECT_ACTIVE_LINK_BY_ID, bindings).await })
            })
            .await?;
        row.ok_or_else(|| AtelierError::NotFound(format!("sheet artifact link_id={link_id}")))?
            .try_into()
    }

    /// Active links of one sheet version, oldest first.
    pub async fn list_sheet_artifacts(
        &self,
        sheet_version_id: Uuid,
    ) -> AtelierResult<Vec<SheetArtifactLink>> {
        let bindings = SheetVersionRefBinding {
            sheet_version: RecordId::new(
                "atelier_sheet_version",
                SurrealUuid::from(sheet_version_id),
            ),
        };
        let rows: Vec<SheetArtifactLinkRow> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_ACTIVE_LINKS_FOR_VERSION, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter().map(SheetArtifactLink::try_from).collect()
    }

    /// Soft-detach one active link. The row is kept (`detached_at_utc`,
    /// `detached_by`), leaves the active set, and a second detach is NotFound.
    pub async fn detach_sheet_artifact(
        &self,
        link_id: Uuid,
        detached_by: &str,
    ) -> AtelierResult<SheetArtifactLink> {
        require_trimmed("detached_by", detached_by)?;
        let current = self.get_sheet_artifact(link_id).await?;
        let bindings = DetachSheetArtifactBindings {
            link_record: RecordId::new("atelier_sheet_artifact_link", SurrealUuid::from(link_id)),
            detached_by: detached_by.to_owned(),
        };
        let written = self
            .write_with_event(
                DETACH_SHEET_ARTIFACT_STATEMENT,
                bindings,
                sheet_artifact_event_family::SHEET_ARTIFACT_DETACHED,
                "atelier_sheet_artifact_link",
                &link_id.to_string(),
                serde_json::json!({
                    "link_id": link_id,
                    "typed_ref": current.typed_ref,
                    "sheet_version_ref": current.sheet_version_ref,
                    "artifact_kind": current.artifact_kind.as_token(),
                    "artifact_ref": current.artifact_ref,
                    "detached_by": detached_by,
                }),
            )
            .await;
        let row: Option<SheetArtifactLinkRow> = match written {
            Ok(row) => row,
            Err(error) if error.to_string().contains(SHEET_ARTIFACT_NOT_ACTIVE_THROW) => {
                return Err(AtelierError::NotFound(format!(
                    "sheet artifact link_id={link_id}"
                )));
            }
            Err(error) => return Err(error),
        };
        row.ok_or_else(|| {
            AtelierError::Internal("detaching a sheet artifact returned no row".to_owned())
        })?
        .try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_ref_predicate_matches_the_reference_migration() {
        for accepted in [
            "artifact://.handshake/artifacts/L1/0191/payload",
            "posekit://rig/0191",
            "comfy://workflow-run/0191",
            "receipt://atelier/comfy/0191",
        ] {
            assert!(
                validate_native_portable_ref("ref", accepted).is_ok(),
                "{accepted} must be accepted"
            );
        }
        for (rejected, forbidden) in [
            ("", false),
            (" padded", false),
            ("artifact://with space/x", false),
            ("D:\\training\\openpose\\bad.png", true),
            ("/tmp/openpose.png", true),
            ("artifact://atelier/cache.db?x=1", true),
            ("artifact://atelier/cache.sqlite3/x", true),
            ("sqlite:memory", true),
            ("file:///x", true),
            ("~/x", true),
            ("//share/x", true),
            ("artifact://a/../b", true),
            ("../b", true),
            ("artifact://x/.GOV/y", true),
            ("electron:x", true),
            ("artifact://ckc/x", true),
            ("artifact://x/castkit/y", true),
            ("openai:gpt", true),
            ("http://localhost:8000/x", true),
            ("http://127.0.0.1/x", true),
            ("http://[::1]/x", true),
            ("http://user@ollama/x", true),
            ("localhost:8000", true),
            ("0.0.0.0:1", true),
            ("artifact://x//localhost/y", true),
            ("artifact://x/%USERPROFILE%/y", true),
            ("artifact://x/c:/y", true),
        ] {
            let err = validate_native_portable_ref("ref", rejected)
                .expect_err(&format!("{rejected:?} must be rejected"));
            if forbidden {
                assert!(
                    matches!(err, AtelierError::ForbiddenStorage(_)),
                    "{rejected:?} should be ForbiddenStorage, got {err:?}"
                );
            } else {
                assert!(
                    matches!(err, AtelierError::Validation(_)),
                    "{rejected:?} should be Validation, got {err:?}"
                );
            }
        }
    }

    #[test]
    fn reuse_role_is_a_lowercase_portable_token() {
        assert!(validate_optional_reuse_role(Some("cui_openpose_conditioning")).is_ok());
        assert!(validate_optional_reuse_role(Some("a.b-c_9")).is_ok());
        assert!(validate_optional_reuse_role(Some("CUI_OpenPose")).is_err());
        assert!(validate_optional_reuse_role(Some("-lead")).is_err());
        assert!(validate_optional_reuse_role(Some(" padded")).is_err());
        assert!(validate_optional_reuse_role(None).is_ok());
    }

    #[test]
    fn statements_select_every_row_column() {
        for column in [
            "link_id",
            "character_internal_id",
            "sheet_version_id",
            "artifact_kind",
            "artifact_ref",
            "manifest_ref",
            "source_ref",
            "label",
            "reuse_role",
            "linked_by",
            "metadata",
            "created_at_utc",
            "detached_at_utc",
            "detached_by",
        ] {
            for statement in [
                SELECT_ACTIVE_LINK_BY_REF,
                SELECT_ACTIVE_LINK_BY_ID,
                LIST_ACTIVE_LINKS_FOR_VERSION,
                LINK_SHEET_ARTIFACT_STATEMENT,
                DETACH_SHEET_ARTIFACT_STATEMENT,
            ] {
                assert!(
                    statement.contains(column),
                    "SheetArtifactLinkRow reads `{column}` but a statement does not select it"
                );
            }
        }
    }
}

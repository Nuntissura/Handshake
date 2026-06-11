//! Atelier/Lens domain (WP-KERNEL-005 legacy source fold-in).
//!
//! Storage authority is PostgreSQL + EventLedger + ArtifactStore + CRDT only.
//! SQLite is FORBIDDEN in any form (runtime, tests, fixtures, cache, fallback);
//! see [`assert_postgres_url`] (MT-004) and the kernel `no_sqlite_tripwire`.
//!
//! Module boundaries (MT-003): `core` (character identity + append-only sheet
//! versions), `media` (DAM), with `intake`/`collections`/`search`/`exports`
//! folded in by later microtasks. Every mutation is intended to emit an
//! EventLedger / Flight Recorder event from the [`event_family`] set (MT-005).

#[cfg(feature = "runtime-full")]
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
#[cfg(feature = "runtime-full")]
use crate::storage::Database;
use sha2::{Digest, Sha256};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Postgres, Row, Transaction};
#[cfg(feature = "runtime-full")]
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

pub mod acceptance;
pub mod action_receipt;
pub mod annotation;
pub mod bulk;
pub mod collections;
pub mod comfy;
pub mod command_corpus;
pub mod core;
pub mod dcc_flight_recorder;
pub mod documents;
pub mod downloader;
pub mod editable_surface_authority;
pub mod exports;
pub mod filesystem_health;
pub mod image_import;
pub mod intake;
pub mod links;
pub mod media;
pub mod model_lease;
pub mod model_manual_merge;
pub mod moodboards;
pub mod pose;
pub mod relationships;
pub mod scripts;
pub mod search;
pub mod settings;
pub mod sheet;
pub mod source_evidence;
pub mod sourcing;
pub mod state_probe;
pub mod stealth_window;
pub mod transcript;
pub mod validator_first_pass;
pub mod visual_steer_feedback;

pub use self::bulk::{
    BulkExportRequestResult, BulkOperationReceipt, BulkTagRequest, BulkTrashMediaRequest,
    DeletionArchiveRequest, DeletionImpactPreview, DeletionImpactPreviewRequest,
    DeletionImpactTarget, DeletionRestoreRequest, DeletionTargetKind, DeletionTargetRef,
};
pub use self::core::{Character, NewCharacter};
pub use self::filesystem_health::{
    FilesystemHealthCheck, FilesystemHealthCheckRequest, FilesystemHealthFinding,
    FilesystemHealthFindingKind, FilesystemHealthReport,
};
pub use self::image_import::{
    ClipboardImageImportRequest, ImageImportRecord, UrlImageImportRequest,
};
pub use self::media::{
    BulkMediaReviewMetadataResult, MediaAsset, MediaDerivative, MediaDerivativeFailure,
    MediaDerivativeGenerated, MediaDerivativeKind, MediaDerivativeRequest, MediaDerivativeStatus,
    MediaReviewMetadata, MediaReviewMetadataUpdate, MediaSidecar, MediaSidecarRelationKind,
    MediaSourceProvenanceRefs, NewMediaAsset, NewMediaSidecarRelation,
    SetMediaSourceProvenanceRefs,
};
pub use self::relationships::{
    CharacterRelationship, CharacterRelationshipGraph, CharacterRelationshipGraphEdge,
    CharacterRelationshipGraphNode, NewCharacterRelationship, UpdateCharacterRelationship,
};
pub use self::sheet::{
    BulkSheetFieldEditResult, NewSheetVersion, ParsedSheetFieldType, ParsedSheetTemplate,
    SheetBlockInstance, SheetBlockInstanceField, SheetBlockSchema, SheetFieldEdit,
    SheetFieldEditRequest, SheetFieldEditResult, SheetFieldSelector, SheetTemplateAst,
    SheetTemplateField, SheetTemplateSection, SheetUnmappedLine, SheetVersion,
    SheetVersionRevertRequest, SheetVersionRevertResult,
};

/// Errors surfaced by the atelier domain.
#[derive(Debug, Error)]
pub enum AtelierError {
    #[error("atelier database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("atelier entity not found: {0}")]
    NotFound(String),
    #[error("atelier conflict: {0}")]
    Conflict(String),
    #[error("forbidden storage backend: {0}")]
    ForbiddenStorage(String),
    #[error("atelier validation error: {0}")]
    Validation(String),
    #[error("atelier event ledger error: {0}")]
    EventLedger(String),
    #[error("atelier flight recorder error: {0}")]
    #[cfg(feature = "runtime-full")]
    FlightRecorder(String),
}

pub type AtelierResult<T> = Result<T, AtelierError>;

/// Atelier EventLedger / Flight Recorder event families (MT-005).
///
/// These are the canonical seams every Core/Data mutation must emit so the
/// operator surface, Locus, and replay can reconstruct atelier history.
pub mod event_family {
    use super::action_receipt::action_receipt_event_family;
    use super::collections::collections_event_family;
    use super::comfy::comfy_event_family;
    use super::documents::documents_event_family;
    use super::exports::export_event_family;
    use super::filesystem_health::filesystem_health_event_family;
    use super::intake::intake_event_family;
    use super::links::links_event_family;
    use super::moodboards::moodboard_event_family;
    use super::pose::pose_event_family;
    use super::relationships::relationships_event_family;
    use super::scripts::scripts_event_family;
    use super::search::search_event_family;
    use super::settings::settings_event_family;
    use super::source_evidence::source_evidence_event_family;
    use super::command_corpus::diagnostics_event_family;
    use super::command_corpus::command_log_event_family;
    use super::dcc_flight_recorder::dcc_flight_recorder_event_family;
    use super::model_manual_merge::model_manual_merge_event_family;
    use super::settings::model_workflow_event_family;
    use super::state_probe::diagnostics_projection_event_family;
    use super::state_probe::state_probe_event_family;
    use super::stealth_window::stealth_ref_event_family;
    use super::visual_steer_feedback::visual_steer_event_family;

    pub const CHARACTER_CREATED: &str = "atelier.character.created";
    pub const SHEET_VERSION_APPENDED: &str = "atelier.sheet.version_appended";
    pub const SHEET_TEMPLATE_PARSED: &str = "atelier.sheet.template_parsed";
    pub const SHEET_FIELD_EDITS_APPLIED: &str = "atelier.sheet.field_edits_applied";
    pub const SHEET_FIELD_EDIT_REJECTED: &str = "atelier.sheet.field_edit_rejected";
    pub const SHEET_VERSION_REVERTED: &str = "atelier.sheet.version_reverted";
    pub const MEDIA_ASSET_MATERIALIZED: &str = "atelier.media.asset_materialized";
    pub const MEDIA_DERIVATIVE_REQUESTED: &str = "atelier.media.derivative_requested";
    pub const MEDIA_DERIVATIVE_GENERATING: &str = "atelier.media.derivative_generating";
    pub const MEDIA_DERIVATIVE_GENERATED: &str = "atelier.media.derivative_generated";
    pub const MEDIA_DERIVATIVE_FAILED: &str = "atelier.media.derivative_failed";
    pub const MEDIA_DERIVATIVE_RETRIED: &str = "atelier.media.derivative_retried";
    pub const MEDIA_REVIEW_METADATA_UPDATED: &str = "atelier.media.review_metadata_updated";
    pub const MEDIA_SIDECAR_RECORDED: &str = "atelier.media.sidecar_recorded";
    pub const MEDIA_SOURCE_PROVENANCE_REFS_SET: &str = "atelier.media.source_provenance_refs_set";
    pub const IMAGE_IMPORT_RECORDED: &str = "atelier.image_import.recorded";
    pub const BULK_OPERATION_APPLIED: &str = "atelier.bulk.operation_applied";

    /// All known atelier event families (used by parity/coverage checks).
    pub const ALL: &[&str] = &[
        CHARACTER_CREATED,
        SHEET_VERSION_APPENDED,
        SHEET_TEMPLATE_PARSED,
        SHEET_FIELD_EDITS_APPLIED,
        SHEET_FIELD_EDIT_REJECTED,
        SHEET_VERSION_REVERTED,
        MEDIA_ASSET_MATERIALIZED,
        MEDIA_DERIVATIVE_REQUESTED,
        MEDIA_DERIVATIVE_GENERATING,
        MEDIA_DERIVATIVE_GENERATED,
        MEDIA_DERIVATIVE_FAILED,
        MEDIA_DERIVATIVE_RETRIED,
        MEDIA_REVIEW_METADATA_UPDATED,
        MEDIA_SIDECAR_RECORDED,
        MEDIA_SOURCE_PROVENANCE_REFS_SET,
        IMAGE_IMPORT_RECORDED,
        BULK_OPERATION_APPLIED,
        comfy_event_family::PROBE_RECORDED,
        comfy_event_family::CAPABILITY_REGISTERED,
        comfy_event_family::CAPABILITY_REJECTED,
        comfy_event_family::OUTPUT_MATERIALIZED,
        comfy_event_family::OUTPUT_DEDUPLICATED,
        comfy_event_family::FALLBACK_ENGAGED,
        comfy_event_family::RECEIPT_PRODUCED,
        comfy_event_family::WORKFLOW_RECEIPT_RECORDED,
        comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RECORDED,
        comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RETRIED,
        comfy_event_family::REPLAY_REQUESTED,
        comfy_event_family::REPLAY_COMPLETED,
        comfy_event_family::REPLAY_FAILED,
        comfy_event_family::WORKFLOW_SPEC_REGISTERED,
        comfy_event_family::VERSION_METADATA_RECORDED,
        comfy_event_family::JOB_ENQUEUED,
        comfy_event_family::JOB_RUNNING,
        comfy_event_family::JOB_COMPLETED,
        comfy_event_family::JOB_FAILED,
        comfy_event_family::JOB_CANCELLED,
        comfy_event_family::JOB_TIMED_OUT,
        comfy_event_family::JOB_PARTIAL_EVIDENCE_PRESERVED,
        comfy_event_family::DIAGNOSTIC_BUNDLE_RECORDED,
        action_receipt_event_family::ACTION_RECEIPT_RECORDED,
        intake_event_family::INTAKE_BATCH_CREATED,
        intake_event_family::INTAKE_ITEM_ADDED,
        intake_event_family::INTAKE_ITEM_CLASSIFIED,
        intake_event_family::INTAKE_ITEM_REJECTION_AUDITED,
        intake_event_family::INTAKE_BATCH_CLOSED,
        intake_event_family::INTAKE_BATCH_RESUMED,
        intake_event_family::INTAKE_FOLDER_SCAN_COMPLETED,
        intake_event_family::RESET_RECORDED,
        intake_event_family::ORPHAN_MANIFEST_RECORDED,
        intake_event_family::ORPHAN_MANIFEST_ITEM_ADOPTED,
        export_event_family::EXPORT_REQUESTED,
        export_event_family::EXPORT_RENDERED,
        export_event_family::EXPORT_MANIFEST_ITEM_ADDED,
        export_event_family::EXPORT_INTAKE_LINK_ATTACHED,
        export_event_family::CONTACT_SHEET_RASTER_EXPORT_PLANNED,
        export_event_family::WEB_PORTFOLIO_EXPORT_REQUESTED,
        export_event_family::WEB_PORTFOLIO_EXPORT_RENDERED,
        export_event_family::BACKUP_MANIFEST_RECORDED,
        export_event_family::BACKUP_RESTORE_PREFLIGHT_RECORDED,
        collections_event_family::COLLECTION_CREATED,
        collections_event_family::COLLECTION_UPDATED,
        collections_event_family::COLLECTION_IMAGES_ADDED,
        collections_event_family::COLLECTION_IMAGES_REMOVED,
        collections_event_family::CONTACT_SHEET_CREATED,
        collections_event_family::CONTACT_SHEET_SVG_RENDERED,
        collections_event_family::MEDIA_ASSET_TAGGED,
        collections_event_family::MEDIA_ASSET_UNTAGGED,
        collections_event_family::COLLECTION_METADATA_APPLIED,
        documents_event_family::CHARACTER_DOCUMENT_CREATED,
        documents_event_family::CHARACTER_DOCUMENT_VERSION_APPENDED,
        documents_event_family::STORY_CARD_ADDED,
        documents_event_family::STORY_BEAT_ADDED,
        links_event_family::BRACKET_LINKS_REBUILT,
        relationships_event_family::CHARACTER_RELATIONSHIP_CREATED,
        relationships_event_family::CHARACTER_RELATIONSHIP_UPDATED,
        relationships_event_family::CHARACTER_RELATIONSHIP_DELETED,
        moodboard_event_family::MOODBOARD_SNAPSHOT_RECORDED,
        moodboard_event_family::MOODBOARD_OPERATION_RECORDED,
        moodboard_event_family::MOODBOARD_EXPORT_REQUESTED,
        pose_event_family::POSE_RIG_INGESTED,
        pose_event_family::POSE_HEAD_POSE_RECORDED,
        pose_event_family::POSE_CALIBRATION_SET,
        pose_event_family::POSE_SIDECAR_RECORDED,
        pose_event_family::POSE_CONTEXT_STATE_SET,
        pose_event_family::POSE_WORKSPACE_RIG_STATE_SET,
        pose_event_family::IDENTITY_PROFILE_APPENDED,
        pose_event_family::IDENTITY_CROP_ARTIFACT_RECORDED,
        pose_event_family::POSE_DEFERRED_FEATURE_RECORDED,
        scripts_event_family::CHARACTER_SCRIPT_CREATED,
        scripts_event_family::CHARACTER_SCRIPT_USAGE_RECORDED,
        filesystem_health_event_family::CHECK_RECORDED,
        search_event_family::CHARACTER_TAGGED,
        search_event_family::CHARACTER_UNTAGGED,
        search_event_family::TAG_RULE_UPSERTED,
        search_event_family::TAG_RULE_DELETED,
        search_event_family::DERIVED_TAGS_RECOMPUTED,
        search_event_family::SIMILARITY_PROJECTED,
        search_event_family::SIMILARITY_REBUILD_COMPLETED,
        search_event_family::SIMILARITY_REBUILD_FAILED,
        search_event_family::AI_TAG_SUGGESTION_RECORDED,
        search_event_family::AI_TAG_SUGGESTION_ACCEPTED,
        search_event_family::AI_TAG_SUGGESTION_REJECTED,
        search_event_family::AI_TAG_SUGGESTION_APPLIED,
        search_event_family::SAVED_SEARCH_UPSERTED,
        search_event_family::SAVED_SEARCH_DELETED,
        settings_event_family::PREFERENCE_SET,
        settings_event_family::PREFERENCE_RESET_TO_DEFAULT,
        settings_event_family::PREFERENCE_DELETED,
        settings_event_family::RETENTION_PRUNE_CONFIRMED,
        source_evidence_event_family::SOURCE_EVIDENCE_MATRIX_RECORDED,
        state_probe_event_family::STATE_PROBE_CATALOG_RECORDED,
        state_probe_event_family::DIAGNOSTICS_VALIDATION_ROW_RECORDED,
        diagnostics_event_family::DIAGNOSTICS_ERROR_TAXONOMY_RECORDED,
        diagnostics_event_family::DIAGNOSTICS_PROMPT_RESPONSE_MATRIX_RECORDED,
        diagnostics_event_family::DIAGNOSTICS_RESET_ORPHAN_PROJECTED,
        command_log_event_family::COMMAND_LOG_RECORDED,
        command_log_event_family::SESSION_HEARTBEAT_RECORDED,
        command_log_event_family::SESSION_FLAGGED_STALE,
        model_workflow_event_family::MODEL_CONFIG_RECORDED,
        model_workflow_event_family::MODEL_APPLY_DRAFTED,
        model_workflow_event_family::MODEL_APPLY_STATE_ADVANCED,
        model_workflow_event_family::SYNTHETIC_INPUT_RECORDED,
        diagnostics_projection_event_family::WORK_STATE_PROJECTION_RECORDED,
        diagnostics_projection_event_family::DCC_PANEL_PROJECTION_RECORDED,
        diagnostics_projection_event_family::SCREENSHOT_ARTIFACT_STORED,
        diagnostics_projection_event_family::SCREENSHOT_ARTIFACT_RETENTION_CLEANED,
        diagnostics_projection_event_family::SPEC_DRIFT_FINDING_RECORDED,
        visual_steer_event_family::VISUAL_STEER_FEEDBACK_RECORDED,
        dcc_flight_recorder_event_family::DCC_WORKFLOW_PANEL_PROJECTION_RECORDED,
        dcc_flight_recorder_event_family::FR_WORKFLOW_EVENT_RECORDED,
        model_manual_merge_event_family::MANUAL_ROW_MERGE_RECORDED,
        model_manual_merge_event_family::MANUAL_DRIFT_GUARD_RECORDED,
        stealth_ref_event_family::STEALTH_REF_WINDOW_CREATED,
        stealth_ref_event_family::STEALTH_REF_ADDED,
        stealth_ref_event_family::STEALTH_REF_REMOVED,
        stealth_ref_event_family::STEALTH_REF_REORDERED,
        stealth_ref_event_family::STEALTH_REF_CAPTURED,
        stealth_ref_event_family::STEALTH_REF_WINDOW_CLOSED,
    ];
}

/// Runtime rejection of forbidden legacy source storage assumptions (MT-004).
///
/// SQLite is forbidden in any form; only `postgres://` / `postgresql://`
/// connection strings are accepted as atelier storage authority.
pub fn assert_postgres_url(url: &str) -> AtelierResult<()> {
    let normalized = url.trim().to_ascii_lowercase();
    let is_postgres =
        normalized.starts_with("postgres://") || normalized.starts_with("postgresql://");
    if is_postgres {
        return Ok(());
    }
    if normalized.starts_with("sqlite:")
        || normalized.ends_with(".sqlite")
        || normalized.ends_with(".sqlite3")
        || normalized.ends_with(".db")
    {
        return Err(AtelierError::ForbiddenStorage(
            "SQLite is forbidden in Handshake; atelier requires PostgreSQL".to_string(),
        ));
    }
    Err(AtelierError::ForbiddenStorage(
        "atelier requires a PostgreSQL DATABASE_URL (postgres:// or postgresql://)".to_string(),
    ))
}

/// Reject stale local-runtime assumptions in user/product refs that cross the
/// atelier persistence boundary (MT-004).
fn authority_host_from_ref(value: &str) -> Option<&str> {
    let (_, after_scheme) = value.split_once("://")?;
    let authority = after_scheme
        .split(|ch| matches!(ch, '/' | '?' | '#'))
        .next()
        .unwrap_or_default();
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    if let Some(rest) = host_port.strip_prefix('[') {
        return rest.split_once(']').map(|(host, _)| host);
    }
    host_port.split(':').next()
}

fn is_loopback_or_unspecified_host(host: &str) -> bool {
    host == "localhost"
        || host.starts_with("127.")
        || host == "0.0.0.0"
        || host == "::1"
        || host == "[::1]"
}

pub fn reject_legacy_runtime_ref(field: &str, value: &str) -> AtelierResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }

    let lower = trimmed.to_ascii_lowercase();
    let normalized = lower.replace('\\', "/");
    let sqlite_probe = normalized
        .split(|ch| matches!(ch, '?' | '#'))
        .next()
        .unwrap_or(normalized.as_str());
    let has_sqlite_ref = normalized.starts_with("sqlite:")
        || sqlite_probe.ends_with(".sqlite")
        || sqlite_probe.ends_with(".sqlite3")
        || sqlite_probe.ends_with(".db")
        || sqlite_probe.contains(".sqlite/")
        || sqlite_probe.contains(".sqlite3/")
        || sqlite_probe.contains(".db/");
    let has_forbidden_segment = normalized
        .split(|ch| matches!(ch, '/' | ':' | '.' | '?' | '#' | '&' | '=' | '@'))
        .any(|segment| matches!(segment, "ckc" | "castkit" | "electron"));
    let has_windows_drive = trimmed.len() >= 2
        && trimmed.as_bytes()[1] == b':'
        && trimmed.as_bytes()[0].is_ascii_alphabetic();
    let has_embedded_windows_drive = normalized
        .as_bytes()
        .windows(3)
        .any(|window| window[0] == b'/' && window[1].is_ascii_alphabetic() && window[2] == b':');
    let has_forbidden_namespace = normalized == "ckc"
        || normalized == "castkit"
        || normalized.starts_with("ckc:")
        || normalized.starts_with("castkit:")
        || normalized.contains("/ckc/")
        || normalized.contains("/castkit/");
    let has_direct_llm_scheme = normalized.starts_with("llm:")
        || normalized.starts_with("openai:")
        || normalized.starts_with("anthropic:")
        || normalized.starts_with("ollama:")
        || normalized.starts_with("model-server:")
        || normalized.starts_with("model_server:");
    let has_direct_llm_authority = authority_host_from_ref(&normalized).is_some_and(|host| {
        matches!(
            host,
            "llm" | "openai" | "anthropic" | "ollama" | "model-server" | "model_server"
        )
    });
    let has_local_authority_host =
        authority_host_from_ref(&normalized).is_some_and(is_loopback_or_unspecified_host);
    let has_local_authority = normalized.contains("://localhost")
        || normalized.contains("://127.")
        || normalized.contains("://0.0.0.0")
        || normalized.contains("://[::1]")
        || normalized.contains("://::1")
        || normalized.contains("//localhost/");
    let has_bare_loopback = normalized == "localhost"
        || normalized.starts_with("localhost:")
        || normalized.starts_with("localhost/")
        || normalized.starts_with("127.")
        || normalized.starts_with("0.0.0.0")
        || normalized.starts_with("[::1]")
        || normalized.starts_with("::1");
    let has_machine_path = has_windows_drive
        || has_embedded_windows_drive
        || normalized.starts_with("file:")
        || normalized.contains("file://")
        || normalized.starts_with("//")
        || normalized.starts_with('/')
        || normalized.starts_with("~/")
        || normalized.contains("%userprofile%");

    if normalized.contains(".gov")
        || has_sqlite_ref
        || normalized.contains("/../")
        || normalized.ends_with("/..")
        || normalized.starts_with("../")
        || normalized.starts_with("electron:")
        || normalized.contains("/electron/")
        || has_forbidden_segment
        || has_forbidden_namespace
        || has_direct_llm_scheme
        || has_direct_llm_authority
        || has_local_authority_host
        || has_local_authority
        || has_bare_loopback
        || has_machine_path
    {
        return Err(AtelierError::ForbiddenStorage(format!(
            "{field} must be a Handshake-native portable ref, not SQLite/Electron/CKC/CastKit/localhost/direct-LLM/.GOV/machine-local storage"
        )));
    }

    Ok(())
}

pub(crate) fn event_ref_for_text(text: &str) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(text.as_bytes())))
}

fn event_ref_for_value(value: &serde_json::Value) -> serde_json::Value {
    let bytes = if let Some(text) = value.as_str() {
        text.as_bytes().to_vec()
    } else {
        serde_json::to_vec(value).unwrap_or_else(|_| value.to_string().into_bytes())
    };
    serde_json::Value::String(format!("sha256:{}", hex::encode(Sha256::digest(&bytes))))
}

fn event_refs_for_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .iter()
                .filter(|value| !value.is_null())
                .map(event_ref_for_value)
                .collect(),
        ),
        serde_json::Value::Null => serde_json::Value::Null,
        _ => event_ref_for_value(value),
    }
}

fn sensitive_event_replacement_key(key: &str) -> Option<&'static str> {
    match key {
        "character_internal_id" => Some("character_ref"),
        "character_internal_ids" | "character_ids" => Some("character_refs"),
        "idempotency_key" => Some("idempotency_key_ref"),
        "ingestion_key" => Some("ingestion_key_ref"),
        "source_path" => Some("source_path_ref"),
        "source_paths" => Some("source_path_refs"),
        "source_label" => Some("source_label_ref"),
        "normalized_url" => Some("normalized_url_ref"),
        "source_provenance" => Some("source_provenance_ref"),
        "source_provenances" => Some("source_provenance_refs"),
        "source_ref" => Some("source_ref_ref"),
        "reference_ref" => Some("reference_ref_ref"),
        "artifact_manifest_ref" => Some("artifact_manifest_ref_ref"),
        "artifact_manifest_refs" => Some("artifact_manifest_ref_refs"),
        "pack_path" => Some("pack_path_ref"),
        "configured_root" => Some("configured_root_ref"),
        "root_path" => Some("root_path_ref"),
        "output_root" => Some("output_root_ref"),
        "job_profile_ref" => Some("job_profile_ref_ref"),
        "display_name" => Some("display_name_ref"),
        "author" => Some("author_ref"),
        "file_name" => Some("file_name_ref"),
        "value" => Some("value_ref"),
        "value_before" => Some("value_before_ref"),
        "value_after" => Some("value_after_ref"),
        "default_value" => Some("default_value_ref"),
        "requested_by" => Some("requested_by_ref"),
        "confirmed_by" => Some("confirmed_by_ref"),
        _ => None,
    }
}

fn sensitive_event_count_key(key: &str) -> Option<&'static str> {
    match key {
        "character_internal_ids" | "character_ids" => Some("character_count"),
        "source_paths" => Some("source_path_count"),
        "artifact_manifest_refs" => Some("artifact_manifest_ref_count"),
        _ => None,
    }
}

fn sanitize_atelier_event_payload(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => {
            let mut safe = serde_json::Map::with_capacity(object.len());
            for (key, value) in object {
                if let Some(replacement_key) = sensitive_event_replacement_key(&key) {
                    if key == "character_internal_id" && value.is_null() {
                        safe.insert(
                            "character_scope".to_string(),
                            serde_json::Value::String("global".to_string()),
                        );
                        continue;
                    }
                    if let Some(count_key) = sensitive_event_count_key(&key) {
                        if let serde_json::Value::Array(values) = &value {
                            safe.insert(
                                count_key.to_string(),
                                serde_json::Value::Number(values.len().into()),
                            );
                        }
                    }
                    if !value.is_null() {
                        safe.insert(replacement_key.to_string(), event_refs_for_value(&value));
                    }
                } else {
                    safe.insert(key, sanitize_atelier_event_payload(value));
                }
            }
            serde_json::Value::Object(safe)
        }
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(sanitize_atelier_event_payload)
                .collect(),
        ),
        other => other,
    }
}

/// PostgreSQL-backed atelier data store. Wraps a shared [`PgPool`].
#[derive(Clone)]
pub struct AtelierStore {
    pool: PgPool,
    #[cfg(feature = "runtime-full")]
    flight_recorder: Option<Arc<dyn FlightRecorder>>,
}

impl AtelierStore {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            #[cfg(feature = "runtime-full")]
            flight_recorder: None,
        }
    }

    #[cfg(feature = "runtime-full")]
    pub fn with_event_ledger(pool: PgPool, _event_ledger: Arc<dyn Database>) -> Self {
        Self {
            pool,
            flight_recorder: None,
        }
    }

    #[cfg(feature = "runtime-full")]
    pub fn with_observability(
        pool: PgPool,
        _event_ledger: Arc<dyn Database>,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            pool,
            flight_recorder: Some(flight_recorder),
        }
    }

    /// Connect to a PostgreSQL DATABASE_URL and build a store. Rejects SQLite
    /// and any non-PostgreSQL backend (MT-004).
    pub async fn connect(database_url: &str) -> AtelierResult<Self> {
        assert_postgres_url(database_url)?;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self {
            pool,
            #[cfg(feature = "runtime-full")]
            flight_recorder: None,
        })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Idempotent, concurrency-safe bootstrap of the atelier schema from the
    /// canonical migration files (0030 foundation + 0031 core-data). A
    /// transaction-scoped advisory lock serializes concurrent bootstrap so
    /// parallel governed sessions / swarm agents never race on CREATE TABLE
    /// (the IF NOT EXISTS race). The lock auto-releases on commit. Safe to call
    /// repeatedly and from many connections at once.
    pub async fn ensure_schema(&self) -> AtelierResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(7305441001::bigint)")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0018_kernel_event_ledger.sql"
        ))
        .execute(&mut *tx)
        .await?;

        let ready_after_lock: bool = sqlx::query_scalar(
            r#"SELECT
                  to_regclass('atelier_event') IS NOT NULL
              AND to_regclass('atelier_intake_item') IS NOT NULL
              AND to_regclass('atelier_preference') IS NOT NULL
              AND to_regclass('atelier_pose_rig') IS NOT NULL
              AND to_regclass('atelier_pose_sidecar') IS NOT NULL
              AND to_regclass('atelier_pose_context_state') IS NOT NULL
              AND to_regclass('atelier_pose_workspace_rig_state') IS NOT NULL
              AND to_regclass('atelier_pose_deferred_feature') IS NOT NULL
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_pose_calibration'
                    AND column_name IN (
                        'head_pose_ref',
                        'marker_visibility',
                        'marker_colors',
                        'hand_rows',
                        'history_refs'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 5
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_pose_calibration'
                    AND constraint_name = 'chk_atelier_pose_calibration_history_refs_json'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_pose_rig'
                    AND column_name IN (
                        'detector_model',
                        'detector_model_version',
                        'source_asset_version_ref',
                        'source_asset_path_ref',
                        'confidence_available',
                        'error_reason'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 6
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_pose_rig'
                    AND constraint_name = 'chk_atelier_pose_rig_status_error'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_pose_sidecar'
                    AND column_name IN (
                        'source_asset_id',
                        'source_ref',
                        'role',
                        'manifest_ref',
                        'width',
                        'height',
                        'status',
                        'error_message'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 8
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_pose_sidecar'
                    AND constraint_name = 'chk_atelier_pose_sidecar_status_error'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_pose_sidecar'
                    AND constraint_name = 'chk_atelier_pose_sidecar_manifest_ref'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_pose_sidecar'
                    AND constraint_name = 'chk_atelier_pose_sidecar_role_kind'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_pose_context_state'
                    AND column_name IN (
                        'state_seq',
                        'workspace_ref',
                        'kind',
                        'source_asset_id',
                        'character_internal_id',
                        'collection_id',
                        'selected_rig_id',
                        'requested_by'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 8
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_pose_context_state'
                    AND constraint_name = 'chk_atelier_pose_context_state_kind_links'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_pose_workspace_rig_state'
                    AND column_name IN (
                        'workspace_ref',
                        'session_ref',
                        'rig_id',
                        'open',
                        'sort_order',
                        'active',
                        'dirty_calibration',
                        'panel_state',
                        'requested_by',
                        'created_at_utc',
                        'updated_at_utc'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 11
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_pose_workspace_rig_state'
                    AND constraint_name = 'chk_atelier_pose_workspace_rig_state_panel_state'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_pose_workspace_rig_state'
                    AND constraint_name = 'chk_atelier_pose_workspace_rig_state_session_ref'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_pose_workspace_rig_state'
                    AND constraint_name = 'chk_atelier_pose_workspace_rig_state_open_active'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM pg_indexes
                  WHERE schemaname = ANY(current_schemas(false))
                    AND tablename = 'atelier_pose_workspace_rig_state'
                    AND indexname = 'uq_atelier_pose_workspace_rig_state_active'
                    AND indexdef ILIKE '%workspace_ref, session_ref%'
                    AND indexdef ILIKE '%active%'
                    AND indexdef ILIKE '%open%'
              )
              AND EXISTS (
                  SELECT 1
                  FROM pg_indexes
                  WHERE schemaname = ANY(current_schemas(false))
                    AND tablename = 'atelier_pose_workspace_rig_state'
                    AND indexname = 'uq_atelier_pose_workspace_rig_state_open_order'
                    AND indexdef ILIKE '%workspace_ref, session_ref, sort_order%'
                    AND indexdef ILIKE '%open%'
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_identity_profile'
                    AND column_name IN (
                        'version',
                        'name',
                        'description',
                        'source_ref',
                        'crop_ref',
                        'artifact_ref',
                        'updated_at_utc',
                        'deleted_at_utc'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 8
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_identity_profile'
                    AND constraint_name = 'chk_atelier_identity_profile_version'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND to_regclass('atelier_identity_crop_artifact') IS NOT NULL
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_identity_crop_artifact'
                    AND column_name IN (
                        'crop_id',
                        'profile_id',
                        'profile_version',
                        'character_internal_id',
                        'source_ref',
                        'crop_box',
                        'landmarks',
                        'artifact_ref',
                        'manifest_ref',
                        'content_hash',
                        'byte_len',
                        'mime',
                        'width',
                        'height',
                        'manifest',
                        'created_by',
                        'created_at_utc'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 17
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_identity_crop_artifact'
                    AND constraint_name = 'chk_atelier_identity_crop_artifact_size'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_comfy_intake_output'
                    AND column_name = 'workflow_input_metadata'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_comfy_intake_output'
                    AND constraint_name = 'chk_atelier_comfy_intake_output_workflow_input_metadata_json'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND to_regclass('atelier_comfy_workflow_receipt') IS NOT NULL
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_comfy_workflow_receipt'
                    AND column_name IN (
                        'receipt_id',
                        'system_id',
                        'workflow_run_id',
                        'character_ref',
                        'workflow_spec_ref',
                        'workflow_json_ref',
                        'prompt_ref',
                        'all_refs',
                        'outputs',
                        'status',
                        'error_ref',
                        'evidence',
                        'receipt_json',
                        'created_at_utc',
                        'updated_at_utc'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 15
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_comfy_workflow_receipt'
                    AND constraint_name = 'chk_atelier_comfy_workflow_receipt_error'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_comfy_workflow_receipt'
                    AND constraint_name = 'chk_atelier_comfy_workflow_receipt_character_ref'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND to_regclass('atelier_comfy_output_registration_failure') IS NOT NULL
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_comfy_output_registration_failure'
                    AND column_name IN (
                        'failure_id',
                        'workflow_run_id',
                        'node_execution_id',
                        'attempted_registration_id',
                        'source_node_instance_id',
                        'source_output_slot',
                        'media_kind',
                        'mime',
                        'artifact_ref',
                        'artifact_manifest_ref',
                        'content_hash',
                        'routing_intent',
                        'parent_artifact_ref',
                        'prompt_json_ref',
                        'graph_hash',
                        'seed',
                        'workflow_input_metadata',
                        'failure_stage',
                        'failure_reason',
                        'evidence',
                        'status',
                        'retry_count',
                        'resolved_intake_output_id',
                        'created_at_utc',
                        'updated_at_utc'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 25
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_comfy_output_registration_failure'
                    AND constraint_name = 'chk_atelier_comfy_output_registration_failure_resolution'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND to_regclass('atelier_comfy_workflow_spec') IS NOT NULL
              AND to_regclass('atelier_comfy_version_metadata') IS NOT NULL
              AND to_regclass('atelier_comfy_job') IS NOT NULL
              AND to_regclass('atelier_comfy_diagnostic_bundle') IS NOT NULL
              AND to_regclass('atelier_diagnostics_validation_matrix') IS NOT NULL
              AND to_regclass('atelier_diagnostics_error_taxonomy') IS NOT NULL
              AND to_regclass('atelier_diagnostics_prompt_response_matrix') IS NOT NULL
              AND to_regclass('atelier_command_log') IS NOT NULL
              AND to_regclass('atelier_diagnostics_session') IS NOT NULL
              AND to_regclass('atelier_model_config') IS NOT NULL
              AND to_regclass('atelier_model_apply') IS NOT NULL
              AND to_regclass('atelier_synthetic_input_guard') IS NOT NULL
              AND to_regclass('atelier_work_state_projection') IS NOT NULL
              AND to_regclass('atelier_dcc_panel_projection') IS NOT NULL
              AND to_regclass('atelier_screenshot_artifact_storage') IS NOT NULL
              AND to_regclass('atelier_spec_drift_finding') IS NOT NULL
              AND to_regclass('atelier_command_corpus_parity_report') IS NOT NULL
              AND to_regclass('atelier_sheet_parse_snapshot') IS NOT NULL
              AND to_regclass('atelier_bulk_operation_receipt') IS NOT NULL
              AND to_regclass('atelier_trash_marker') IS NOT NULL
              AND to_regclass('atelier_similarity_rebuild_job') IS NOT NULL
              AND to_regclass('atelier_ai_tag_suggestion') IS NOT NULL
              AND to_regclass('atelier_media_sidecar') IS NOT NULL
              AND to_regclass('atelier_filesystem_health_check') IS NOT NULL
              AND to_regclass('atelier_filesystem_health_finding') IS NOT NULL
              AND to_regclass('atelier_image_import_request') IS NOT NULL
              AND to_regclass('atelier_media_derivative') IS NOT NULL
              AND to_regclass('atelier_media_review_metadata') IS NOT NULL
              AND to_regclass('atelier_stealth_capture') IS NOT NULL
              AND to_regclass('atelier_source_evidence_record') IS NOT NULL
              AND to_regclass('atelier_anchor_verification_record') IS NOT NULL
              AND to_regclass('atelier_model_manual_row_merge') IS NOT NULL
              AND to_regclass('atelier_model_manual_drift_guard') IS NOT NULL
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints tc
                  JOIN information_schema.key_column_usage kcu
                    ON tc.constraint_name = kcu.constraint_name
                   AND tc.table_schema = kcu.table_schema
                   AND tc.table_name = kcu.table_name
                  WHERE tc.table_schema = ANY(current_schemas(false))
                    AND tc.table_name = 'atelier_source_evidence_record'
                    AND tc.constraint_type = 'PRIMARY KEY'
                  GROUP BY tc.constraint_name
                  HAVING array_agg(kcu.column_name::text ORDER BY kcu.ordinal_position)
                         = ARRAY['matrix_id'::text, 'source_id'::text]
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints tc
                  JOIN information_schema.key_column_usage kcu
                    ON tc.constraint_name = kcu.constraint_name
                   AND tc.table_schema = kcu.table_schema
                   AND tc.table_name = kcu.table_name
                  WHERE tc.table_schema = ANY(current_schemas(false))
                    AND tc.table_name = 'atelier_anchor_verification_record'
                    AND tc.constraint_type = 'PRIMARY KEY'
                  GROUP BY tc.constraint_name
                  HAVING array_agg(kcu.column_name::text ORDER BY kcu.ordinal_position)
                         = ARRAY['matrix_id'::text, 'anchor_id'::text]
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints tc
                  JOIN information_schema.key_column_usage kcu
                    ON tc.constraint_name = kcu.constraint_name
                   AND tc.table_schema = kcu.table_schema
                   AND tc.table_name = kcu.table_name
                  WHERE tc.table_schema = ANY(current_schemas(false))
                    AND tc.table_name = 'atelier_anchor_verification_record'
                    AND tc.constraint_type = 'FOREIGN KEY'
                  GROUP BY tc.constraint_name
                  HAVING array_agg(kcu.column_name::text ORDER BY kcu.ordinal_position)
                         = ARRAY['matrix_id'::text, 'source_id'::text]
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_media_asset'
                    AND column_name = 'artifact_manifest'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_media_asset'
                    AND column_name = 'retention_class'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_event'
                    AND column_name = 'kernel_event_id'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_event'
                    AND column_name = 'kernel_event_sequence'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_preference'
                    AND column_name = 'revision'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_preference'
                    AND column_name = 'source'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_md_download_session'
                    AND column_name = 'protocol_id'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_md_download_session'
                    AND column_name = 'capability_profile_id'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_md_download_session'
                    AND column_name = 'capability_grant_ref'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_media_source_provenance_ref'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_media_source_provenance_ref'
                    AND constraint_name = 'chk_atelier_media_source_provenance_ref_trimmed_nonempty'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_intake_batch'
                    AND column_name = 'source_ref'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_intake_batch'
                    AND column_name = 'mode'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_intake_batch'
                    AND column_name = 'resume_cursor'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_intake_batch'
                    AND column_name = 'resumed_at_utc'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_intake_batch'
                    AND constraint_name = 'chk_atelier_intake_batch_status'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_intake_item_rejection_audit'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_intake_item_rejection_audit'
                    AND constraint_name = 'fk_atelier_intake_rejection_audit_item_batch'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_intake_item'
                    AND constraint_name = 'chk_atelier_intake_item_lane'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_intake_batch'
                    AND column_name = 'profile_mode'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_intake_batch'
                    AND column_name = 'target_character_id'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_intake_batch'
                    AND column_name = 'target_sheet_version_id'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_intake_batch'
                    AND column_name = 'target_collection_id'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_intake_batch'
                    AND constraint_name = 'chk_atelier_intake_profile_mode'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_intake_batch'
                    AND constraint_name = 'chk_atelier_intake_profile_targets'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_export_intake_link'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_media_asset_tag'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_collection_metadata_application'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_contact_sheet_svg_artifact'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_contact_sheet_raster_export_plan'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_character_document'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_character_document_version'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_story_card'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_story_beat'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM pg_trigger
                  WHERE tgname = 'trg_atelier_story_card_requires_story_document'
                    AND NOT tgisinternal
              )
              AND EXISTS (
                  SELECT 1
                  FROM pg_trigger
                  WHERE tgname = 'trg_atelier_story_beat_card_matches_story_document'
                    AND NOT tgisinternal
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_character_script'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_character_script'
                    AND constraint_name = 'chk_atelier_character_script_data_only_authority'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_character_script'
                    AND constraint_name = 'chk_atelier_character_script_provenance_refs_string_array'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_character_script'
                    AND constraint_name = 'chk_atelier_character_script_usage_refs_string_array'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_character_script'
                    AND constraint_name = 'chk_atelier_character_script_ref_whitespace_guard_v2'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_bracket_link_projection'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_bracket_link_projection'
                    AND constraint_name = 'chk_atelier_bracket_link_target_kind'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_bracket_link_projection'
                    AND constraint_name = 'chk_atelier_bracket_link_text_whitespace_guard_v2'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.triggers
                  WHERE event_object_table = 'atelier_bracket_link_projection'
                    AND trigger_name = 'trg_atelier_bracket_link_projection_guard'
                    AND trigger_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_bracket_link_projection'
                    AND constraint_name = 'chk_atelier_bracket_link_projection_guard_v7'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_character_relationship'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND to_regclass('atelier_character_relationship_graph_projection') IS NOT NULL
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_character_relationship_graph_projection'
                    AND column_name IN (
                        'edge_id',
                        'source_character_id',
                        'target_character_id',
                        'relationship_kind',
                        'label',
                        'notes',
                        'updated_at_utc'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 7
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_character_relationship'
                    AND constraint_name = 'chk_atelier_character_relationship_distinct_endpoints'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_character_relationship'
                    AND constraint_name = 'uq_atelier_character_relationship_edge'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_character_relationship'
                    AND constraint_name = 'chk_atelier_character_relationship_kind_trimmed'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_character_relationship'
                    AND constraint_name = 'chk_atelier_character_relationship_label_trimmed'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_character_relationship'
                    AND constraint_name = 'chk_atelier_character_relationship_notes_trimmed'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_saved_search'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND to_regclass('atelier_saved_search_retrieval_projection') IS NOT NULL
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_saved_search_retrieval_projection'
                    AND column_name = 'rating'
                    AND data_type = 'smallint'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_name = 'atelier_saved_search_retrieval_projection'
                    AND column_name IN (
                        'saved_search_id',
                        'asset_id',
                        'content_hash',
                        'artifact_ref',
                        'jump_target',
                        'tags_json',
                        'favorite',
                        'rating',
                        'matched_color_hex',
                        'view_mode',
                        'content_tier'
                    )
                    AND table_schema = ANY(current_schemas(false))
                  HAVING COUNT(DISTINCT column_name) = 11
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_saved_search'
                    AND constraint_name = 'chk_atelier_saved_search_scope'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_saved_search'
                    AND constraint_name = 'chk_atelier_saved_search_view_mode'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_saved_search'
                    AND constraint_name = 'chk_atelier_saved_search_color_hex'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_web_portfolio_export_request'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_web_portfolio_export_result'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_web_portfolio_export_request'
                    AND constraint_name = 'chk_atelier_web_portfolio_request_slug'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_web_portfolio_export_request'
                    AND constraint_name = 'chk_atelier_web_portfolio_request_status'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_web_portfolio_export_result'
                    AND constraint_name = 'chk_atelier_web_portfolio_result_artifact_ref'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_web_portfolio_export_result'
                    AND constraint_name = 'chk_atelier_web_portfolio_result_manifest_contract'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_backup_manifest'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_backup_restore_preflight'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_reset_operation'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_orphan_manifest'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_orphan_manifest_item'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_state_probe_catalog_entry'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_state_probe_catalog_entry'
                    AND constraint_name = 'chk_atelier_state_probe_read_model'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_state_probe_catalog_entry'
                    AND constraint_name = 'chk_atelier_state_probe_required_pre_visual'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_state_probe_catalog_entry'
                    AND constraint_name = 'chk_atelier_state_probe_fields_object'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_action_receipt'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_backup_manifest'
                    AND constraint_name = 'chk_atelier_backup_manifest_json_contract'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_backup_restore_preflight'
                    AND constraint_name = 'chk_atelier_backup_restore_preflight_status'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_backup_restore_preflight'
                    AND constraint_name = 'chk_atelier_backup_restore_preflight_refusal'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_moodboard'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_moodboard'
                    AND constraint_name = 'chk_atelier_moodboard_required_structure'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.triggers
                  WHERE event_object_table = 'atelier_moodboard'
                    AND trigger_name = 'trg_atelier_moodboard_guard'
                    AND trigger_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.triggers
                  WHERE event_object_table = 'atelier_character_document'
                    AND trigger_name = 'trg_atelier_moodboard_document_guard'
                    AND trigger_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.triggers
                  WHERE event_object_table = 'atelier_character_document_version'
                    AND trigger_name = 'trg_atelier_moodboard_version_guard'
                    AND trigger_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_moodboard_operation_receipt'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.triggers
                  WHERE event_object_table = 'atelier_moodboard_operation_receipt'
                    AND trigger_name = 'trg_atelier_moodboard_operation_receipt_guard'
                    AND trigger_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.tables
                  WHERE table_name = 'atelier_moodboard_export_request'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.triggers
                  WHERE event_object_table = 'atelier_moodboard_export_request'
                    AND trigger_name = 'trg_atelier_moodboard_export_request_guard'
                    AND trigger_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_moodboard_operation_receipt'
                    AND constraint_name = 'chk_atelier_moodboard_operation_receipt_contract_v2'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_moodboard_export_request'
                    AND constraint_name = 'chk_atelier_moodboard_export_manifest_contract_v2'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_moodboard_export_request'
                    AND constraint_name = 'chk_atelier_moodboard_export_receipt_contract_v2'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_moodboard_operation_receipt'
                    AND constraint_name = 'chk_atelier_moodboard_operation_receipt_contract_v3'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_moodboard_export_request'
                    AND constraint_name = 'chk_atelier_moodboard_export_manifest_contract_v3'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.table_constraints
                  WHERE table_name = 'atelier_moodboard_export_request'
                    AND constraint_name = 'chk_atelier_moodboard_export_receipt_contract_v3'
                    AND table_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.triggers
                  WHERE event_object_table = 'atelier_moodboard_operation_receipt'
                    AND trigger_name = 'trg_atelier_moodboard_operation_contract_guard_v4'
                    AND trigger_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.triggers
                  WHERE event_object_table = 'atelier_moodboard_export_request'
                    AND trigger_name = 'trg_atelier_moodboard_export_contract_guard_v4'
                    AND trigger_schema = ANY(current_schemas(false))
              )
              AND EXISTS (
                  SELECT 1
                  FROM information_schema.triggers
                  WHERE event_object_table = 'atelier_moodboard_export_request'
                    AND trigger_name = 'trg_atelier_moodboard_export_counts_contract_guard_v5'
                    AND trigger_schema = ANY(current_schemas(false))
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM information_schema.columns
                  WHERE table_schema = ANY(current_schemas(false))
                    AND column_default IS NOT NULL
                    AND (
                      (table_name = 'atelier_stealth_window' AND column_name = 'window_ref_id')
                      OR (table_name = 'atelier_stealth_ref' AND column_name = 'ref_id')
                      OR (table_name = 'atelier_stealth_capture' AND column_name = 'capture_id')
                    )
              )"#,
        )
        .fetch_one(&mut *tx)
        .await?;
        if ready_after_lock {
            tx.commit().await?;
            self.repair_contact_sheet_manifest_schema_namespace()
                .await?;
            self.repair_media_asset_artifact_manifests().await?;
            return Ok(());
        }

        sqlx::raw_sql(include_str!("../../migrations/0030_atelier_foundation.sql"))
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(include_str!("../../migrations/0031_atelier_core_data.sql"))
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0032_atelier_pose_diagnostics.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0033_atelier_event_ledger_projection.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0034_atelier_preference_metadata.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0035_atelier_stealth_uuid_v7_bound_ids.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0036_atelier_downloader_capability_grants.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0037_atelier_sheet_parser_ast.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0038_atelier_contact_sheet_schema_namespace.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0039_atelier_bulk_operation_receipts.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0040_atelier_media_artifact_manifest.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0041_atelier_source_evidence_matrix.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0042_atelier_source_evidence_matrix_scope.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0043_atelier_media_review_metadata.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0044_atelier_media_derivatives.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0045_atelier_similarity_rebuild_jobs.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0046_atelier_ai_tag_suggestions.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0047_atelier_media_sidecars.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0048_atelier_filesystem_health.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0049_atelier_image_import.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0050_atelier_media_source_provenance_refs.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0080_atelier_media_source_provenance_ref_guards.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0051_atelier_intake_batch_resume.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0052_atelier_intake_item_lifecycle.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0081_atelier_intake_rejection_audit_item_batch_fk.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0053_atelier_intake_profile_targets.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0054_atelier_export_intake_links.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0055_atelier_collection_metadata_application.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0056_atelier_contact_sheet_svg_artifact.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0057_atelier_contact_sheet_raster_export_plan.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0058_atelier_character_documents.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0059_atelier_story_cards_beats.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0060_atelier_story_card_beat_guards.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0061_atelier_character_scripts.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0062_atelier_character_script_ref_guards.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0063_atelier_character_script_ref_whitespace_guard.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0064_atelier_bracket_links.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0065_atelier_bracket_link_text_guards.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0066_atelier_bracket_link_projection_guards.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0067_atelier_bracket_link_projection_rebuildability_guard.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0068_atelier_bracket_link_projection_v4_cleanup.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0069_atelier_bracket_link_projection_current_label_guard.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0070_atelier_bracket_link_projection_strict_current_guard.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0071_atelier_bracket_link_projection_rust_marker_parity.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0072_atelier_moodboard_schema_layer_model.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0073_atelier_moodboard_direct_sql_guards.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0074_atelier_moodboard_operations_exports.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0075_atelier_moodboard_operation_export_contract_guards.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0076_atelier_moodboard_contract_null_strict_guards.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0077_atelier_moodboard_full_contract_triggers.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0078_atelier_moodboard_export_counts_guard.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0082_atelier_character_relationships.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0083_atelier_saved_searches.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0084_atelier_web_portfolio_exports.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0085_atelier_backup_manifests.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0089_atelier_reset_orphan_adoption.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0090_atelier_pose_sidecars.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0091_atelier_pose_sidecar_contract.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0092_atelier_pose_context_state.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0093_atelier_pose_workspace_rig_state.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0094_atelier_pose_workspace_rig_state_session_open.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0095_atelier_pose_rig_provenance.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0096_atelier_pose_rig_error_reason.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0097_atelier_pose_sidecar_manifest_role.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0098_atelier_pose_calibration_typed_data.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0099_atelier_identity_profile_record_fields.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0100_atelier_identity_crop_artifact.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0101_atelier_comfy_identity_workflow_metadata.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0102_atelier_comfy_workflow_receipt.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0103_atelier_comfy_output_registration_failure.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0104_atelier_comfy_workflow_history_stats.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0105_atelier_pose_deferred_feature.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0106_atelier_comfy_workflow_spec.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0107_atelier_comfy_version_metadata.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0108_atelier_comfy_job_queue.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0109_atelier_comfy_diagnostic_bundle.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0111_atelier_diagnostics_validation_matrix.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0112_atelier_diagnostics_typed_surfaces.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0113_atelier_command_log_session_heartbeat.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0114_atelier_model_config_apply.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0115_atelier_diagnostics_projections.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0116_atelier_dcc_flight_recorder.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0086_atelier_state_probe_catalog.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0088_atelier_state_probe_catalog_guards.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0087_atelier_action_receipts.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0122_atelier_model_manual_merge_drift.sql"
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../migrations/0129_atelier_visual_steer_retention.sql"
        ))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.repair_contact_sheet_manifest_schema_namespace()
            .await?;
        self.repair_media_asset_artifact_manifests().await?;
        Ok(())
    }

    async fn repair_contact_sheet_manifest_schema_namespace(&self) -> AtelierResult<()> {
        let table_exists: bool =
            sqlx::query_scalar("SELECT to_regclass('atelier_contact_sheet') IS NOT NULL")
                .fetch_one(&self.pool)
                .await?;
        if !table_exists {
            return Ok(());
        }

        let legacy_schema = collections::legacy_contact_sheet_manifest_schema();
        sqlx::query(
            r#"UPDATE atelier_contact_sheet
               SET manifest = jsonb_set(
                   COALESCE(manifest, '{}'::jsonb),
                   '{schema}',
                   to_jsonb($2::text),
                   true
               )
               WHERE manifest->>'schema' = $1"#,
        )
        .bind(legacy_schema)
        .bind(collections::CONTACT_SHEET_MANIFEST_SCHEMA)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Append an atelier domain event to the event ledger table (MT-005).
    pub async fn record_event(
        &self,
        event_family: &str,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: serde_json::Value,
    ) -> AtelierResult<()> {
        let mut tx = self.pool.begin().await?;
        if let Err(err) = self
            .record_event_in_tx(&mut tx, event_family, aggregate_type, aggregate_id, payload)
            .await
        {
            tx.rollback().await?;
            return Err(err);
        }
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn record_event_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event_family: &str,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: serde_json::Value,
    ) -> AtelierResult<()> {
        let atelier_event_id = Uuid::now_v7();
        let run_id = format!("atelier-domain-event:{atelier_event_id}");
        let safe_payload = sanitize_atelier_event_payload(payload);
        let kernel_payload = serde_json::json!({
            "atelier_event_id": atelier_event_id,
            "event_family": event_family,
            "aggregate_type": aggregate_type,
            "aggregate_id": aggregate_id,
            "atelier_payload": safe_payload.clone(),
        });
        let event = NewKernelEvent::builder(
            run_id.clone(),
            run_id,
            KernelEventType::AtelierDomainEventRecorded,
            KernelActor::System("atelier".to_string()),
        )
        .aggregate(aggregate_type, aggregate_id)
        .idempotency_key(format!("atelier-event:{atelier_event_id}"))
        .source_component("atelier")
        .payload(kernel_payload)
        .build()
        .map_err(|err| AtelierError::EventLedger(err.to_string()))?;
        let kernel_event = KernelEvent::from_new(event.clone());
        let payload_json = serde_json::to_string(&event.payload)
            .map_err(|err| AtelierError::EventLedger(err.to_string()))?;
        let row = sqlx::query(
            r#"
            WITH inserted AS (
                INSERT INTO kernel_event_ledger (
                    event_id,
                    event_version,
                    kernel_task_run_id,
                    session_run_id,
                    aggregate_type,
                    aggregate_id,
                    idempotency_key,
                    event_type,
                    actor_kind,
                    actor_id,
                    causation_id,
                    correlation_id,
                    payload_hash,
                    source_component,
                    payload,
                    created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15::jsonb, $16)
                ON CONFLICT (idempotency_key) DO NOTHING
                RETURNING event_id, event_sequence
            )
            SELECT event_id, event_sequence FROM inserted
            UNION ALL
            SELECT event_id, event_sequence
            FROM kernel_event_ledger
            WHERE idempotency_key = $7
            LIMIT 1"#,
        )
        .bind(&kernel_event.event_id)
        .bind(&event.event_version)
        .bind(&event.kernel_task_run_id)
        .bind(&event.session_run_id)
        .bind(&event.aggregate_type)
        .bind(&event.aggregate_id)
        .bind(&event.idempotency_key)
        .bind(event.event_type.as_str())
        .bind(event.actor.actor_kind())
        .bind(event.actor.actor_id())
        .bind(event.causation_id.as_deref())
        .bind(event.correlation_id.as_deref())
        .bind(&event.payload_hash)
        .bind(&event.source_component)
        .bind(payload_json)
        .bind(kernel_event.created_at)
        .fetch_one(&mut **tx)
        .await
        .map_err(|err| AtelierError::EventLedger(err.to_string()))?;
        let kernel_event_id: Option<String> = Some(row.get("event_id"));
        let kernel_event_sequence: Option<i64> = Some(row.get("event_sequence"));

        sqlx::query(
            r#"INSERT INTO atelier_event (
                   event_id,
                   event_family,
                   aggregate_type,
                   aggregate_id,
                   kernel_event_id,
                   kernel_event_sequence,
                   payload
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(atelier_event_id)
        .bind(event_family)
        .bind(aggregate_type)
        .bind(aggregate_id)
        .bind(kernel_event_id.clone())
        .bind(kernel_event_sequence)
        .bind(safe_payload.clone())
        .execute(&mut **tx)
        .await?;

        #[cfg(feature = "runtime-full")]
        {
            if let Some(flight_recorder) = &self.flight_recorder {
                let event = FlightRecorderEvent::new(
                    FlightRecorderEventType::Diagnostic,
                    FlightRecorderActor::System,
                    atelier_event_id,
                    serde_json::json!({
                        "diagnostic_id": "atelier_domain_event",
                        "authority_source": "postgres_event_ledger",
                        "projection_only": true,
                        "atelier_event_id": atelier_event_id,
                        "event_family": event_family,
                        "aggregate_type": aggregate_type,
                        "aggregate_id": aggregate_id,
                        "kernel_event_id": kernel_event_id,
                        "kernel_event_sequence": kernel_event_sequence,
                        "source_component": "atelier",
                        "payload": safe_payload,
                    }),
                )
                .with_actor_id("atelier")
                .with_workflow_id("atelier.domain_event");
                flight_recorder
                    .record_event(event)
                    .await
                    .map_err(|err| AtelierError::FlightRecorder(err.to_string()))?;
            }
        }
        Ok(())
    }

    /// Count events of a given family (used by tests / coverage proofs).
    pub async fn count_events(&self, event_family: &str) -> AtelierResult<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM atelier_event WHERE event_family = $1")
                .bind(event_family)
                .fetch_one(&self.pool)
                .await?;
        Ok(count)
    }

    /// Count events for one aggregate. Tests use this when the shared live
    /// PostgreSQL database may contain rows from prior runs.
    pub async fn count_events_for_aggregate(
        &self,
        event_family: &str,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> AtelierResult<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*)
               FROM atelier_event
               WHERE event_family = $1
                 AND aggregate_type = $2
                 AND aggregate_id = $3"#,
        )
        .bind(event_family)
        .bind(aggregate_type)
        .bind(aggregate_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    #[test]
    fn rejects_sqlite_urls() {
        assert!(assert_postgres_url("sqlite://./x.db").is_err());
        assert!(assert_postgres_url("/var/lib/handshake.sqlite").is_err());
        assert!(assert_postgres_url("foo.db").is_err());
    }

    #[test]
    fn accepts_postgres_urls() {
        assert!(assert_postgres_url("postgres://postgres@127.0.0.1:5544/handshake").is_ok());
        assert!(assert_postgres_url("postgresql://u:p@host/db").is_ok());
    }

    #[test]
    fn rejects_legacy_runtime_refs() {
        for value in [
            "electron://renderer/export",
            "sqlite://legacy/cache.db",
            "artifact://atelier/cache.sqlite",
            "artifact://atelier/cache.sqlite3",
            "exports/legacy-cache.db",
            "exports/legacy-cache.db#evidence",
            "artifact://atelier/cache.db/part",
            "ckc://legacy/record",
            "castkit://profile/1",
            "http://localhost:9000/intake",
            "http://user:pass@localhost:9000/intake",
            "http://u@127.0.0.1:9000/intake",
            "http://user:pass@[::1]:9000/intake",
            "artifact://operator@localhost/output",
            "artifact://127.0.0.1/output",
            "artifact://atelier/.GOV/out",
            "artifact://atelier/ckc",
            "artifact://atelier/castkit",
            "artifact://atelier/electron",
            "artifact://atelier/ckc.contact_sheet@1",
            "artifact://atelier/castkit.profile@1",
            "localhost:9000/intake",
            "127.0.0.1:9000/intake",
            "[::1]:9000/intake",
            "C:\\Users\\operator\\file.png",
            "\\\\server\\share\\file.png",
            "/home/operator/file.png",
            "file:///tmp/file.png",
            "evidence/file:///tmp/file.png",
            "evidence/C:\\Users\\operator\\file.png",
            "artifact://atelier/../out",
        ] {
            assert!(
                reject_legacy_runtime_ref("artifact_ref", value).is_err(),
                "{value} should be rejected"
            );
        }
    }

    #[test]
    fn accepts_handshake_native_portable_refs() {
        for value in [
            "artifact://atelier/media/018f7848-3a2e-76e2-93b1-3b4e4b5a6c7d",
            "artifact://.handshake/artifacts/L1/018f7848-3a2e-76e2-93b1-3b4e4b5a6c7d/payload",
            "manifest://atelier/comfy/018f7848-3a2e-76e2-93b1-3b4e4b5a6c7d",
            "source://operator/import/018f7848-3a2e-76e2-93b1-3b4e4b5a6c7d",
            "exports/contact-sheet/018f7848-3a2e-76e2-93b1-3b4e4b5a6c7d.json",
            "test://wp-kernel-005/mt-004",
        ] {
            assert!(
                reject_legacy_runtime_ref("artifact_ref", value).is_ok(),
                "{value} should be accepted"
            );
        }
    }
}

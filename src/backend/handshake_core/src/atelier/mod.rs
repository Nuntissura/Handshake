//! Atelier/Lens domain (WP-KERNEL-005 legacy source fold-in).
//!
//! Storage authority is Handshake's single embedded SurrealDB store (RocksDB
//! engine, in-process, namespace `handshake`, database `primary`) plus the
//! EventLedger + ArtifactStore + CRDT. SQLite and PostgreSQL are FORBIDDEN in
//! any form (runtime, tests, fixtures, cache, fallback); see
//! [`assert_embedded_store_backend`] (MT-004, MT-138) and the kernel
//! `no_sqlite_tripwire`.
//!
//! SURREALDB PORT (WP-KERNEL-012 MT-138). This file owns the domain SEAM — the
//! store handle, the schema-readiness gate, and the event-recording path that
//! every submodule writes through. The submodules are ported behind it, one
//! domain at a time; a submodule that has not been ported yet still names
//! `sqlx` and does not compile. The seam is deliberately ported first because
//! every one of those submodules reaches the database through it.
//!
//! Two shape changes travel with the port and are worth stating once, here,
//! rather than re-deriving them in thirty-four files:
//!
//! * There is no borrowed transaction handle. PostgreSQL let a caller open
//!   `pool.begin()` and pass `&mut Transaction` down; the embedded store
//!   exposes a scoped context instead ([`SurrealStorage::with_data_operation`]),
//!   and statements that must be atomic are written as one
//!   `BEGIN TRANSACTION; ...; COMMIT TRANSACTION;` string. So
//!   `record_event_in_tx(&mut tx, ..)` became
//!   [`AtelierStore::record_event_in_ctx`], which takes the scoped context.
//! * Schema is not replayed at runtime. `ensure_schema` used to execute ~150
//!   migration files under an advisory lock; the canonical SurrealDB schema is
//!   applied when the store opens, so the method is now a READINESS GATE that
//!   verifies the atelier tables are present and fails closed when they are
//!   not. The migration corpus survives as compile-time provenance
//!   (`storage::surreal::schema`), not as a runtime code path.
//!
//! Dropping the replay also dropped the two backfill repairs it called at the
//! end (`repair_contact_sheet_manifest_schema_namespace` here, and
//! `repair_media_asset_artifact_manifests` in [`media`]). Both rewrote rows
//! written by an OLDER build into the current shape. There is no upgrade path
//! from the removed PostgreSQL database into the embedded store, so no row a
//! backfill could target can exist: an atelier table is either empty or was
//! written by this build. Porting them would have produced two statements that
//! can only ever match zero rows.
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
use crate::storage::surreal::{SurrealDataContext, SurrealStorage, SurrealStorageError};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
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
    Database(#[from] SurrealStorageError),
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
    /// An invariant the store was supposed to uphold did not hold.
    ///
    /// This is for "cannot happen" outcomes that the type system does not rule
    /// out — a CREATE that returns no row, a required field missing from a row
    /// the schema declares as mandatory. It is deliberately distinct from
    /// [`Self::Database`] (the store reported a failure) and from
    /// [`Self::Validation`] (the caller sent something wrong): those are
    /// expected outcomes, and this one means the code and the schema disagree.
    #[error("atelier internal invariant violated: {0}")]
    Internal(String),
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
    use super::command_corpus::command_log_event_family;
    use super::command_corpus::diagnostics_event_family;
    use super::dcc_flight_recorder::dcc_flight_recorder_event_family;
    use super::documents::documents_event_family;
    use super::exports::export_event_family;
    use super::filesystem_health::filesystem_health_event_family;
    use super::intake::intake_event_family;
    use super::links::links_event_family;
    use super::model_manual_merge::model_manual_merge_event_family;
    use super::moodboards::moodboard_event_family;
    use super::pose::pose_event_family;
    use super::relationships::relationships_event_family;
    use super::scripts::scripts_event_family;
    use super::search::search_event_family;
    use super::settings::model_workflow_event_family;
    use super::settings::settings_event_family;
    use super::source_evidence::source_evidence_event_family;
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
        intake_event_family::INTAKE_ITEM_LOOM_PROJECTION_LINKED,
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

/// Runtime rejection of forbidden legacy source storage assumptions
/// (MT-004, widened by MT-138).
///
/// Handshake has exactly one database and it is embedded: the SurrealDB store
/// the backend opens in-process. There is no connection string to point
/// somewhere else, so the only thing a caller can still get wrong is to hand
/// atelier a legacy DSN inherited from configuration, a script, or an old
/// environment. Both legacy families are rejected by name rather than lumped
/// into one message, because the two mistakes have different fixes: a SQLite
/// path means someone reintroduced an embedded relational file, while a
/// PostgreSQL DSN means someone is still pointing at the removed server.
pub fn assert_embedded_store_backend(reference: &str) -> AtelierResult<()> {
    let normalized = reference.trim().to_ascii_lowercase();
    if normalized.starts_with("sqlite:")
        || normalized.ends_with(".sqlite")
        || normalized.ends_with(".sqlite3")
        || normalized.ends_with(".db")
    {
        return Err(AtelierError::ForbiddenStorage(
            "SQLite is forbidden in Handshake; atelier uses the embedded SurrealDB store"
                .to_string(),
        ));
    }
    if normalized.starts_with("postgres://") || normalized.starts_with("postgresql://") {
        return Err(AtelierError::ForbiddenStorage(
            "PostgreSQL has been removed from Handshake; atelier uses the embedded SurrealDB \
             store and takes no connection string"
                .to_string(),
        ));
    }
    Ok(())
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

/// The atelier half of one recorded domain event, carried from
/// [`AtelierStore::prepare_event`] to [`AtelierStore::finish_event`].
struct PreparedAtelierEvent {
    atelier_event_id: Uuid,
    event_family: String,
    aggregate_type: String,
    aggregate_id: String,
    safe_payload: serde_json::Value,
    bindings: RecordEventBindings,
}

/// Named parameters for [`atelier_event_sql!`].
///
/// Every field is a `$name` in that fragment; nothing is concatenated into the
/// query text.
///
/// A submodule that composes the fragment into its own statement calls
/// [`Self::with_domain`] to attach its own values under `$domain`, keeping the
/// two parameter namespaces separate.
#[derive(Clone, SurrealValue)]
pub(crate) struct RecordEventBindings {
    ledger_id: RecordId,
    kernel_event_id: String,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    kernel_aggregate_type: String,
    kernel_aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
    ledger_payload: JsonValue,
    created_at: Datetime,
    atelier_id: RecordId,
    atelier_event_uuid: SurrealUuid,
    atelier_event_id: String,
    event_family: String,
    atelier_payload: JsonValue,
}

impl RecordEventBindings {
    /// Attach a submodule's own bindings under `$domain`.
    pub(crate) fn with_domain<D>(self, domain: D) -> AtelierEventBindings<D>
    where
        D: SurrealValue,
    {
        AtelierEventBindings {
            domain,
            event: self,
        }
    }
}

/// A submodule's bindings plus the event bindings, for a statement that
/// composes [`atelier_event_sql!`].
///
/// The event fields are FLATTENED to the top level so the fragment can name
/// them directly (`$idempotency_key`, `$ledger_id`, ...), while the caller's
/// own values stay namespaced under `$domain`. Keeping the two namespaces
/// separate is what lets a submodule name a binding whatever suits it without
/// having to know which names the event fragment already occupies.
#[derive(Clone, SurrealValue)]
pub(crate) struct AtelierEventBindings<D>
where
    D: SurrealValue,
{
    domain: D,
    #[surreal(flatten)]
    event: RecordEventBindings,
}

/// The empty `$domain` used by [`AtelierStore::record_event`], which binds no
/// values of its own.
#[derive(Clone, SurrealValue)]
pub(crate) struct NoDomain {}

#[derive(SurrealValue)]
struct IdempotencyKeyBinding {
    idempotency_key: String,
}

/// The ledger identity an atelier event was stamped with.
#[derive(SurrealValue)]
struct RecordedLedgerRow {
    event_id: String,
    event_sequence: i64,
}

#[derive(SurrealValue)]
struct EventFamilyBinding {
    event_family: String,
}

#[derive(SurrealValue)]
struct AggregateCountBindings {
    event_family: String,
    aggregate_type: String,
    aggregate_id: String,
}

/// The SurrealQL that appends one atelier domain event.
///
/// This is a macro rather than a `const` so a caller can `concat!` it into the
/// middle of its OWN statement. That is what preserves the guarantee the
/// PostgreSQL code got from `pool.begin()`: the domain row and the event that
/// describes it are written by ONE statement, so they commit together or not
/// at all. A submodule that wrote the row in one round trip and the event in
/// another would have reintroduced the window where a crash leaves a mutation
/// with no event, which is exactly what the shared transaction existed to
/// prevent.
///
/// The fragment reads `$idempotency_key`, `$ledger_id`, `$atelier_id` and the
/// rest of [`RecordEventBindings`] from the top-level parameters, and defines
/// `$ledger_row` (the ledger identity: `event_id` + `event_sequence`) for the
/// caller to return or ignore. Caller-owned bindings live under `$domain`, so
/// the two namespaces cannot collide.
///
/// Idempotency is the PostgreSQL contract, preserved: replaying an event with
/// the same `idempotency_key` writes no second ledger row and still resolves
/// to the sequence the first write was given. `ON CONFLICT DO NOTHING` plus a
/// `UNION ALL` re-read expressed that; the guarded `IF $existing IS NONE` plus
/// the re-read expresses it here. `event_sequence` is deliberately not set by
/// this fragment - the schema defaults it from the `kernel_event_sequence`
/// SEQUENCE, which is what keeps ledger ordering monotonic and gap-free.
macro_rules! atelier_event_sql {
    () => {
        "LET $existing = (SELECT VALUE id FROM kernel_event_ledger \
           WHERE idempotency_key = $idempotency_key LIMIT 1)[0]; \
         IF $existing IS NONE { \
           CREATE $ledger_id CONTENT { \
             event_id: $kernel_event_id, \
             event_version: $event_version, \
             kernel_task_run_id: $kernel_task_run_id, \
             session_run_id: $session_run_id, \
             aggregate_type: $kernel_aggregate_type, \
             aggregate_id: $kernel_aggregate_id, \
             idempotency_key: $idempotency_key, \
             event_type: $event_type, \
             actor_kind: $actor_kind, \
             actor_id: $actor_id, \
             causation_id: $causation_id, \
             correlation_id: $correlation_id, \
             payload_hash: $payload_hash, \
             source_component: $source_component, \
             payload: $ledger_payload, \
             created_at: $created_at \
           }; \
         }; \
         LET $ledger_row = (SELECT event_id, event_sequence FROM kernel_event_ledger \
           WHERE idempotency_key = $idempotency_key LIMIT 1)[0]; \
         CREATE $atelier_id CONTENT { \
           event_id: $atelier_event_uuid, \
           event_family: $event_family, \
           aggregate_type: $kernel_aggregate_type, \
           aggregate_id: $kernel_aggregate_id, \
           kernel_event_id: $ledger_row.event_id, \
           kernel_event_sequence: $ledger_row.event_sequence, \
           payload: $atelier_payload \
         };"
    };
}

pub(crate) use atelier_event_sql;

/// Record an atelier event and nothing else.
const RECORD_EVENT_STATEMENT: &str = concat!(
    "RETURN { ",
    atelier_event_sql!(),
    " RETURN $ledger_row; };"
);

/// Atelier data store.
///
/// Holds Handshake's embedded SurrealDB store. The former `PgPool` is gone
/// along with the server it pooled connections to; there is nothing to pool
/// because the store runs inside this process.
#[derive(Clone)]
pub struct AtelierStore {
    store: SurrealStorage,
    #[cfg(feature = "runtime-full")]
    flight_recorder: Option<Arc<dyn FlightRecorder>>,
}

/// Every table the atelier domain reads or writes.
///
/// This list is the atelier half of the canonical SurrealDB schema and exists
/// so [`AtelierStore::ensure_schema`] can say WHICH table is missing instead of
/// failing somewhere deep in a submodule with a confusing empty result. It was
/// derived from the tables created by the migration files the old
/// `ensure_schema` replayed, so it covers the same surface that method used to
/// guarantee.
pub const ATELIER_TABLES: &[&str] = &[
    "atelier_action_receipt",
    "atelier_ai_tag_suggestion",
    "atelier_anchor_verification_record",
    "atelier_backup_manifest",
    "atelier_backup_restore_preflight",
    "atelier_bracket_link_projection",
    "atelier_bulk_operation_receipt",
    "atelier_caption_artifact",
    "atelier_character",
    "atelier_character_document",
    "atelier_character_document_version",
    "atelier_character_relationship",
    "atelier_character_script",
    "atelier_character_tag",
    "atelier_collection",
    "atelier_collection_item",
    "atelier_collection_metadata_application",
    "atelier_comfy_bridge_probe",
    "atelier_comfy_capability_registration",
    "atelier_comfy_capability_reject",
    "atelier_comfy_declared_output",
    "atelier_comfy_diagnostic_bundle",
    "atelier_comfy_fallback_marker",
    "atelier_comfy_intake_output",
    "atelier_comfy_job",
    "atelier_comfy_output_registration_failure",
    "atelier_comfy_version_metadata",
    "atelier_comfy_workflow_receipt",
    "atelier_comfy_workflow_spec",
    "atelier_command_corpus_blocked",
    "atelier_command_corpus_entry",
    "atelier_command_corpus_parity_report",
    "atelier_command_log",
    "atelier_contact_sheet",
    "atelier_contact_sheet_raster_export_plan",
    "atelier_contact_sheet_svg_artifact",
    "atelier_dcc_panel_projection",
    "atelier_dcc_workflow_panel_projection",
    "atelier_diagnostics_error_taxonomy",
    "atelier_diagnostics_prompt_response_matrix",
    "atelier_diagnostics_session",
    "atelier_diagnostics_validation_matrix",
    "atelier_event",
    "atelier_export_intake_link",
    "atelier_export_manifest_entry",
    "atelier_export_request",
    "atelier_export_result",
    "atelier_filesystem_health_check",
    "atelier_filesystem_health_finding",
    "atelier_fr_workflow_event",
    "atelier_handler_version_matrix",
    "atelier_identity_crop_artifact",
    "atelier_identity_profile",
    "atelier_image_import_request",
    "atelier_intake_batch",
    "atelier_intake_item",
    "atelier_intake_item_rejection_audit",
    "atelier_md_allowlist_policy",
    "atelier_md_auth_context",
    "atelier_md_checkpoint",
    "atelier_md_download_session",
    "atelier_md_item_state",
    "atelier_md_output_root",
    "atelier_md_session_receipt",
    "atelier_media_annotation",
    "atelier_media_asset",
    "atelier_media_asset_tag",
    "atelier_media_derivative",
    "atelier_media_probe_report",
    "atelier_media_review_metadata",
    "atelier_media_sidecar",
    "atelier_media_source_provenance_ref",
    "atelier_model_apply",
    "atelier_model_config",
    "atelier_model_manual_drift_guard",
    "atelier_model_manual_row_merge",
    "atelier_moodboard",
    "atelier_moodboard_export_request",
    "atelier_moodboard_operation_receipt",
    "atelier_orphan_manifest",
    "atelier_orphan_manifest_item",
    "atelier_pose_calibration",
    "atelier_pose_context_state",
    "atelier_pose_deferred_feature",
    "atelier_pose_head_pose",
    "atelier_pose_rig",
    "atelier_pose_sidecar",
    "atelier_pose_workspace_rig_state",
    "atelier_preference",
    "atelier_reset_operation",
    "atelier_saved_search",
    "atelier_screenshot_artifact_storage",
    "atelier_sheet_parse_snapshot",
    "atelier_sheet_version",
    "atelier_similarity_projection",
    "atelier_similarity_rebuild_job",
    "atelier_source_evidence_record",
    "atelier_sourcing_binding_decision",
    "atelier_sourcing_ingestion_receipt",
    "atelier_sourcing_spec",
    "atelier_spec_drift_finding",
    "atelier_state_probe_catalog_entry",
    "atelier_stealth_capture",
    "atelier_stealth_ref",
    "atelier_stealth_window",
    "atelier_story_beat",
    "atelier_story_card",
    "atelier_synthetic_input_guard",
    "atelier_tag",
    "atelier_tag_rule",
    "atelier_transcript_artifact",
    "atelier_transcript_receipt",
    "atelier_trash_marker",
    "atelier_version_mismatch_receipt",
    "atelier_visual_steer_feedback",
    "atelier_web_portfolio_export_request",
    "atelier_web_portfolio_export_result",
    "atelier_work_state_projection",
];

impl AtelierStore {
    pub fn new(store: SurrealStorage) -> Self {
        Self {
            store,
            #[cfg(feature = "runtime-full")]
            flight_recorder: None,
        }
    }

    #[cfg(feature = "runtime-full")]
    pub fn with_event_ledger(store: SurrealStorage, _event_ledger: Arc<dyn Database>) -> Self {
        Self {
            store,
            flight_recorder: None,
        }
    }

    #[cfg(feature = "runtime-full")]
    pub fn with_observability(
        store: SurrealStorage,
        _event_ledger: Arc<dyn Database>,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            store,
            flight_recorder: Some(flight_recorder),
        }
    }

    /// The embedded store this domain writes through.
    pub fn store(&self) -> &SurrealStorage {
        &self.store
    }

    /// Run one scoped data operation against the embedded store.
    ///
    /// Every atelier submodule reaches the database through here. The context
    /// is sealed and lease-bound: it cannot outlive the call, which is what
    /// lets the backend drain in-flight atelier work before it closes the
    /// store on shutdown.
    pub(crate) async fn with_data<T, F>(&self, operation: F) -> AtelierResult<T>
    where
        T: Send,
        F: for<'a> FnOnce(
            SurrealDataContext<'a>,
        ) -> crate::storage::surreal::SurrealOperation<'a, T>,
    {
        Ok(self.store.with_data_operation(operation).await?)
    }

    /// Verify the atelier schema is present, and fail closed when it is not.
    ///
    /// The PostgreSQL version of this method REPLAYED 88 migration files under
    /// a transaction-scoped advisory lock on every call, because each process
    /// had to converge a shared server it did not own. None of that applies to
    /// an embedded store: the canonical schema is applied once when the store
    /// opens (`storage::surreal::schema::bootstrap_schema`), inside a
    /// transaction that already refuses a divergent lineage, and this process
    /// is the only writer.
    ///
    /// So the method keeps its name and its guarantee to callers - after it
    /// returns `Ok`, the atelier tables exist - and drops the mechanism. It
    /// reports the first missing table by name rather than a bare boolean,
    /// because "atelier schema not ready" with no table name was the least
    /// actionable failure the old readiness check could produce.
    pub async fn ensure_schema(&self) -> AtelierResult<()> {
        let defined: Vec<String> = self
            .store
            .with_data_operation(|ctx| {
                Box::pin(async move {
                    ctx.query_values::<String, ()>(
                        "RETURN array::sort(object::keys((INFO FOR DB).tables));",
                        (),
                    )
                    .await
                })
            })
            .await?;
        let missing: Vec<&str> = ATELIER_TABLES
            .iter()
            .copied()
            .filter(|table| !defined.iter().any(|name| name == table))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        Err(AtelierError::ForbiddenStorage(format!(
            "atelier schema is not present in the embedded store: {} of {} tables are missing \
             (first missing: {}). The schema is applied when the store opens; a store that \
             opened without it must be rebuilt, not repaired here.",
            missing.len(),
            ATELIER_TABLES.len(),
            missing[0],
        )))
    }

    /// Append an atelier domain event to the event ledger (MT-005).
    ///
    /// Opens its own scoped store operation. A caller that is already inside
    /// one calls [`Self::record_event_in_ctx`] instead, so the event lands in
    /// the same statement as the mutation it describes.
    pub async fn record_event(
        &self,
        event_family: &str,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: serde_json::Value,
    ) -> AtelierResult<()> {
        let prepared = self.prepare_event(event_family, aggregate_type, aggregate_id, payload)?;
        let bindings = prepared.bindings.clone().with_domain(NoDomain {});
        let recorded: Option<RecordedLedgerRow> = self
            .store
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(RECORD_EVENT_STATEMENT, bindings).await })
            })
            .await?;
        self.finish_event(prepared, recorded).await
    }

    /// [`Self::record_event`] inside a caller's scoped store operation.
    ///
    /// This is the replacement for the former `record_event_in_tx`, which took
    /// a borrowed PostgreSQL `Transaction`. The embedded store has no such
    /// handle; atomicity comes from the statement itself, which is one
    /// `BEGIN TRANSACTION; ... COMMIT TRANSACTION;` round trip that writes the
    /// kernel ledger row and the atelier projection row together or writes
    /// neither.
    pub(crate) async fn record_event_in_ctx(
        &self,
        ctx: &SurrealDataContext<'_>,
        event_family: &str,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: serde_json::Value,
    ) -> AtelierResult<()> {
        let prepared = self.prepare_event(event_family, aggregate_type, aggregate_id, payload)?;
        let recorded: Option<RecordedLedgerRow> = ctx
            .query_first(
                RECORD_EVENT_STATEMENT,
                prepared.bindings.clone().with_domain(NoDomain {}),
            )
            .await?;
        self.finish_event(prepared, recorded).await
    }

    /// Run one domain mutation and its event as a single atomic statement.
    ///
    /// This is the shape almost every atelier mutation wants, and the reason
    /// [`atelier_event_sql!`] is a macro. `statement` is the caller's own
    /// SurrealQL with the event fragment `concat!`-ed into it, so the domain
    /// row and its event are written by one statement and commit together -
    /// the guarantee `pool.begin()` used to provide.
    ///
    /// `domain` carries the caller's bindings; they land under `$domain` and
    /// cannot collide with the fragment's own parameter names. The statement
    /// is expected to `RETURN` whatever the caller needs back, typically the
    /// row it just wrote.
    ///
    /// Returns `None` only if the statement returned nothing, which for a
    /// `CREATE`-shaped statement means the write did not happen; callers that
    /// require a row map that to [`AtelierError::Internal`].
    pub(crate) async fn write_with_event<R, D>(
        &self,
        statement: &'static str,
        domain: D,
        event_family: &str,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: serde_json::Value,
    ) -> AtelierResult<Option<R>>
    where
        R: surrealdb::types::SurrealValue + Send,
        D: SurrealValue + Send + 'static,
    {
        let prepared = self.prepare_event(event_family, aggregate_type, aggregate_id, payload)?;
        let bindings = prepared.bindings.clone().with_domain(domain);
        let written: Option<R> = self
            .store
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(statement, bindings).await })
            })
            .await?;
        // The event committed with the row above; this only mirrors it onto the
        // Flight Recorder, so the ledger identity is re-read rather than
        // threaded back through the caller's return type.
        let recorded: Option<RecordedLedgerRow> = self
            .store
            .with_data_operation({
                let key = prepared.bindings.idempotency_key.clone();
                move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(
                            "SELECT event_id, event_sequence FROM kernel_event_ledger \
                             WHERE idempotency_key = $idempotency_key LIMIT 1;",
                            IdempotencyKeyBinding {
                                idempotency_key: key,
                            },
                        )
                        .await
                    })
                }
            })
            .await?;
        self.finish_event(prepared, recorded).await?;
        Ok(written)
    }

    /// Build the kernel event and the bindings the write statement needs.
    ///
    /// Split out of the write so both entry points above produce identical
    /// rows; the only thing that differs between them is who owns the store
    /// operation.
    fn prepare_event(
        &self,
        event_family: &str,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: serde_json::Value,
    ) -> AtelierResult<PreparedAtelierEvent> {
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
        let ledger_payload = event.payload.as_object().cloned().ok_or_else(|| {
            AtelierError::EventLedger(
                "kernel event payload must be a JSON object to store in kernel_event_ledger"
                    .to_string(),
            )
        })?;
        let atelier_payload = safe_payload.as_object().cloned().ok_or_else(|| {
            AtelierError::EventLedger(
                "atelier event payload must be a JSON object to store in atelier_event".to_string(),
            )
        })?;
        Ok(PreparedAtelierEvent {
            atelier_event_id,
            event_family: event_family.to_owned(),
            aggregate_type: aggregate_type.to_owned(),
            aggregate_id: aggregate_id.to_owned(),
            safe_payload,
            bindings: RecordEventBindings {
                ledger_id: RecordId::new(
                    "kernel_event_ledger",
                    kernel_event.event_id.clone(),
                ),
                atelier_id: RecordId::new(
                    "atelier_event",
                    SurrealUuid::from(atelier_event_id),
                ),
                atelier_event_uuid: SurrealUuid::from(atelier_event_id),
                kernel_event_id: kernel_event.event_id.clone(),
                event_version: event.event_version.clone(),
                kernel_task_run_id: event.kernel_task_run_id.clone(),
                session_run_id: event.session_run_id.clone(),
                kernel_aggregate_type: event.aggregate_type.clone(),
                kernel_aggregate_id: event.aggregate_id.clone(),
                idempotency_key: event.idempotency_key.clone(),
                event_type: event.event_type.as_str().to_owned(),
                actor_kind: event.actor.actor_kind().to_owned(),
                actor_id: event.actor.actor_id().to_owned(),
                causation_id: event.causation_id.clone(),
                correlation_id: event.correlation_id.clone(),
                payload_hash: event.payload_hash.clone(),
                source_component: event.source_component.clone(),
                ledger_payload: JsonValue::Object(ledger_payload),
                created_at: Datetime::from(kernel_event.created_at),
                atelier_event_id: atelier_event_id.to_string(),
                event_family: event_family.to_owned(),
                atelier_payload: JsonValue::Object(atelier_payload),
            },
        })
    }

    /// Mirror a recorded event onto the Flight Recorder.
    ///
    /// The store write is authority and has already committed by the time this
    /// runs. A failure here is still returned rather than swallowed: a
    /// diagnostic surface that silently drops events is worse than a loud one.
    async fn finish_event(
        &self,
        prepared: PreparedAtelierEvent,
        recorded: Option<RecordedLedgerRow>,
    ) -> AtelierResult<()> {
        let recorded = recorded.ok_or_else(|| {
            AtelierError::EventLedger(
                "kernel_event_ledger write returned no row for the atelier domain event"
                    .to_string(),
            )
        })?;
        let PreparedAtelierEvent {
            atelier_event_id,
            event_family,
            aggregate_type,
            aggregate_id,
            safe_payload,
            ..
        } = prepared;

        #[cfg(feature = "runtime-full")]
        {
            if let Some(flight_recorder) = &self.flight_recorder {
                let event = FlightRecorderEvent::new(
                    FlightRecorderEventType::Diagnostic,
                    FlightRecorderActor::System,
                    atelier_event_id,
                    serde_json::json!({
                        "diagnostic_id": "atelier_domain_event",
                        "authority_source": "surreal_event_ledger",
                        "projection_only": true,
                        "atelier_event_id": atelier_event_id,
                        "event_family": event_family,
                        "aggregate_type": aggregate_type,
                        "aggregate_id": aggregate_id,
                        "kernel_event_id": recorded.event_id,
                        "kernel_event_sequence": recorded.event_sequence,
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
        #[cfg(not(feature = "runtime-full"))]
        {
            let _ = (
                atelier_event_id,
                event_family,
                aggregate_type,
                aggregate_id,
                safe_payload,
                recorded,
            );
        }
        Ok(())
    }

    /// Count events of a given family (used by tests / coverage proofs).
    pub async fn count_events(&self, event_family: &str) -> AtelierResult<i64> {
        let bindings = EventFamilyBinding {
            event_family: event_family.to_owned(),
        };
        let count: Option<i64> = self
            .store
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "RETURN count(SELECT id FROM atelier_event \
                         WHERE event_family = $event_family);",
                        bindings,
                    )
                    .await
                })
            })
            .await?;
        Ok(count.unwrap_or_default())
    }

    /// Count events for one aggregate. Tests use this when the live store may
    /// still hold rows from a prior run in the same data directory.
    pub async fn count_events_for_aggregate(
        &self,
        event_family: &str,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> AtelierResult<i64> {
        let bindings = AggregateCountBindings {
            event_family: event_family.to_owned(),
            aggregate_type: aggregate_type.to_owned(),
            aggregate_id: aggregate_id.to_owned(),
        };
        let count: Option<i64> = self
            .store
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "RETURN count(SELECT id FROM atelier_event \
                         WHERE event_family = $event_family \
                           AND aggregate_type = $aggregate_type \
                           AND aggregate_id = $aggregate_id);",
                        bindings,
                    )
                    .await
                })
            })
            .await?;
        Ok(count.unwrap_or_default())
    }
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    #[test]
    fn rejects_sqlite_urls() {
        assert!(assert_embedded_store_backend("sqlite://./x.db").is_err());
        assert!(assert_embedded_store_backend("/var/lib/handshake.sqlite").is_err());
        assert!(assert_embedded_store_backend("foo.db").is_err());
    }

    /// The inversion that came with MT-138: a PostgreSQL DSN used to be the ONLY
    /// accepted atelier storage reference and is now rejected outright, because
    /// the server it points at no longer exists in Handshake. Anything that
    /// still hands atelier one is misconfigured, not merely legacy.
    #[test]
    fn rejects_postgres_urls() {
        for dsn in [
            "postgres://postgres@127.0.0.1:5544/handshake",
            "postgresql://u:p@host/db",
        ] {
            let error = assert_embedded_store_backend(dsn)
                .expect_err("a PostgreSQL DSN is no longer valid atelier storage");
            assert!(
                matches!(error, AtelierError::ForbiddenStorage(_)),
                "a legacy DSN must be refused as forbidden storage, got {error:?}"
            );
        }
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

//! WP-KERNEL-005 atelier Core-Data: real PostgreSQL round-trip proofs for the
//! six folded-in submodules (intake / collections / search / exports /
//! annotation / settings). Run with a live DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_core_data_tests -- --nocapture
//!
//! No mocks: each test connects the actual `AtelierStore` to a real Postgres,
//! ensures the schema, exercises one submodule with REAL data, and asserts the
//! load-bearing invariants from the adversarial review. Tables persist between
//! runs, so all public ids / hashes / keys are made unique per run via
//! `Uuid::new_v4()` to avoid cross-run collisions. Only `handshake_core` +
//! `tokio` + `uuid` (+ std) are used; sqlx is used only for narrow persisted
//! event-payload inspection where the proof target is the stored row itself.

use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

mod atelier_pg_support;

use handshake_core::atelier::acceptance::AtelierAcceptanceConstraints;
use handshake_core::atelier::annotation::{AnnotationKind, NewMediaAnnotation};
use handshake_core::atelier::collections::{
    collections_event_family, CollectionMetadataApplicationRequest, NewCollection,
};
use handshake_core::atelier::documents::{
    documents_event_family, AppendCharacterDocumentVersion, CharacterDocumentType,
    NewCharacterDocument, NewStoryBeat, NewStoryCard,
};
use handshake_core::atelier::exports::{
    build_llm_evidence_pack_manifest, export_event_family, validate_llm_evidence_pack_manifest,
    BackupManifestFile, BackupRestorePreflightRequest, BackupRestorePreflightStatus,
    ContactSheetRasterExportFormat, ContactSheetRasterExportStatus, ExportFormat,
    LlmEvidencePackFile, LlmEvidencePackFileKind, LlmEvidenceSourceAnchor, ManifestItemKind,
    NewBackupManifest, NewExportRequest, NewWebPortfolioExportRequest, SharePackBuildRequest,
    SharePackSubsetSelector, SharePackUsageReadmeArtifact, WebPortfolioManifestItem,
    BACKUP_MANIFEST_SCHEMA_ID, LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID,
    WEB_PORTFOLIO_MANIFEST_SCHEMA_ID,
};
use handshake_core::atelier::intake::{
    intake_event_family, ApplyIntakeClassificationRequest, AtelierResetMode, AtelierResetRequest,
    BatchStatus, IntakeBatchMode, IntakeLane, IntakeProfileMode, NewIntakeBatch, NewIntakeItem,
    OrphanAdoptionRequest, OrphanAdoptionStatus,
};
use handshake_core::atelier::links::{links_event_family, BracketLinkTargetKind};
use handshake_core::atelier::moodboards::{
    moodboard_event_family, MoodboardExportFormat, MoodboardExportStatus, MoodboardOperationKind,
    NewMoodboardExportRequest, NewMoodboardOperation, NewMoodboardSnapshot, MOODBOARD_SCHEMA_ID,
};
use handshake_core::atelier::relationships::{
    relationships_event_family, NewCharacterRelationship, UpdateCharacterRelationship,
};
use handshake_core::atelier::scripts::{
    scripts_event_family, CharacterScriptAuthorityMode, NewCharacterScript,
};
use handshake_core::atelier::search::{
    search_event_family, AiTagSuggestionDecision, AiTagSuggestionStatus, LensContentTier,
    LensExtractionTier, LensSearchFilters, LensViewMode, MatchType, NewAiTagSuggestion,
    NewSavedSearch, NewTagRule, SavedSearchFilters, SavedSearchScope, SimilarityRebuildJobStatus,
    TagType,
};
use handshake_core::atelier::settings::{
    PreferenceScope, PreferenceType, PreferenceValueSource, RetentionDefaultPolicy, SetPreference,
};
use handshake_core::atelier::{
    event_family, AtelierStore, DeletionArchiveRequest, DeletionRestoreRequest, DeletionTargetKind,
    DeletionTargetRef, FilesystemHealthCheckRequest, FilesystemHealthFindingKind,
    MediaDerivativeGenerated, MediaDerivativeKind, MediaDerivativeRequest,
    MediaSidecarRelationKind, NewCharacter, NewMediaAsset, NewMediaSidecarRelation,
    NewSheetVersion, SetMediaSourceProvenanceRefs,
};
use sqlx::Row;
use uuid::Uuid;

const SIDECAR_VISIBILITY_HEALTH_LOCK_ID: i64 = 5_023_022;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

fn assert_portable_artifact_handle(field: &str, value: &str) {
    assert!(
        value.starts_with("artifact://.handshake/artifacts/"),
        "{field} must be an ArtifactStore handle, got {value}"
    );
    assert!(
        !value.contains(".GOV")
            && !value.contains('\\')
            && !value.contains(":\\")
            && !value.starts_with('/')
            && !value.starts_with("file:"),
        "{field} must not be a .GOV, drive-letter, or filesystem path, got {value}"
    );
}

async fn acquire_sidecar_visibility_health_lock(
    store: &AtelierStore,
) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut tx = store
        .pool()
        .begin()
        .await
        .expect("begin sidecar visibility health lock transaction");
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(SIDECAR_VISIBILITY_HEALTH_LOCK_ID)
        .execute(&mut *tx)
        .await
        .expect("acquire sidecar visibility health lock");
    tx
}

/// Materialize a fresh, run-unique media asset and return its `asset_id`.
async fn fresh_asset(store: &AtelierStore) -> Uuid {
    let artifact = atelier_pg_support::write_native_media_artifact(b"core-data-test-media");
    let asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash,
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("core-data-test".to_string()),
            artifact_ref: artifact.artifact_ref,
        })
        .await
        .expect("materialize media asset");
    asset.asset_id
}

fn artifact_payload_path(artifact: &atelier_pg_support::NativeMediaArtifact) -> PathBuf {
    artifact
        .workspace_root
        .join(".handshake")
        .join("artifacts")
        .join("L1")
        .join(artifact.artifact_id.to_string())
        .join("payload")
}

fn artifact_manifest_path(artifact: &atelier_pg_support::NativeMediaArtifact) -> PathBuf {
    artifact
        .workspace_root
        .join(".handshake")
        .join("artifacts")
        .join("L1")
        .join(artifact.artifact_id.to_string())
        .join("artifact.json")
}

fn quote_pg_ident(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

async fn sidecar_visibility_constraints(store: &AtelierStore) -> Vec<(String, String)> {
    sqlx::query(
        r#"SELECT conname, pg_get_constraintdef(oid) AS constraint_def
           FROM pg_constraint
           WHERE conrelid = 'atelier_media_sidecar'::regclass
             AND contype = 'c'
             AND (
                pg_get_constraintdef(oid) ILIKE '%hidden_from_gallery%'
                OR pg_get_constraintdef(oid) ILIKE '%searchable_by_relation%'
             )
           ORDER BY conname"#,
    )
    .fetch_all(store.pool())
    .await
    .expect("read sidecar visibility constraints")
    .iter()
    .map(|row| {
        (
            row.get::<String, _>("conname"),
            row.get::<String, _>("constraint_def"),
        )
    })
    .collect()
}

async fn drop_sidecar_visibility_constraints(
    store: &AtelierStore,
    constraints: &[(String, String)],
) {
    for (name, _) in constraints {
        sqlx::query(&format!(
            "ALTER TABLE atelier_media_sidecar DROP CONSTRAINT {}",
            quote_pg_ident(name)
        ))
        .execute(store.pool())
        .await
        .expect("drop sidecar visibility constraint for drift probe");
    }
}

async fn restore_sidecar_visibility_constraints(
    store: &AtelierStore,
    constraints: &[(String, String)],
    sidecar_id: Uuid,
) {
    sqlx::query(
        r#"UPDATE atelier_media_sidecar
           SET hidden_from_gallery = TRUE,
               searchable_by_relation = TRUE,
               updated_at_utc = NOW()
           WHERE sidecar_id = $1"#,
    )
    .bind(sidecar_id)
    .execute(store.pool())
    .await
    .expect("repair drifted sidecar row before restoring constraints");

    for (name, definition) in constraints {
        let check_sql = if definition.contains("hidden_from_gallery") {
            "CHECK (hidden_from_gallery = TRUE)"
        } else {
            "CHECK (searchable_by_relation = TRUE)"
        };
        sqlx::query(&format!(
            "ALTER TABLE atelier_media_sidecar ADD CONSTRAINT {} {}",
            quote_pg_ident(name),
            check_sql
        ))
        .execute(store.pool())
        .await
        .expect("restore sidecar visibility constraint after drift probe");
    }
}

fn similarity_png_bytes() -> Vec<u8> {
    let mut image = image::RgbaImage::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let pixel = if x < 8 {
                image::Rgba([255, 0, 0, 255])
            } else if y < 8 {
                image::Rgba([0, 0, 255, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            };
            image.put_pixel(x, y, pixel);
        }
    }
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("encode similarity fixture png");
    bytes
}

fn is_hex64(value: &str) -> bool {
    value.len() == 16 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

#[test]
fn atelier_parent_event_family_registry_includes_ai_tag_suggestion_lifecycle() {
    for family in [
        search_event_family::AI_TAG_SUGGESTION_RECORDED,
        search_event_family::AI_TAG_SUGGESTION_ACCEPTED,
        search_event_family::AI_TAG_SUGGESTION_REJECTED,
        search_event_family::AI_TAG_SUGGESTION_APPLIED,
    ] {
        assert!(
            event_family::ALL.contains(&family),
            "parent atelier event registry must expose {family}"
        );
    }
}

#[tokio::test]
async fn atelier_filesystem_health_records_diagnostics_without_resync_or_delete() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_filesystem_health_records_diagnostics_without_resync_or_delete: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let sidecar_health_lock = acquire_sidecar_visibility_health_lock(&store).await;
    assert!(
        event_family::ALL.contains(
            &handshake_core::atelier::filesystem_health::filesystem_health_event_family::CHECK_RECORDED,
        ),
        "filesystem health event family must be discoverable through the parent atelier registry"
    );

    let primary = fresh_asset(&store).await;
    let sidecar_asset = fresh_asset(&store).await;
    store
        .record_media_sidecar_relation(&NewMediaSidecarRelation {
            parent_asset_id: primary,
            sidecar_asset_id: sidecar_asset,
            relation_kind: MediaSidecarRelationKind::OpenPoseJson,
            created_by: "mt-023-operator".to_string(),
        })
        .await
        .expect("record sidecar relation for health check");

    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("mt-023-health-{}", Uuid::new_v4()),
            source_label: "mt-023-health".to_string(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("open health intake batch");
    let intake_item = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: format!("artifact://atelier/intake/{}", Uuid::new_v4()),
                file_name: "pending-original.png".to_string(),
                byte_len: 128,
                content_hash: Some(format!("sha256:{}", Uuid::new_v4().simple())),
            },
        )
        .await
        .expect("add pending intake item");

    let invalid_asset_id = Uuid::now_v7();
    let invalid_hash = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    sqlx::query(
        r#"INSERT INTO atelier_media_asset
             (asset_id, content_hash, mime, byte_len, source_provenance,
              artifact_ref, retention_class, artifact_manifest)
           VALUES ($1, $2, 'image/png', 64, 'legacy-import',
                   'artifact://.GOV/missing-original',
                   'atelier.media.original.retained',
                   jsonb_build_object(
                     'schema', $3::text,
                     'asset_id', $1::text,
                     'content_hash', $2::text,
                     'mime', 'image/png',
                     'byte_len', 64,
                     'size_bytes', 64,
                     'validation_state', 'invalid_legacy_artifact_ref',
                     'artifact_store', jsonb_build_object(
                       'status', 'unresolved',
                       'reason', 'legacy artifact_ref is not a native ArtifactStore payload handle'
                     )
                   ))"#,
    )
    .bind(invalid_asset_id)
    .bind(&invalid_hash)
    .bind(handshake_core::atelier::media::MEDIA_ARTIFACT_MANIFEST_SCHEMA)
    .execute(store.pool())
    .await
    .expect("insert intentionally invalid media row for health diagnostics");

    let report = store
        .run_filesystem_health_check(&FilesystemHealthCheckRequest {
            requested_by: "mt-023-operator".to_string(),
            scope_label: Some("mt-023-gallery".to_string()),
        })
        .await
        .expect("run filesystem health check");
    let finding_kinds: Vec<FilesystemHealthFindingKind> = report
        .findings
        .iter()
        .map(|finding| finding.finding_kind)
        .collect();
    assert!(finding_kinds.contains(&FilesystemHealthFindingKind::MissingOriginal));
    assert!(finding_kinds.contains(&FilesystemHealthFindingKind::MissingThumbnail));
    assert!(finding_kinds.contains(&FilesystemHealthFindingKind::InboxPending));
    assert!(finding_kinds.contains(&FilesystemHealthFindingKind::UntrackedOriginal));
    assert_eq!(
        report.check.summary["auto_resync"],
        serde_json::json!(false),
        "health diagnostics must not auto-resync"
    );
    assert_eq!(
        report.check.summary["auto_delete"],
        serde_json::json!(false),
        "health diagnostics must not auto-delete"
    );
    assert!(
        report.check.summary["sidecars_checked_count"]
            .as_i64()
            .is_some_and(|count| count >= 1),
        "sidecar matrix must be included in the health check"
    );
    assert!(
        !finding_kinds.contains(&FilesystemHealthFindingKind::SidecarVisibilityAnomaly),
        "valid hidden/searchable sidecars should not be reported as anomalies"
    );

    let persisted_findings = store
        .list_filesystem_health_findings(report.check.check_id)
        .await
        .expect("list persisted health findings");
    assert_eq!(persisted_findings.len(), report.findings.len());

    let intake_after = store
        .get_intake_item(batch.batch_id, &intake_item.source_path)
        .await
        .expect("read intake item after health check")
        .expect("intake item still exists");
    assert_eq!(
        intake_after.lane,
        IntakeLane::Pending,
        "health diagnostics must not classify or delete inbox-pending items"
    );
    let invalid_exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM atelier_media_asset WHERE asset_id = $1)")
            .bind(invalid_asset_id)
            .fetch_one(store.pool())
            .await
            .expect("check invalid asset still exists");
    assert!(
        invalid_exists,
        "health diagnostics must not delete missing-original evidence rows"
    );
    let thumbnail_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM atelier_media_derivative WHERE asset_id = $1")
            .bind(primary)
            .fetch_one(store.pool())
            .await
            .expect("count derivatives after health check");
    assert_eq!(
        thumbnail_count, 0,
        "health diagnostics must not create or resync thumbnails"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                handshake_core::atelier::filesystem_health::filesystem_health_event_family::CHECK_RECORDED,
                "atelier_filesystem_health_check",
                &report.check.check_id.to_string(),
            )
            .await
            .expect("count filesystem health event"),
        1
    );
    sidecar_health_lock
        .commit()
        .await
        .expect("release sidecar visibility health lock");
}

#[tokio::test]
async fn atelier_filesystem_health_reports_sidecar_visibility_anomaly_from_catalog_drift() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_filesystem_health_reports_sidecar_visibility_anomaly_from_catalog_drift: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let sidecar_health_lock = acquire_sidecar_visibility_health_lock(&store).await;
    let parent = fresh_asset(&store).await;
    let sidecar = fresh_asset(&store).await;
    let sidecar_relation = store
        .record_media_sidecar_relation(&NewMediaSidecarRelation {
            parent_asset_id: parent,
            sidecar_asset_id: sidecar,
            relation_kind: MediaSidecarRelationKind::WorkflowJson,
            created_by: "mt-023-sidecar-anomaly".to_string(),
        })
        .await
        .expect("record valid sidecar relation");
    let constraints = sidecar_visibility_constraints(&store).await;
    assert!(
        constraints
            .iter()
            .any(|(_, definition)| definition.contains("hidden_from_gallery")),
        "sidecar table must normally enforce hidden-from-gallery"
    );
    assert!(
        constraints
            .iter()
            .any(|(_, definition)| definition.contains("searchable_by_relation")),
        "sidecar table must normally enforce searchable-by-relation"
    );

    drop_sidecar_visibility_constraints(&store, &constraints).await;
    sqlx::query(
        r#"UPDATE atelier_media_sidecar
           SET hidden_from_gallery = FALSE,
               searchable_by_relation = FALSE,
               updated_at_utc = NOW()
           WHERE sidecar_id = $1"#,
    )
    .bind(sidecar_relation.sidecar_id)
    .execute(store.pool())
    .await
    .expect("simulate drifted sidecar visibility row");
    let report_result = store
        .run_filesystem_health_check(&FilesystemHealthCheckRequest {
            requested_by: "mt-023-sidecar-anomaly".to_string(),
            scope_label: Some("sidecar-anomaly".to_string()),
        })
        .await;
    restore_sidecar_visibility_constraints(&store, &constraints, sidecar_relation.sidecar_id).await;
    sidecar_health_lock
        .commit()
        .await
        .expect("release sidecar visibility health lock");
    let report = report_result.expect("run filesystem health check with drifted sidecar");

    let anomaly = report
        .findings
        .iter()
        .find(|finding| {
            finding.finding_kind == FilesystemHealthFindingKind::SidecarVisibilityAnomaly
                && finding.target_id == sidecar_relation.sidecar_id.to_string()
        })
        .expect("drifted sidecar must be reported as sidecar_visibility_anomaly");
    assert_eq!(anomaly.target_type, "atelier_media_sidecar");
    assert_eq!(
        anomaly.details["hidden_from_gallery"],
        serde_json::json!(false),
        "anomaly finding must expose the drifted gallery flag"
    );
    assert_eq!(
        anomaly.details["searchable_by_relation"],
        serde_json::json!(false),
        "anomaly finding must expose the drifted relation-search flag"
    );
    assert!(
        report.check.summary["sidecar_visibility_anomalies_count"]
            .as_i64()
            .is_some_and(|count| count >= 1),
        "summary must count sidecar visibility anomalies"
    );
    let persisted = store
        .list_filesystem_health_findings(report.check.check_id)
        .await
        .expect("list persisted sidecar anomaly findings");
    assert!(
        persisted.iter().any(|finding| {
            finding.finding_kind == FilesystemHealthFindingKind::SidecarVisibilityAnomaly
                && finding.target_id == sidecar_relation.sidecar_id.to_string()
        }),
        "sidecar anomaly finding must persist for later review"
    );
}

#[tokio::test]
async fn atelier_filesystem_health_detects_missing_artifactstore_original_payload_and_manifest() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_filesystem_health_detects_missing_artifactstore_original_payload_and_manifest: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let missing_payload_artifact =
        atelier_pg_support::write_native_media_artifact(b"mt-023-missing-payload-original");
    let missing_payload_asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: missing_payload_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: missing_payload_artifact.byte_len,
            source_provenance: Some("mt-023-missing-payload".to_string()),
            artifact_ref: missing_payload_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize original whose payload will be removed");
    let payload_path = artifact_payload_path(&missing_payload_artifact);
    fs::remove_file(&payload_path).expect("remove ArtifactStore payload fixture");

    let missing_manifest_artifact =
        atelier_pg_support::write_native_media_artifact(b"mt-023-missing-manifest-original");
    let missing_manifest_asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: missing_manifest_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: missing_manifest_artifact.byte_len,
            source_provenance: Some("mt-023-missing-manifest".to_string()),
            artifact_ref: missing_manifest_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize original whose manifest will be removed");
    let manifest_path = artifact_manifest_path(&missing_manifest_artifact);
    fs::remove_file(&manifest_path).expect("remove ArtifactStore manifest fixture");

    let report = store
        .run_filesystem_health_check(&FilesystemHealthCheckRequest {
            requested_by: "mt-023-artifactstore-originals".to_string(),
            scope_label: Some("artifactstore-originals".to_string()),
        })
        .await
        .expect("run filesystem health check");
    assert!(
        report.findings.iter().any(|finding| {
            finding.finding_kind == FilesystemHealthFindingKind::MissingOriginal
                && finding.target_id == missing_payload_asset.asset_id.to_string()
                && finding.details["artifact_ref"]
                    == serde_json::json!(missing_payload_artifact.artifact_ref)
                && finding.details["artifact_issue"]
                    .as_str()
                    .is_some_and(|issue| issue.contains("content hash validation"))
        }),
        "missing ArtifactStore payload must be reported as a missing_original finding"
    );
    assert!(
        report.findings.iter().any(|finding| {
            finding.finding_kind == FilesystemHealthFindingKind::MissingOriginal
                && finding.target_id == missing_manifest_asset.asset_id.to_string()
                && finding.details["artifact_ref"]
                    == serde_json::json!(missing_manifest_artifact.artifact_ref)
                && finding.details["artifact_issue"]
                    .as_str()
                    .is_some_and(|issue| issue.contains("manifest"))
        }),
        "missing ArtifactStore manifest must be reported as a missing_original finding"
    );
    assert!(
        !payload_path.exists(),
        "health diagnostics must not recreate a missing original payload"
    );
    assert!(
        !manifest_path.exists(),
        "health diagnostics must not recreate a missing original manifest"
    );
}

#[tokio::test]
async fn atelier_filesystem_health_detects_missing_generated_thumbnail_artifact_payload() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_filesystem_health_detects_missing_generated_thumbnail_artifact_payload: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let original_artifact =
        atelier_pg_support::write_native_media_artifact(b"mt-023-thumbnail-original");
    let original = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: original_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: original_artifact.byte_len,
            source_provenance: Some("mt-023-thumbnail-original".to_string()),
            artifact_ref: original_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize original for thumbnail proof");
    let requested = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: original.asset_id,
            derivative_kind: MediaDerivativeKind::Thumbnail,
            target_width: 128,
            target_height: 128,
            format: "png".to_string(),
            requested_by: "mt-023-thumbnail-requester".to_string(),
        })
        .await
        .expect("request thumbnail derivative");
    store
        .mark_media_derivative_generating(requested.derivative_id, "mt-023-thumbnail-worker")
        .await
        .expect("mark thumbnail generating");
    let thumbnail_artifact =
        atelier_pg_support::write_native_media_artifact(b"mt-023-thumbnail-payload");
    let generated = store
        .record_media_derivative_generated_with_artifact(&MediaDerivativeGenerated {
            derivative_id: requested.derivative_id,
            artifact_ref: thumbnail_artifact.artifact_ref.clone(),
            artifact_manifest_ref: format!(
                "artifact://.handshake/artifacts/L1/{}/artifact.json",
                thumbnail_artifact.artifact_id
            ),
            mime: "image/png".to_string(),
            byte_len: thumbnail_artifact.byte_len,
            updated_by: "mt-023-thumbnail-worker".to_string(),
        })
        .await
        .expect("record generated thumbnail");
    let thumbnail_payload_path = artifact_payload_path(&thumbnail_artifact);
    fs::remove_file(&thumbnail_payload_path).expect("remove generated thumbnail payload fixture");

    let report = store
        .run_filesystem_health_check(&FilesystemHealthCheckRequest {
            requested_by: "mt-023-thumbnail-health".to_string(),
            scope_label: Some("thumbnail-artifact".to_string()),
        })
        .await
        .expect("run filesystem health check");
    assert!(
        report.findings.iter().any(|finding| {
            finding.finding_kind == FilesystemHealthFindingKind::MissingThumbnail
                && finding.target_id == generated.derivative_id.to_string()
                && finding.details["asset_id"] == serde_json::json!(original.asset_id)
                && finding.details["artifact_ref"]
                    == serde_json::json!(thumbnail_artifact.artifact_ref)
                && finding.details["artifact_issue"]
                    .as_str()
                    .is_some_and(|issue| issue.contains("content hash validation"))
        }),
        "generated thumbnail with missing ArtifactStore payload must be reported"
    );
    assert!(
        !thumbnail_payload_path.exists(),
        "health diagnostics must not recreate a missing thumbnail payload"
    );
}

#[tokio::test]
async fn atelier_filesystem_health_does_not_mark_generated_thumbnail_payload_untracked() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_filesystem_health_does_not_mark_generated_thumbnail_payload_untracked: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let original_artifact =
        atelier_pg_support::write_native_media_artifact(b"mt-023-healthy-thumbnail-original");
    let original = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: original_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: original_artifact.byte_len,
            source_provenance: Some("mt-023-healthy-thumbnail-original".to_string()),
            artifact_ref: original_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize original for healthy thumbnail proof");
    let requested = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: original.asset_id,
            derivative_kind: MediaDerivativeKind::Thumbnail,
            target_width: 128,
            target_height: 128,
            format: "png".to_string(),
            requested_by: "mt-023-healthy-thumbnail-requester".to_string(),
        })
        .await
        .expect("request healthy thumbnail derivative");
    store
        .mark_media_derivative_generating(
            requested.derivative_id,
            "mt-023-healthy-thumbnail-worker",
        )
        .await
        .expect("mark healthy thumbnail generating");
    let thumbnail_artifact =
        atelier_pg_support::write_native_media_artifact(b"mt-023-healthy-thumbnail-payload");
    store
        .record_media_derivative_generated_with_artifact(&MediaDerivativeGenerated {
            derivative_id: requested.derivative_id,
            artifact_ref: thumbnail_artifact.artifact_ref.clone(),
            artifact_manifest_ref: format!(
                "artifact://.handshake/artifacts/L1/{}/artifact.json",
                thumbnail_artifact.artifact_id
            ),
            mime: "image/png".to_string(),
            byte_len: thumbnail_artifact.byte_len,
            updated_by: "mt-023-healthy-thumbnail-worker".to_string(),
        })
        .await
        .expect("record healthy generated thumbnail");

    let report = store
        .run_filesystem_health_check(&FilesystemHealthCheckRequest {
            requested_by: "mt-023-healthy-thumbnail-health".to_string(),
            scope_label: Some("healthy-thumbnail-artifact".to_string()),
        })
        .await
        .expect("run filesystem health check");
    assert!(
        !report.findings.iter().any(|finding| {
            finding.finding_kind == FilesystemHealthFindingKind::UntrackedOriginal
                && finding.target_type == "artifact_store_payload"
                && finding.target_id == thumbnail_artifact.artifact_ref
        }),
        "generated derivative payloads are tracked by atelier_media_derivative and must not be reported as untracked originals"
    );
}

#[tokio::test]
async fn atelier_filesystem_health_detects_untracked_artifactstore_original_payload() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_filesystem_health_detects_untracked_artifactstore_original_payload: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let artifact = atelier_pg_support::write_native_media_artifact(b"mt-023-untracked-original");

    let catalog_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM atelier_media_asset WHERE content_hash = $1")
            .bind(&artifact.content_hash)
            .fetch_one(store.pool())
            .await
            .expect("count catalog rows for untracked artifact");
    assert_eq!(
        catalog_rows, 0,
        "untracked fixture must not already have a media asset row"
    );

    let report = store
        .run_filesystem_health_check(&FilesystemHealthCheckRequest {
            requested_by: "mt-023-untracked-artifact".to_string(),
            scope_label: Some("untracked-artifactstore-original".to_string()),
        })
        .await
        .expect("run filesystem health check");
    assert!(
        report.findings.iter().any(|finding| {
            finding.finding_kind == FilesystemHealthFindingKind::UntrackedOriginal
                && finding.target_type == "artifact_store_payload"
                && finding.target_id == artifact.artifact_ref
                && finding.details["content_hash"] == serde_json::json!(artifact.content_hash)
                && finding.details["artifact_ref"] == serde_json::json!(artifact.artifact_ref)
        }),
        "valid ArtifactStore payload without catalog/intake row must be reported as untracked"
    );
    assert!(
        artifact_payload_path(&artifact).exists(),
        "health diagnostics must not delete untracked ArtifactStore payloads"
    );
    assert!(
        artifact_manifest_path(&artifact).exists(),
        "health diagnostics must not delete untracked ArtifactStore manifests"
    );
}

#[tokio::test]
async fn atelier_media_source_provenance_refs_survive_export_pending_archive_and_reingest() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_media_source_provenance_refs_survive_export_pending_archive_and_reingest: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-source-provenance-{}", Uuid::new_v4()),
            display_name: "Source Provenance Subject".to_string(),
        })
        .await
        .expect("create character for provenance proof");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "source provenance proof sheet".to_string(),
            author: "mt-027-author".to_string(),
            tool: Some("mt-027-test".to_string()),
        })
        .await
        .expect("append sheet version for provenance proof");
    let artifact_seed = format!("mt-027-provenance-media-{}", Uuid::new_v4());
    let artifact = atelier_pg_support::write_native_media_artifact(artifact_seed.as_bytes());
    let asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("mt-027-initial".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize media asset for provenance proof");
    let contact_sheet = store
        .create_contact_sheet(
            "mt-027-contact-sheet",
            "manual",
            None,
            &[asset.asset_id],
            &["provenance".to_string()],
            Some(character.internal_id),
            Some(sheet.version_id),
        )
        .await
        .expect("create contact sheet for provenance proof");

    let source_url_ref = format!("source-url://atelier/{}", Uuid::new_v4());
    let source_path_ref = format!("source://operator-inbox/{}", Uuid::new_v4());
    let source_note_ref = format!("note://atelier/source/{}", Uuid::new_v4());
    let contact_sheet_ref = format!("contact-sheet://atelier/{}", contact_sheet.sheet_id);
    let task_ref = format!("task://wp-kernel-005/MT-027/{}", Uuid::new_v4());
    let run_ref = format!("run://atelier/source-provenance/{}", Uuid::new_v4());
    store
        .set_media_source_provenance_refs(&SetMediaSourceProvenanceRefs {
            asset_id: asset.asset_id,
            source_url_ref: Some(source_url_ref.clone()),
            source_path_ref: Some(source_path_ref.clone()),
            source_note_ref: Some(source_note_ref.clone()),
            contact_sheet_ref: Some(contact_sheet_ref.clone()),
            task_ref: Some(task_ref.clone()),
            run_ref: Some(run_ref.clone()),
            updated_by: "mt-027-operator".to_string(),
        })
        .await
        .expect("set structured source provenance refs");

    let source_collection = store
        .create_collection(&NewCollection {
            name: format!("mt-027-source-collection-{}", Uuid::new_v4()),
            notes: "mt-027 source collection before reassignment".to_string(),
            tags: vec!["mt-027".to_string(), "provenance".to_string()],
            character_internal_id: Some(character.internal_id),
            sheet_version_id: Some(sheet.version_id),
        })
        .await
        .expect("create source collection for provenance reassignment proof");
    let target_collection = store
        .create_collection(&NewCollection {
            name: format!("mt-027-target-collection-{}", Uuid::new_v4()),
            notes: "mt-027 target collection after reassignment".to_string(),
            tags: vec!["mt-027".to_string(), "reassigned".to_string()],
            character_internal_id: Some(character.internal_id),
            sheet_version_id: Some(sheet.version_id),
        })
        .await
        .expect("create target collection for provenance reassignment proof");
    assert_eq!(
        store
            .add_images_to_collection(source_collection.collection_id, &[asset.asset_id])
            .await
            .expect("add provenance media to source collection"),
        1,
        "source collection receives the provenance-bearing media asset"
    );
    assert_eq!(
        store
            .remove_images_from_collection(source_collection.collection_id, &[asset.asset_id])
            .await
            .expect("remove provenance media from source collection"),
        1,
        "source collection move removes the media asset exactly once"
    );
    assert_eq!(
        store
            .add_images_to_collection(target_collection.collection_id, &[asset.asset_id])
            .await
            .expect("add provenance media to target collection"),
        1,
        "target collection receives the reassigned media asset"
    );
    let source_members = store
        .list_collection_images(source_collection.collection_id)
        .await
        .expect("list source collection after reassignment");
    assert!(
        source_members
            .iter()
            .all(|member| member.asset_id != asset.asset_id),
        "source collection no longer owns the reassigned media asset"
    );
    let target_members = store
        .list_collection_images(target_collection.collection_id)
        .await
        .expect("list target collection after reassignment");
    assert!(
        target_members
            .iter()
            .any(|member| member.asset_id == asset.asset_id),
        "target collection owns the reassigned media asset"
    );

    let requested = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: asset.asset_id,
            derivative_kind: MediaDerivativeKind::Thumbnail,
            target_width: 128,
            target_height: 128,
            format: "png".to_string(),
            requested_by: "mt-027-thumbnail".to_string(),
        })
        .await
        .expect("request pending thumbnail without touching provenance");
    assert_eq!(
        requested.asset_id, asset.asset_id,
        "pending derivative must target the provenance-bearing asset"
    );

    let export = store
        .request_sheet_export(&NewExportRequest {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            format: ExportFormat::Json,
            label: Some("mt-027-backup-export".to_string()),
            requested_by: "mt-027-exporter".to_string(),
        })
        .await
        .expect("request export for provenance proof");
    let export_artifact_ref = format!("artifact://atelier/export/{}", Uuid::new_v4());
    store
        .record_export_result(
            export.export_id,
            &export_artifact_ref,
            &artifact.content_hash,
            512,
        )
        .await
        .expect("record export result");
    store
        .add_manifest_entry(
            export.export_id,
            ManifestItemKind::Media,
            &asset.artifact_ref,
            "images/original.png",
        )
        .await
        .expect("add media asset to export manifest");

    store
        .archive_deletion_targets(&DeletionArchiveRequest {
            targets: vec![DeletionTargetRef {
                target_type: DeletionTargetKind::MediaAsset,
                target_id: asset.asset_id,
            }],
            reason: "mt-027 archive survival".to_string(),
            requested_by: "mt-027-operator".to_string(),
        })
        .await
        .expect("archive media asset without dropping provenance");
    store
        .restore_deletion_targets(&DeletionRestoreRequest {
            targets: vec![DeletionTargetRef {
                target_type: DeletionTargetKind::MediaAsset,
                target_id: asset.asset_id,
            }],
            reason: "mt-027 restore survival".to_string(),
            requested_by: "mt-027-operator".to_string(),
        })
        .await
        .expect("restore media asset without dropping provenance");

    let duplicate = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("mt-027-reingest".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("reingest same content hash without dropping provenance");
    assert_eq!(
        duplicate.asset_id, asset.asset_id,
        "content-hash reingest must keep the same media identity"
    );

    let persisted = store
        .get_media_source_provenance_refs(asset.asset_id)
        .await
        .expect("load structured provenance refs")
        .expect("structured provenance refs exist");
    assert_eq!(persisted.asset_id, asset.asset_id);
    assert_eq!(
        persisted.source_url_ref.as_deref(),
        Some(source_url_ref.as_str())
    );
    assert_eq!(
        persisted.source_path_ref.as_deref(),
        Some(source_path_ref.as_str())
    );
    assert_eq!(
        persisted.source_note_ref.as_deref(),
        Some(source_note_ref.as_str())
    );
    assert_eq!(
        persisted.contact_sheet_ref.as_deref(),
        Some(contact_sheet_ref.as_str())
    );
    assert_eq!(persisted.task_ref.as_deref(), Some(task_ref.as_str()));
    assert_eq!(persisted.run_ref.as_deref(), Some(run_ref.as_str()));
    assert_eq!(persisted.updated_by, "mt-027-operator");
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_SOURCE_PROVENANCE_REFS_SET,
                "atelier_media_asset",
                &asset.asset_id.to_string(),
            )
            .await
            .expect("count source provenance ref events"),
        1,
        "setting structured provenance refs must write EventLedger evidence"
    );
    let provenance_event_payload: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_media_asset'
             AND aggregate_id = $2
           ORDER BY created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(event_family::MEDIA_SOURCE_PROVENANCE_REFS_SET)
    .bind(asset.asset_id.to_string())
    .fetch_one(store.pool())
    .await
    .expect("load source provenance ref event payload");
    assert_eq!(
        provenance_event_payload["source_url_ref"],
        serde_json::json!(source_url_ref)
    );
    assert_eq!(
        provenance_event_payload["source_path_ref"],
        serde_json::json!(source_path_ref)
    );
    assert_eq!(
        provenance_event_payload["source_note_ref"],
        serde_json::json!(source_note_ref)
    );
    assert_eq!(
        provenance_event_payload["contact_sheet_ref"],
        serde_json::json!(contact_sheet_ref)
    );
    assert_eq!(
        provenance_event_payload["task_ref"],
        serde_json::json!(task_ref)
    );
    assert_eq!(
        provenance_event_payload["run_ref"],
        serde_json::json!(run_ref)
    );

    let manifest = store
        .export_manifest(export.export_id)
        .await
        .expect("read export manifest");
    assert!(
        manifest
            .iter()
            .any(|entry| entry.kind == ManifestItemKind::Media
                && entry.artifact_ref == asset.artifact_ref),
        "export/share-pack manifest must still reference the provenance-bearing asset artifact"
    );
}

#[tokio::test]
async fn atelier_media_source_provenance_refs_reject_invalid_refs_and_direct_sql_bypass() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_media_source_provenance_refs_reject_invalid_refs_and_direct_sql_bypass: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let artifact_seed = format!("mt-027-invalid-provenance-media-{}", Uuid::new_v4());
    let artifact = atelier_pg_support::write_native_media_artifact(artifact_seed.as_bytes());
    let asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("mt-027-invalid".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize media asset for invalid provenance proof");

    let all_none = store
        .set_media_source_provenance_refs(&SetMediaSourceProvenanceRefs {
            asset_id: asset.asset_id,
            source_url_ref: None,
            source_path_ref: None,
            source_note_ref: None,
            contact_sheet_ref: None,
            task_ref: None,
            run_ref: None,
            updated_by: "mt-027-invalid".to_string(),
        })
        .await
        .expect_err("all-none structured source provenance refs must reject");
    assert!(
        all_none.to_string().contains("at least one"),
        "all-none rejection should explain missing refs, got {all_none}"
    );

    let blank = store
        .set_media_source_provenance_refs(&SetMediaSourceProvenanceRefs {
            asset_id: asset.asset_id,
            source_url_ref: Some("".to_string()),
            source_path_ref: None,
            source_note_ref: None,
            contact_sheet_ref: None,
            task_ref: None,
            run_ref: None,
            updated_by: "mt-027-invalid".to_string(),
        })
        .await
        .expect_err("blank structured source provenance refs must reject");
    assert!(
        blank.to_string().contains("source_url_ref"),
        "blank rejection should name source_url_ref, got {blank}"
    );

    let padded = store
        .set_media_source_provenance_refs(&SetMediaSourceProvenanceRefs {
            asset_id: asset.asset_id,
            source_url_ref: Some(" source-url://atelier/padded".to_string()),
            source_path_ref: None,
            source_note_ref: None,
            contact_sheet_ref: None,
            task_ref: None,
            run_ref: None,
            updated_by: "mt-027-invalid".to_string(),
        })
        .await
        .expect_err("padded structured source provenance refs must reject");
    assert!(
        padded.to_string().contains("source_url_ref"),
        "padded rejection should name source_url_ref, got {padded}"
    );

    let legacy = store
        .set_media_source_provenance_refs(&SetMediaSourceProvenanceRefs {
            asset_id: asset.asset_id,
            source_url_ref: Some(".GOV/source/leak".to_string()),
            source_path_ref: None,
            source_note_ref: None,
            contact_sheet_ref: None,
            task_ref: None,
            run_ref: None,
            updated_by: "mt-027-invalid".to_string(),
        })
        .await
        .expect_err("legacy or machine-local structured source provenance refs must reject");
    assert!(
        legacy.to_string().contains("portable ref"),
        "legacy rejection should explain portable-ref requirement, got {legacy}"
    );

    let direct_sql = sqlx::query(
        r#"INSERT INTO atelier_media_source_provenance_ref
             (asset_id, source_path_ref, updated_by)
           VALUES ($1, $2, $3)"#,
    )
    .bind(asset.asset_id)
    .bind(" source://operator-inbox/padded")
    .bind("mt-027-direct-sql")
    .execute(store.pool())
    .await
    .expect_err("database must reject direct SQL padded provenance refs");
    assert!(
        direct_sql
            .to_string()
            .contains("chk_atelier_media_source_provenance_ref_trimmed_nonempty"),
        "direct SQL rejection should name trimmed/non-empty constraint, got {direct_sql}"
    );
    assert!(
        store
            .get_media_source_provenance_refs(asset.asset_id)
            .await
            .expect("query invalid provenance refs")
            .is_none(),
        "rejected invalid provenance refs must not persist"
    );
}

#[tokio::test]
async fn atelier_similarity_computes_projection_and_rebuild_job_from_image_bytes() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_similarity_computes_projection_and_rebuild_job_from_image_bytes: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let bytes = similarity_png_bytes();

    let direct_asset = fresh_asset(&store).await;
    let projected = store
        .project_similarity_from_image_bytes(direct_asset, &bytes)
        .await
        .expect("compute and persist similarity projection from image bytes");
    let dhash = projected
        .dhash_hex
        .as_deref()
        .expect("computed projection includes dHash");
    assert!(is_hex64(dhash), "computed dHash is 64-bit hex");
    let palette = projected
        .palette_json
        .get("dominant")
        .and_then(|value| value.as_array())
        .expect("computed projection includes dominant palette array");
    assert!(
        palette.len() >= 2,
        "dominant palette captures multiple fixture colors"
    );
    assert!(
        palette.iter().all(|entry| entry.get("hex").is_some()),
        "palette entries carry stable hex colors"
    );
    assert_eq!(
        store
            .get_similarity_projection(direct_asset)
            .await
            .expect("read computed projection")
            .expect("computed projection persisted")
            .dhash_hex,
        Some(dhash.to_string()),
        "computed projection is persisted for search"
    );

    let rebuild_asset = fresh_asset(&store).await;
    let job = store
        .rebuild_similarity_projection_from_image_bytes(rebuild_asset, &bytes, "mt-020-worker")
        .await
        .expect("run similarity rebuild job from image bytes");
    assert_eq!(job.status, SimilarityRebuildJobStatus::Completed);
    assert_eq!(job.asset_internal_id, rebuild_asset);
    assert_eq!(job.processed_count, 1);
    assert_eq!(job.failed_count, 0);
    assert!(job.error_ref.is_none());
    assert!(
        job.dhash_hex.as_deref().is_some_and(is_hex64),
        "rebuild job records computed dHash"
    );
    assert!(
        store
            .get_similarity_projection(rebuild_asset)
            .await
            .expect("read rebuilt projection")
            .is_some(),
        "rebuild job persists the computed similarity projection"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                search_event_family::SIMILARITY_REBUILD_COMPLETED,
                "atelier_similarity_rebuild_job",
                &job.job_id.to_string(),
            )
            .await
            .expect("count similarity rebuild completion events"),
        1,
        "rebuild completion writes EventLedger evidence"
    );
}

#[tokio::test]
async fn atelier_rejects_legacy_runtime_refs_at_persistence_boundaries() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_rejects_legacy_runtime_refs_at_persistence_boundaries: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("runtime-ref-tripwire-{}", Uuid::new_v4()),
            source_label: "runtime ref tripwire".to_string(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("open tripwire intake batch");
    let intake_err = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: "http://localhost:9000/intake/a.png".to_string(),
                file_name: "a.png".to_string(),
                byte_len: 1,
                content_hash: None,
            },
        )
        .await
        .expect_err("localhost intake refs are forbidden");
    assert!(
        intake_err
            .to_string()
            .contains("Handshake-native portable ref"),
        "unexpected intake error: {intake_err}"
    );

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-runtime-ref-{}", Uuid::new_v4()),
            display_name: "Runtime Ref Subject".to_string(),
        })
        .await
        .expect("create character");
    let version = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "name: Runtime Ref Subject".to_string(),
            author: "operator".to_string(),
            tool: None,
        })
        .await
        .expect("append sheet version");
    let request = store
        .request_sheet_export(&NewExportRequest {
            character_internal_id: character.internal_id,
            sheet_version_id: version.version_id,
            format: ExportFormat::Markdown,
            label: Some("tripwire".to_string()),
            requested_by: "operator".to_string(),
        })
        .await
        .expect("request export");

    let export_err = store
        .record_export_result(
            request.export_id,
            "C:\\Users\\operator\\exports\\sheet.md",
            &format!("sha256-{}", Uuid::new_v4()),
            12,
        )
        .await
        .expect_err("machine-local export refs are forbidden");
    assert!(
        export_err
            .to_string()
            .contains("Handshake-native portable ref"),
        "unexpected export error: {export_err}"
    );

    let manifest_err = store
        .add_manifest_entry(
            request.export_id,
            ManifestItemKind::Sheet,
            &format!("artifact://atelier/export/{}", Uuid::new_v4()),
            ".GOV/leaked-sheet.md",
        )
        .await
        .expect_err(".GOV output pack paths are forbidden");
    assert!(
        manifest_err
            .to_string()
            .contains("Handshake-native portable ref"),
        "unexpected manifest error: {manifest_err}"
    );
}

#[tokio::test]
async fn atelier_intake_lanes_idempotency_and_close_guard() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_intake_lanes_idempotency_and_close_guard: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    // --- open batch is idempotent on idempotency_key ---
    let key = format!("intake-key-{}", Uuid::new_v4());
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: key.clone(),
            source_label: "operator inbox scan".to_string(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("open intake batch");
    assert_eq!(batch.status, BatchStatus::Open);
    let batch_again = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: key.clone(),
            source_label: "operator inbox scan (rescan)".to_string(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("re-open same intake batch");
    assert_eq!(
        batch.batch_id, batch_again.batch_id,
        "re-opening the same idempotency_key must return the existing batch"
    );

    // --- add item is idempotent on (batch, source_path) ---
    let source_path = format!("source://operator-inbox/{}/a.png", Uuid::new_v4());
    let item = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: source_path.clone(),
                file_name: "a.png".to_string(),
                byte_len: 1234,
                content_hash: Some(format!("sha256-{}", Uuid::new_v4())),
            },
        )
        .await
        .expect("add intake item");
    assert_eq!(
        item.lane,
        IntakeLane::Pending,
        "new items enter the Pending lane"
    );
    let item_again = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: source_path.clone(),
                file_name: "a.png".to_string(),
                byte_len: 1234,
                content_hash: None,
            },
        )
        .await
        .expect("re-add same source path");
    assert_eq!(
        item.item_id, item_again.item_id,
        "re-adding the same (batch, source_path) must not duplicate the item"
    );

    // Add a second item so we can exercise lane spread + the close guard.
    let source_path_2 = format!("source://operator-inbox/{}/b.png", Uuid::new_v4());
    let item2 = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: source_path_2.clone(),
                file_name: "b.png".to_string(),
                byte_len: 5678,
                content_hash: None,
            },
        )
        .await
        .expect("add second intake item");

    // Two items, both still Pending.
    let counts0 = store
        .intake_lane_counts(batch.batch_id)
        .await
        .expect("lane counts pre-triage");
    assert_eq!(counts0.pending, 2, "both items start in the Pending lane");
    assert_eq!(counts0.accepted, 0);
    assert_eq!(counts0.rejected, 0);
    assert_eq!(counts0.deferred, 0);
    assert_eq!(counts0.skipped, 0);
    assert_eq!(counts0.failed, 0);

    // --- close REFUSES while items are still Pending ---
    let close_err = store.close_intake_batch(batch.batch_id).await;
    assert!(
        close_err.is_err(),
        "closing with Pending-lane items must error, not silently drop them"
    );

    // --- classify moves lane and PRESERVES source_path (no delete) ---
    let classified = store
        .classify_intake_item(item.item_id, IntakeLane::Accepted, Some("looks good"))
        .await
        .expect("classify first item accepted");
    assert_eq!(
        classified.lane,
        IntakeLane::Accepted,
        "lane moved to Accepted"
    );
    assert_eq!(
        classified.source_path, source_path,
        "classify must preserve the original source_path (never delete the source)"
    );
    let classified_again = store
        .classify_intake_item(item.item_id, IntakeLane::Accepted, Some("looks good"))
        .await
        .expect("repeating same accepted classification is idempotent");
    assert_eq!(classified_again, classified);
    assert_eq!(
        store
            .count_events_for_aggregate(
                intake_event_family::INTAKE_ITEM_CLASSIFIED,
                "atelier_intake_item",
                &item.item_id.to_string(),
            )
            .await
            .expect("count accepted classification events"),
        1,
        "repeating the same accepted classification must not duplicate EventLedger rows"
    );

    let rejected = store
        .classify_intake_item(item2.item_id, IntakeLane::Rejected, Some("dup"))
        .await
        .expect("classify second item rejected");
    assert_eq!(rejected.lane, IntakeLane::Rejected);
    assert_eq!(
        rejected.source_path, source_path_2,
        "rejecting an item only moves its lane; source_path is retained"
    );
    let rejected_again = store
        .classify_intake_item(item2.item_id, IntakeLane::Rejected, Some("dup"))
        .await
        .expect("repeating same rejected classification is idempotent");
    assert_eq!(rejected_again, rejected);
    assert_eq!(
        store
            .count_events_for_aggregate(
                intake_event_family::INTAKE_ITEM_CLASSIFIED,
                "atelier_intake_item",
                &item2.item_id.to_string(),
            )
            .await
            .expect("count rejected classification events"),
        1,
        "repeating the same rejected classification must not duplicate classified events"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                intake_event_family::INTAKE_ITEM_REJECTION_AUDITED,
                "atelier_intake_item",
                &item2.item_id.to_string(),
            )
            .await
            .expect("count rejected audit events"),
        1,
        "repeating the same rejected classification must not duplicate audit events"
    );
    let wrong_batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("intake-wrong-audit-batch-{}", Uuid::new_v4()),
            source_label: "wrong audit batch".to_string(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("open wrong batch for audit FK proof");
    let wrong_batch_audit = sqlx::query(
        r#"INSERT INTO atelier_intake_item_rejection_audit
             (item_id, batch_id, lane, reason, source_path_ref)
           VALUES ($1, $2, 'rejected', $3, $4)"#,
    )
    .bind(item2.item_id)
    .bind(wrong_batch.batch_id)
    .bind(format!("wrong-batch-audit-{}", Uuid::new_v4()))
    .bind(format!("sha256:{}", Uuid::new_v4().simple()))
    .execute(store.pool())
    .await
    .expect_err("database must reject rejection audit rows whose batch_id does not own item_id");
    assert!(
        wrong_batch_audit
            .to_string()
            .contains("fk_atelier_intake_rejection_audit_item_batch"),
        "wrong-batch audit rejection should name composite FK, got {wrong_batch_audit}"
    );

    // --- lane counts correct after triage ---
    let counts1 = store
        .intake_lane_counts(batch.batch_id)
        .await
        .expect("lane counts post-triage");
    assert_eq!(counts1.pending, 0, "no items left Pending after triage");
    assert_eq!(counts1.accepted, 1);
    assert_eq!(counts1.rejected, 1);
    assert_eq!(counts1.deferred, 0);
    assert_eq!(counts1.skipped, 0);
    assert_eq!(counts1.failed, 0);

    // The rejected item's row is still listable (no silent delete path).
    let all_items = store
        .list_intake_items(batch.batch_id, None)
        .await
        .expect("list all items");
    assert_eq!(all_items.len(), 2, "both source rows are preserved");

    // --- close SUCCEEDS once all items are classified out of New ---
    let closed = store
        .close_intake_batch(batch.batch_id)
        .await
        .expect("close batch after triage complete");
    assert_eq!(closed.status, BatchStatus::Closed, "batch is now Closed");
}

#[tokio::test]
async fn atelier_intake_item_lifecycle_skipped_failed_and_rejection_audits() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_intake_item_lifecycle_skipped_failed_and_rejection_audits: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    assert!(
        event_family::ALL.contains(&intake_event_family::INTAKE_ITEM_REJECTION_AUDITED),
        "negative intake lifecycle audit events must be discoverable through the parent registry"
    );

    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("mt-029-lifecycle-{}", Uuid::new_v4()),
            source_label: "mt-029 lifecycle".to_string(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("open intake batch");

    let mut items = Vec::new();
    for name in [
        "pending", "accepted", "rejected", "deferred", "skipped", "failed",
    ] {
        items.push(
            store
                .add_intake_item(
                    batch.batch_id,
                    &NewIntakeItem {
                        source_path: format!(
                            "source://operator-inbox/{}/{}.png",
                            Uuid::new_v4(),
                            name
                        ),
                        file_name: format!("{name}.png"),
                        byte_len: 512,
                        content_hash: Some(format!("sha256:{}", Uuid::new_v4())),
                    },
                )
                .await
                .expect("add lifecycle item"),
        );
    }
    assert_eq!(items[0].lane, IntakeLane::Pending);

    store
        .classify_intake_item(items[1].item_id, IntakeLane::Accepted, Some("accepted"))
        .await
        .expect("accepted state");
    store
        .classify_intake_item(
            items[2].item_id,
            IntakeLane::Rejected,
            Some("duplicate source"),
        )
        .await
        .expect("rejected state");
    store
        .classify_intake_item(items[3].item_id, IntakeLane::Deferred, Some("needs review"))
        .await
        .expect("deferred state");
    store
        .classify_intake_item(
            items[4].item_id,
            IntakeLane::Skipped,
            Some("unsupported file"),
        )
        .await
        .expect("skipped state");
    store
        .classify_intake_item(
            items[5].item_id,
            IntakeLane::Failed,
            Some("hash read failed"),
        )
        .await
        .expect("failed state");
    store
        .classify_intake_item(
            items[2].item_id,
            IntakeLane::Rejected,
            Some("duplicate source"),
        )
        .await
        .expect("idempotent repeated rejection");

    let counts = store
        .intake_lane_counts(batch.batch_id)
        .await
        .expect("lifecycle lane counts");
    assert_eq!(counts.pending, 1);
    assert_eq!(counts.accepted, 1);
    assert_eq!(counts.rejected, 1);
    assert_eq!(counts.deferred, 1);
    assert_eq!(counts.skipped, 1);
    assert_eq!(counts.failed, 1);

    let skipped = store
        .list_intake_items(batch.batch_id, Some(IntakeLane::Skipped))
        .await
        .expect("list skipped lane");
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].source_path, items[4].source_path);

    let audits = store
        .list_intake_rejection_audits(batch.batch_id)
        .await
        .expect("list rejection audit rows");
    assert_eq!(
        audits.len(),
        3,
        "rejected/skipped/failed rows each get one idempotent audit row"
    );
    let audit_by_lane: HashMap<_, _> = audits.iter().map(|audit| (audit.lane, audit)).collect();
    assert_eq!(
        audit_by_lane
            .get(&IntakeLane::Rejected)
            .expect("rejected audit")
            .reason,
        "duplicate source"
    );
    assert_eq!(
        audit_by_lane
            .get(&IntakeLane::Skipped)
            .expect("skipped audit")
            .reason,
        "unsupported file"
    );
    assert_eq!(
        audit_by_lane
            .get(&IntakeLane::Failed)
            .expect("failed audit")
            .reason,
        "hash read failed"
    );
    for audit in &audits {
        assert_eq!(audit.batch_id, batch.batch_id);
        assert!(
            audit.source_path_ref.starts_with("sha256:"),
            "audit rows should retain a source ref, not leak raw source paths"
        );
    }

    let rejected_events = store
        .count_events_for_aggregate(
            intake_event_family::INTAKE_ITEM_REJECTION_AUDITED,
            "atelier_intake_item",
            &items[2].item_id.to_string(),
        )
        .await
        .expect("rejected audit event count");
    assert_eq!(
        rejected_events, 1,
        "repeating the same rejected state/reason must not duplicate audit events"
    );
}

#[tokio::test]
async fn atelier_intake_classification_apply_links_media_and_rolls_back_invalid_target() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_intake_classification_apply_links_media_and_rolls_back_invalid_target: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("mt-031-character-{}", Uuid::new_v4()),
            display_name: "MT-031 Character".to_string(),
        })
        .await
        .expect("create character for classification apply");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "mt-031 linked sheet".to_string(),
            author: "mt-031".to_string(),
            tool: Some("mt-031-test".to_string()),
        })
        .await
        .expect("append sheet for classification apply");
    let collection = store
        .create_collection(&NewCollection {
            name: format!("mt-031-target-collection-{}", Uuid::new_v4()),
            notes: "mt-031 target collection".to_string(),
            tags: vec!["mt-031".to_string()],
            character_internal_id: Some(character.internal_id),
            sheet_version_id: Some(sheet.version_id),
        })
        .await
        .expect("create target collection");
    let artifact = atelier_pg_support::write_native_media_artifact(
        format!("mt-031-media-{}", Uuid::new_v4()).as_bytes(),
    );
    let asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("mt-031-intake-source".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize media asset for accepted intake item");
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("mt-031-apply-{}", Uuid::new_v4()),
            source_label: "mt-031 classification apply".to_string(),
            source_ref: Some(format!("source://mt-031/{}", Uuid::new_v4())),
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::CharacterLinked,
            character_internal_id: Some(character.internal_id),
            target_character_id: Some(character.internal_id),
            target_sheet_version_id: Some(sheet.version_id),
            target_collection_id: Some(collection.collection_id),
            resume_cursor: None,
        })
        .await
        .expect("open linked intake batch for apply");
    let accepted_item = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: format!("source://mt-031/accepted/{}", Uuid::new_v4()),
                file_name: "accepted.png".to_string(),
                byte_len: artifact.byte_len,
                content_hash: Some(asset.content_hash.clone()),
            },
        )
        .await
        .expect("add accepted intake item");

    let applied = store
        .apply_intake_classification(&ApplyIntakeClassificationRequest {
            item_id: accepted_item.item_id,
            lane: IntakeLane::Accepted,
            reason: Some("accepted into target collection".to_string()),
        })
        .await
        .expect("apply accepted classification to media target");
    assert_eq!(applied.item.lane, IntakeLane::Accepted);
    assert_eq!(applied.asset_id, Some(asset.asset_id));
    assert_eq!(applied.collection_id, Some(collection.collection_id));
    assert!(applied.collection_inserted);
    let members = store
        .list_collection_images(collection.collection_id)
        .await
        .expect("list target collection after accepted apply");
    assert!(
        members
            .iter()
            .any(|member| member.asset_id == asset.asset_id),
        "accepted intake item must link its matching media asset into the target collection"
    );

    let applied_again = store
        .apply_intake_classification(&ApplyIntakeClassificationRequest {
            item_id: accepted_item.item_id,
            lane: IntakeLane::Accepted,
            reason: Some("accepted into target collection".to_string()),
        })
        .await
        .expect("reapply accepted classification idempotently");
    assert_eq!(applied_again.item, applied.item);
    assert_eq!(applied_again.asset_id, Some(asset.asset_id));
    assert_eq!(applied_again.collection_id, Some(collection.collection_id));
    assert!(
        !applied_again.collection_inserted,
        "reapplying accepted classification must not duplicate target collection membership"
    );

    let rejected_item = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: format!("source://mt-031/rejected/{}", Uuid::new_v4()),
                file_name: "rejected.png".to_string(),
                byte_len: 128,
                content_hash: None,
            },
        )
        .await
        .expect("add rejected intake item");
    let rejected = store
        .apply_intake_classification(&ApplyIntakeClassificationRequest {
            item_id: rejected_item.item_id,
            lane: IntakeLane::Rejected,
            reason: Some("operator rejected".to_string()),
        })
        .await
        .expect("apply rejected classification");
    assert_eq!(rejected.item.lane, IntakeLane::Rejected);
    assert_eq!(rejected.asset_id, None);
    assert_eq!(
        store
            .count_events_for_aggregate(
                intake_event_family::INTAKE_ITEM_REJECTION_AUDITED,
                "atelier_intake_item",
                &rejected_item.item_id.to_string(),
            )
            .await
            .expect("count rejected apply audit event"),
        1,
        "rejected apply decisions must write rejection audit evidence"
    );

    let missing_hash = format!(
        "sha256:{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let invalid_item = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: format!("source://mt-031/invalid/{}", Uuid::new_v4()),
                file_name: "invalid.png".to_string(),
                byte_len: artifact.byte_len,
                content_hash: Some(missing_hash),
            },
        )
        .await
        .expect("add invalid-target intake item");
    let error = store
        .apply_intake_classification(&ApplyIntakeClassificationRequest {
            item_id: invalid_item.item_id,
            lane: IntakeLane::Accepted,
            reason: Some("invalid target should rollback".to_string()),
        })
        .await
        .expect_err("accepted apply with missing target media asset must reject");
    assert!(
        error.to_string().contains("target media asset"),
        "invalid target error should name target media asset, got {error}"
    );
    let persisted_invalid = store
        .get_intake_item(batch.batch_id, &invalid_item.source_path)
        .await
        .expect("reload invalid-target item")
        .expect("invalid-target item still exists");
    assert_eq!(
        persisted_invalid.lane,
        IntakeLane::Pending,
        "invalid target must roll back the lane change"
    );
}

#[tokio::test]
async fn atelier_intake_batch_mode_resume_and_source_ref_survive_reconnect() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_intake_batch_mode_resume_and_source_ref_survive_reconnect: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    assert!(
        event_family::ALL.contains(&intake_event_family::INTAKE_BATCH_RESUMED),
        "intake batch resume events must be discoverable through the parent atelier event registry"
    );
    let key = format!("mt-028-resume-batch-{}", Uuid::new_v4());
    let source_ref = format!("source://operator-inbox/{}", Uuid::new_v4());
    let initial_cursor = format!("cursor://atelier/intake/{}/scan-0001", Uuid::new_v4());
    let resumed_cursor = format!("cursor://atelier/intake/{}/scan-0002", Uuid::new_v4());

    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: key.clone(),
            source_label: "operator inbox scan".to_string(),
            source_ref: Some(source_ref.clone()),
            mode: IntakeBatchMode::FolderScan,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: Some(initial_cursor.clone()),
        })
        .await
        .expect("open resumable intake batch");
    assert_eq!(batch.source_ref, source_ref);
    assert_eq!(batch.mode, IntakeBatchMode::FolderScan);
    assert_eq!(batch.status, BatchStatus::Open);
    assert_eq!(
        batch.resume_cursor.as_deref(),
        Some(initial_cursor.as_str())
    );

    let resumed = store
        .mark_intake_batch_in_progress(batch.batch_id, &resumed_cursor, "mt-028-resumer")
        .await
        .expect("mark intake batch in progress with resume cursor");
    assert_eq!(resumed.batch_id, batch.batch_id);
    assert_eq!(resumed.status, BatchStatus::InProgress);
    assert_eq!(
        resumed.resume_cursor.as_deref(),
        Some(resumed_cursor.as_str())
    );
    assert!(
        resumed.resumed_at_utc.is_some(),
        "resuming a batch records the resume timestamp"
    );

    let reopened_store = connected_store(&url).await;
    let persisted = reopened_store
        .get_intake_batch_by_key(&key)
        .await
        .expect("load intake batch after reconnect")
        .expect("batch persisted across reconnect");
    assert_eq!(persisted.batch_id, batch.batch_id);
    assert_eq!(persisted.source_ref, source_ref);
    assert_eq!(persisted.mode, IntakeBatchMode::FolderScan);
    assert_eq!(persisted.status, BatchStatus::InProgress);
    assert_eq!(
        persisted.resume_cursor.as_deref(),
        Some(resumed_cursor.as_str())
    );
    assert!(
        persisted.resumed_at_utc.is_some(),
        "resume timestamp survives reconnect"
    );

    let in_progress = reopened_store
        .list_intake_batches(Some(BatchStatus::InProgress), 50)
        .await
        .expect("list in-progress intake batches");
    assert!(
        in_progress
            .iter()
            .any(|candidate| candidate.batch_id == batch.batch_id),
        "status-filtered listing must include resumed in-progress batches"
    );
    assert_eq!(
        reopened_store
            .count_events_for_aggregate(
                intake_event_family::INTAKE_BATCH_RESUMED,
                "atelier_intake_batch",
                &batch.batch_id.to_string(),
            )
            .await
            .expect("count intake batch resumed events"),
        1,
        "resume status changes must write EventLedger evidence"
    );
}

#[tokio::test]
async fn atelier_intake_loose_and_linked_target_refs_survive_reconnect() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_intake_loose_and_linked_target_refs_survive_reconnect: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("mt-030-character-{}", Uuid::new_v4()),
            display_name: "MT-030 Character".to_string(),
        })
        .await
        .expect("create linked character");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "mt-030 linked sheet".to_string(),
            author: "mt-030".to_string(),
            tool: Some("mt-030-test".to_string()),
        })
        .await
        .expect("create linked sheet version");
    let collection = store
        .create_collection(&NewCollection {
            name: format!("mt-030-linked-collection-{}", Uuid::new_v4()),
            notes: "mt-030 linked collection".to_string(),
            tags: vec!["mt-030".to_string()],
            character_internal_id: Some(character.internal_id),
            sheet_version_id: Some(sheet.version_id),
        })
        .await
        .expect("create linked collection");

    let loose_key = format!("mt-030-loose-{}", Uuid::new_v4());
    let loose = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: loose_key.clone(),
            source_label: "mt-030 loose profile intake".to_string(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("open loose intake batch");
    assert_eq!(loose.profile_mode, IntakeProfileMode::LooseProfile);
    assert_eq!(loose.target_character_id, None);
    assert_eq!(loose.target_sheet_version_id, None);
    assert_eq!(loose.target_collection_id, None);

    let loose_as_linked = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: loose_key.clone(),
            source_label: "mt-030 loose profile intake".to_string(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::CharacterLinked,
            character_internal_id: Some(character.internal_id),
            target_character_id: Some(character.internal_id),
            target_sheet_version_id: Some(sheet.version_id),
            target_collection_id: Some(collection.collection_id),
            resume_cursor: None,
        })
        .await
        .expect_err("same idempotency key must reject loose-to-linked contract drift");
    assert!(
        loose_as_linked
            .to_string()
            .contains("incompatible intake batch idempotency_key"),
        "unexpected loose-to-linked error: {loose_as_linked}"
    );
    let loose_after_conflict = store
        .get_intake_batch_by_key(&loose_key)
        .await
        .expect("load loose batch after rejected drift")
        .expect("loose batch remains persisted");
    assert_eq!(loose_after_conflict.batch_id, loose.batch_id);
    assert_eq!(
        loose_after_conflict.profile_mode,
        IntakeProfileMode::LooseProfile
    );
    assert_eq!(loose_after_conflict.target_collection_id, None);

    let linked_key = format!("mt-030-linked-{}", Uuid::new_v4());
    let linked = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: linked_key.clone(),
            source_label: "mt-030 character linked intake".to_string(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::CharacterLinked,
            character_internal_id: Some(character.internal_id),
            target_character_id: Some(character.internal_id),
            target_sheet_version_id: Some(sheet.version_id),
            target_collection_id: Some(collection.collection_id),
            resume_cursor: None,
        })
        .await
        .expect("open linked intake batch");
    assert_eq!(linked.profile_mode, IntakeProfileMode::CharacterLinked);
    assert_eq!(linked.character_internal_id, Some(character.internal_id));
    assert_eq!(linked.target_character_id, Some(character.internal_id));
    assert_eq!(linked.target_sheet_version_id, Some(sheet.version_id));
    assert_eq!(linked.target_collection_id, Some(collection.collection_id));

    let other_collection = store
        .create_collection(&NewCollection {
            name: format!("mt-030-other-collection-{}", Uuid::new_v4()),
            notes: "mt-030 incompatible linked collection".to_string(),
            tags: vec!["mt-030".to_string(), "other-target".to_string()],
            character_internal_id: Some(character.internal_id),
            sheet_version_id: Some(sheet.version_id),
        })
        .await
        .expect("create incompatible linked collection");
    let linked_different_target = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: linked_key.clone(),
            source_label: "mt-030 character linked intake".to_string(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::CharacterLinked,
            character_internal_id: Some(character.internal_id),
            target_character_id: Some(character.internal_id),
            target_sheet_version_id: Some(sheet.version_id),
            target_collection_id: Some(other_collection.collection_id),
            resume_cursor: None,
        })
        .await
        .expect_err("same idempotency key must reject linked target drift");
    assert!(
        linked_different_target
            .to_string()
            .contains("target_collection_id"),
        "unexpected linked-target error: {linked_different_target}"
    );
    let linked_after_conflict = store
        .get_intake_batch_by_key(&linked_key)
        .await
        .expect("load linked batch after rejected target drift")
        .expect("linked batch remains persisted");
    assert_eq!(linked_after_conflict.batch_id, linked.batch_id);
    assert_eq!(
        linked_after_conflict.target_collection_id,
        Some(collection.collection_id)
    );

    let reconnected = connected_store(&url).await;
    let linked_after = reconnected
        .get_intake_batch_by_key(&linked_key)
        .await
        .expect("load linked batch after reconnect")
        .expect("linked batch exists");
    assert_eq!(
        linked_after.profile_mode,
        IntakeProfileMode::CharacterLinked
    );
    assert_eq!(
        linked_after.target_character_id,
        Some(character.internal_id)
    );
    assert_eq!(linked_after.target_sheet_version_id, Some(sheet.version_id));
    assert_eq!(
        linked_after.target_collection_id,
        Some(collection.collection_id)
    );

    let linked_batches = reconnected
        .list_intake_batches_by_profile_mode(Some(IntakeProfileMode::CharacterLinked), 50)
        .await
        .expect("list linked profile batches");
    assert!(
        linked_batches
            .iter()
            .any(|batch| batch.batch_id == linked.batch_id),
        "linked intake batches must be queryable by profile mode"
    );

    let loose_batches = reconnected
        .list_intake_batches_by_profile_mode(Some(IntakeProfileMode::LooseProfile), 50)
        .await
        .expect("list loose profile batches");
    assert!(
        loose_batches
            .iter()
            .any(|batch| batch.batch_id == loose.batch_id),
        "loose intake batches must remain distinguishable from linked batches"
    );
}

#[tokio::test]
async fn atelier_intake_sheet_collection_links_survive_export() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_intake_sheet_collection_links_survive_export: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("mt-032-character-{}", Uuid::new_v4()),
            display_name: "MT-032 Character".to_string(),
        })
        .await
        .expect("create export-linked intake character");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "mt-032 pinned sheet".to_string(),
            author: "mt-032".to_string(),
            tool: Some("mt-032-test".to_string()),
        })
        .await
        .expect("append sheet version for export-linked intake");
    let collection = store
        .create_collection(&NewCollection {
            name: format!("mt-032-collection-{}", Uuid::new_v4()),
            notes: "mt-032 target collection".to_string(),
            tags: vec!["mt-032".to_string()],
            character_internal_id: Some(character.internal_id),
            sheet_version_id: Some(sheet.version_id),
        })
        .await
        .expect("create target collection for export-linked intake");

    let linked_batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("mt-032-versioned-{}", Uuid::new_v4()),
            source_label: "mt-032 versioned intake".to_string(),
            source_ref: Some(format!("source://mt-032/versioned/{}", Uuid::new_v4())),
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::CharacterLinked,
            character_internal_id: Some(character.internal_id),
            target_character_id: Some(character.internal_id),
            target_sheet_version_id: Some(sheet.version_id),
            target_collection_id: Some(collection.collection_id),
            resume_cursor: None,
        })
        .await
        .expect("open versioned linked intake batch");
    let linked_item = store
        .add_intake_item(
            linked_batch.batch_id,
            &NewIntakeItem {
                source_path: format!("source://mt-032/versioned-item/{}", Uuid::new_v4()),
                file_name: "versioned.png".to_string(),
                byte_len: 128,
                content_hash: Some(format!("sha256:{}", Uuid::new_v4())),
            },
        )
        .await
        .expect("add versioned linked intake item");

    let version_agnostic_batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("mt-032-version-agnostic-{}", Uuid::new_v4()),
            source_label: "mt-032 version-agnostic intake".to_string(),
            source_ref: Some(format!(
                "source://mt-032/version-agnostic/{}",
                Uuid::new_v4()
            )),
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::CharacterLinked,
            character_internal_id: Some(character.internal_id),
            target_character_id: Some(character.internal_id),
            target_sheet_version_id: None,
            target_collection_id: Some(collection.collection_id),
            resume_cursor: None,
        })
        .await
        .expect("open version-agnostic linked intake batch");
    let version_agnostic_item = store
        .add_intake_item(
            version_agnostic_batch.batch_id,
            &NewIntakeItem {
                source_path: format!("source://mt-032/version-agnostic-item/{}", Uuid::new_v4()),
                file_name: "version-agnostic.png".to_string(),
                byte_len: 256,
                content_hash: Some(format!("sha256:{}", Uuid::new_v4())),
            },
        )
        .await
        .expect("add version-agnostic linked intake item");

    let export = store
        .request_sheet_export(&NewExportRequest {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            format: ExportFormat::Json,
            label: Some("mt-032 export".to_string()),
            requested_by: "mt-032-exporter".to_string(),
        })
        .await
        .expect("request export for intake link proof");

    let before_events = store
        .count_events(export_event_family::EXPORT_INTAKE_LINK_ATTACHED)
        .await
        .expect("count export intake link events before attach");

    let versioned_link = store
        .attach_intake_link_to_export(export.export_id, linked_batch.batch_id, linked_item.item_id)
        .await
        .expect("attach versioned intake link to export");
    assert_eq!(versioned_link.export_id, export.export_id);
    assert_eq!(versioned_link.batch_id, linked_batch.batch_id);
    assert_eq!(versioned_link.item_id, linked_item.item_id);
    assert_eq!(
        versioned_link.target_character_id,
        Some(character.internal_id)
    );
    assert_eq!(
        versioned_link.target_sheet_version_id,
        Some(sheet.version_id)
    );
    assert_eq!(
        versioned_link.target_collection_id,
        Some(collection.collection_id)
    );
    assert!(
        !versioned_link.version_agnostic,
        "a batch with a sheet-version target is not version agnostic"
    );

    let version_agnostic_link = store
        .attach_intake_link_to_export(
            export.export_id,
            version_agnostic_batch.batch_id,
            version_agnostic_item.item_id,
        )
        .await
        .expect("attach version-agnostic intake link to export");
    assert_eq!(
        version_agnostic_link.target_sheet_version_id, None,
        "omitting target_sheet_version_id preserves the version-agnostic default"
    );
    assert_eq!(
        version_agnostic_link.target_collection_id,
        Some(collection.collection_id)
    );
    assert!(
        version_agnostic_link.version_agnostic,
        "missing target_sheet_version_id marks the intake export link as version agnostic"
    );

    let reconnected = connected_store(&url).await;
    let links = reconnected
        .export_intake_links(export.export_id)
        .await
        .expect("load export intake links after reconnect");
    assert!(
        links.iter().any(|link| {
            link.item_id == linked_item.item_id
                && link.target_sheet_version_id == Some(sheet.version_id)
                && link.target_collection_id == Some(collection.collection_id)
                && !link.version_agnostic
        }),
        "versioned intake sheet/collection targets must survive export and reconnect"
    );
    assert!(
        links.iter().any(|link| {
            link.item_id == version_agnostic_item.item_id
                && link.target_sheet_version_id.is_none()
                && link.target_collection_id == Some(collection.collection_id)
                && link.version_agnostic
        }),
        "version-agnostic collection link must survive export and reconnect"
    );
    assert_eq!(
        reconnected
            .count_events(export_event_family::EXPORT_INTAKE_LINK_ATTACHED)
            .await
            .expect("count export intake link events after attach"),
        before_events + 2,
        "each attached intake export link emits an EventLedger event"
    );
    assert!(
        event_family::ALL.contains(&export_event_family::EXPORT_INTAKE_LINK_ATTACHED),
        "export intake link events must be surfaced in atelier event-family coverage"
    );
}

#[tokio::test]
async fn atelier_collections_membership_dedup_order_and_contact_sheet() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_collections_membership_dedup_order_and_contact_sheet: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    // --- create a collection ---
    let collection = store
        .create_collection(&NewCollection {
            name: format!("collection-{}", Uuid::new_v4()),
            notes: "core-data test collection".to_string(),
            tags: vec![
                "test".to_string(),
                "  test  ".to_string(),
                "blonde".to_string(),
            ],
            character_internal_id: None,
            sheet_version_id: None,
        })
        .await
        .expect("create collection");
    assert_eq!(
        collection.tags,
        vec!["test".to_string(), "blonde".to_string()],
        "tags are trimmed and de-duplicated on create"
    );

    // --- materialize media assets first, then add to the collection in order ---
    let asset_a = fresh_asset(&store).await;
    let asset_b = fresh_asset(&store).await;
    let asset_c = fresh_asset(&store).await;

    let inserted = store
        .add_images_to_collection(collection.collection_id, &[asset_a, asset_b, asset_c])
        .await
        .expect("add three images");
    assert_eq!(inserted, 3, "three distinct assets inserted");

    // --- re-adding the same assets does not duplicate ---
    let inserted_again = store
        .add_images_to_collection(collection.collection_id, &[asset_a, asset_b])
        .await
        .expect("re-add existing images");
    assert_eq!(
        inserted_again, 0,
        "re-adding existing memberships inserts nothing (ON CONFLICT DO NOTHING)"
    );

    // --- list ordering follows insertion sort_order ---
    let members = store
        .list_collection_images(collection.collection_id)
        .await
        .expect("list collection images");
    assert_eq!(
        members.len(),
        3,
        "membership is exactly three (no duplicates)"
    );
    assert_eq!(
        members[0].asset_id, asset_a,
        "first inserted is first in order"
    );
    assert_eq!(members[1].asset_id, asset_b);
    assert_eq!(members[2].asset_id, asset_c);
    assert_eq!(
        members[0].sort_order, 0,
        "sort_order starts at 0 and increments"
    );
    assert_eq!(members[1].sort_order, 1);
    assert_eq!(members[2].sort_order, 2);

    // --- create a contact sheet manifest snapshotting the membership ---
    let sheet = store
        .create_contact_sheet(
            &format!("sheet-{}", Uuid::new_v4()),
            "manual",
            None,
            &[asset_a, asset_b, asset_c],
            &["proof".to_string()],
            None,
            None,
        )
        .await
        .expect("create contact sheet");
    assert_eq!(sheet.image_count, 3, "manifest captured all three images");
    assert_eq!(sheet.source_type, "manual");
    assert_eq!(
        sheet.manifest.get("schema").and_then(|v| v.as_str()),
        Some("hsk.atelier.contact_sheet@1"),
        "contact-sheet manifests use the Handshake schema namespace"
    );
    let manifest_text = sheet.manifest.to_string().to_ascii_lowercase();
    let forbidden_schema_prefix = ["c", "kc."].concat();
    let forbidden_brand_token = ["cast", "kit"].concat();
    assert!(
        !manifest_text.contains(&forbidden_schema_prefix)
            && !manifest_text.contains(&forbidden_brand_token),
        "contact-sheet manifests must not persist legacy namespace text"
    );
    let items = sheet
        .manifest
        .get("items")
        .and_then(|v| v.as_array())
        .expect("manifest items array");
    assert_eq!(items.len(), 3, "manifest items snapshot the membership");
    // The snapshot records content hashes so the sheet stays reproducible.
    assert!(
        items[0]
            .get("content_hash")
            .and_then(|v| v.as_str())
            .is_some(),
        "each manifest item records its content_hash"
    );
}

#[tokio::test]
async fn atelier_collection_batch_metadata_applies_tags_to_members_preserving_photo_tags() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_collection_batch_metadata_applies_tags_to_members_preserving_photo_tags: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let collection = store
        .create_collection(&NewCollection {
            name: format!("mt-035-collection-{}", Uuid::new_v4()),
            notes: "mt-035 collection metadata batch".to_string(),
            tags: vec![
                " portfolio ".to_string(),
                "moodboard".to_string(),
                "portfolio".to_string(),
            ],
            character_internal_id: None,
            sheet_version_id: None,
        })
        .await
        .expect("create tagged collection for metadata batch");
    assert_eq!(
        collection.tags,
        vec!["portfolio".to_string(), "moodboard".to_string()],
        "collection tags are trimmed and de-duplicated before batch application"
    );
    let asset_a = fresh_asset(&store).await;
    let asset_b = fresh_asset(&store).await;
    store
        .add_images_to_collection(collection.collection_id, &[asset_a, asset_b])
        .await
        .expect("add collection members for metadata batch");
    store
        .tag_media_asset(asset_a, "existing-photo-tag", "mt-035-manual")
        .await
        .expect("seed pre-existing photo tag");

    let before_events = store
        .count_events(collections_event_family::COLLECTION_METADATA_APPLIED)
        .await
        .expect("count collection metadata application events before");
    let application = store
        .apply_collection_metadata_to_images(&CollectionMetadataApplicationRequest {
            collection_id: collection.collection_id,
            requested_by: "mt-035-operator".to_string(),
            remove_tags: Vec::new(),
        })
        .await
        .expect("apply collection metadata to member photos");
    assert_eq!(application.collection_id, collection.collection_id);
    assert_eq!(application.affected_asset_count, 2);
    assert_eq!(
        application.applied_tags,
        vec!["portfolio".to_string(), "moodboard".to_string()],
        "applied media tags are normalized through the shared tag dictionary"
    );
    assert!(
        application.removed_tags.is_empty(),
        "plain metadata application must not remove existing photo tags"
    );

    let asset_a_tags = store
        .list_media_asset_tags(asset_a)
        .await
        .expect("list tags for first asset after collection metadata application");
    let asset_a_text: Vec<String> = asset_a_tags.iter().map(|tag| tag.text.clone()).collect();
    assert!(
        asset_a_text.contains(&"existing-photo-tag".to_string()),
        "pre-existing photo tags must survive collection metadata application"
    );
    assert!(asset_a_text.contains(&"portfolio".to_string()));
    assert!(asset_a_text.contains(&"moodboard".to_string()));

    let asset_b_tags = store
        .list_media_asset_tags(asset_b)
        .await
        .expect("list tags for second asset after collection metadata application");
    let asset_b_text: Vec<String> = asset_b_tags.iter().map(|tag| tag.text.clone()).collect();
    assert_eq!(
        asset_b_text,
        vec!["moodboard".to_string(), "portfolio".to_string()],
        "collection tags apply to every member photo"
    );

    let removal = store
        .apply_collection_metadata_to_images(&CollectionMetadataApplicationRequest {
            collection_id: collection.collection_id,
            requested_by: "mt-035-operator".to_string(),
            remove_tags: vec!["portfolio".to_string()],
        })
        .await
        .expect("explicitly remove one collection-applied photo tag");
    assert_eq!(removal.removed_tags, vec!["portfolio".to_string()]);
    let asset_a_after_removal: Vec<String> = store
        .list_media_asset_tags(asset_a)
        .await
        .expect("list tags for first asset after explicit removal")
        .iter()
        .map(|tag| tag.text.clone())
        .collect();
    assert!(
        !asset_a_after_removal.contains(&"portfolio".to_string()),
        "explicitly removed photo tag is detached from member photos"
    );
    assert!(
        asset_a_after_removal.contains(&"existing-photo-tag".to_string()),
        "unmentioned existing photo tags still survive explicit removal of a different tag"
    );
    assert!(asset_a_after_removal.contains(&"moodboard".to_string()));
    assert_eq!(
        store
            .count_events(collections_event_family::COLLECTION_METADATA_APPLIED)
            .await
            .expect("count collection metadata application events after"),
        before_events + 2,
        "each collection metadata batch application emits an EventLedger event"
    );
    assert!(
        event_family::ALL.contains(&collections_event_family::COLLECTION_METADATA_APPLIED),
        "collection metadata batch events must be surfaced in atelier event-family coverage"
    );
}

#[tokio::test]
async fn atelier_contact_sheet_svg_artifact_regenerates_from_manifest() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_contact_sheet_svg_artifact_regenerates_from_manifest: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let collection = store
        .create_collection(&NewCollection {
            name: format!("mt-036-svg-collection-{}", Uuid::new_v4()),
            notes: "mt-036 svg contact sheet".to_string(),
            tags: vec!["svg-proof".to_string()],
            character_internal_id: None,
            sheet_version_id: None,
        })
        .await
        .expect("create collection for svg contact sheet");
    let asset_a = fresh_asset(&store).await;
    let asset_b = fresh_asset(&store).await;
    let asset_c = fresh_asset(&store).await;
    store
        .add_images_to_collection(collection.collection_id, &[asset_a, asset_b, asset_c])
        .await
        .expect("add members for svg contact sheet");
    let sheet = store
        .create_contact_sheet(
            &format!("mt-036-svg-sheet-{}", Uuid::new_v4()),
            "collection",
            Some(collection.collection_id),
            &[],
            &["svg-proof".to_string()],
            None,
            None,
        )
        .await
        .expect("create collection-sourced contact sheet");

    let before_events = store
        .count_events(collections_event_family::CONTACT_SHEET_SVG_RENDERED)
        .await
        .expect("count contact sheet svg events before render");
    let rendered = store
        .render_contact_sheet_svg_artifact(sheet.sheet_id)
        .await
        .expect("render reproducible contact sheet svg artifact");
    assert_eq!(rendered.sheet_id, sheet.sheet_id);
    assert_eq!(rendered.image_count, 3);
    assert!(rendered
        .artifact_ref
        .starts_with("artifact://atelier/contact-sheet-svg/"));
    assert!(rendered.content_hash.starts_with("sha256:"));
    assert!(
        rendered.svg_text.starts_with("<svg "),
        "contact sheet SVG artifact must be real SVG text"
    );
    for member in [asset_a, asset_b, asset_c] {
        assert!(
            rendered
                .svg_text
                .contains(&format!("data-asset-id=\"{member}\"")),
            "SVG must carry each source image id from the manifest"
        );
    }
    let manifest_items = sheet
        .manifest
        .get("items")
        .and_then(serde_json::Value::as_array)
        .expect("contact sheet manifest has item array");
    for item in manifest_items {
        let content_hash = item
            .get("content_hash")
            .and_then(serde_json::Value::as_str)
            .expect("manifest item content hash");
        assert!(
            rendered.svg_text.contains(content_hash),
            "SVG must be regenerable from manifest content hashes"
        );
    }

    let rendered_again = store
        .render_contact_sheet_svg_artifact(sheet.sheet_id)
        .await
        .expect("render contact sheet svg artifact idempotently");
    assert_eq!(rendered_again.svg_artifact_id, rendered.svg_artifact_id);
    assert_eq!(rendered_again.content_hash, rendered.content_hash);
    assert_eq!(rendered_again.svg_text, rendered.svg_text);

    let reconnected = connected_store(&url).await;
    let after_reconnect = reconnected
        .render_contact_sheet_svg_artifact(sheet.sheet_id)
        .await
        .expect("reload deterministic svg artifact after reconnect");
    assert_eq!(after_reconnect.svg_artifact_id, rendered.svg_artifact_id);
    assert_eq!(after_reconnect.svg_text, rendered.svg_text);
    assert_eq!(
        reconnected
            .count_events(collections_event_family::CONTACT_SHEET_SVG_RENDERED)
            .await
            .expect("count contact sheet svg events after render"),
        before_events + 1,
        "only the first SVG materialization emits an EventLedger event"
    );
    assert!(
        event_family::ALL.contains(&collections_event_family::CONTACT_SHEET_SVG_RENDERED),
        "contact sheet SVG render events must be surfaced in atelier event-family coverage"
    );
}

#[tokio::test]
async fn atelier_contact_sheet_raster_export_is_planned_without_fake_output() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_contact_sheet_raster_export_is_planned_without_fake_output: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let asset = fresh_asset(&store).await;
    let sheet = store
        .create_contact_sheet(
            "mt-037-raster-plan",
            "manual",
            None,
            &[asset],
            &["raster-plan".to_string()],
            None,
            None,
        )
        .await
        .expect("create contact sheet for raster export plan proof");

    let event_count_before = store
        .count_events(export_event_family::CONTACT_SHEET_RASTER_EXPORT_PLANNED)
        .await
        .expect("count raster export planned events before");

    let png = store
        .plan_contact_sheet_raster_export(
            sheet.sheet_id,
            ContactSheetRasterExportFormat::Png,
            "mt-037-operator",
        )
        .await
        .expect("record planned PNG contact sheet export");
    let jpg = store
        .plan_contact_sheet_raster_export(
            sheet.sheet_id,
            ContactSheetRasterExportFormat::Jpg,
            "mt-037-operator",
        )
        .await
        .expect("record planned JPG contact sheet export");
    let png_again = store
        .plan_contact_sheet_raster_export(
            sheet.sheet_id,
            ContactSheetRasterExportFormat::Png,
            "mt-037-operator-repeat",
        )
        .await
        .expect("planned PNG contact sheet export remains idempotent");

    assert_eq!(png.sheet_id, sheet.sheet_id);
    assert_eq!(png.format, ContactSheetRasterExportFormat::Png);
    assert_eq!(png.status, ContactSheetRasterExportStatus::Planned);
    assert_eq!(jpg.format, ContactSheetRasterExportFormat::Jpg);
    assert_eq!(jpg.status, ContactSheetRasterExportStatus::Planned);
    assert_eq!(
        png.plan_id, png_again.plan_id,
        "repeat planning must not create duplicate fake outputs"
    );
    assert!(
        png.reason.contains("deferred") && png.reason.contains("PNG/JPG"),
        "planned marker should explain that raster rendering is deferred"
    );

    let artifact_columns: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::BIGINT
           FROM information_schema.columns
           WHERE table_schema = ANY(current_schemas(false))
             AND table_name = 'atelier_contact_sheet_raster_export_plan'
             AND column_name IN ('artifact_ref', 'content_hash', 'byte_len')"#,
    )
    .fetch_one(store.pool())
    .await
    .expect("inspect raster plan columns");
    assert_eq!(
        artifact_columns, 0,
        "planned raster hook must not model fake rendered artifact output"
    );

    let reconnected = connected_store(&url).await;
    let plans = reconnected
        .list_contact_sheet_raster_export_plans(sheet.sheet_id)
        .await
        .expect("list persisted raster export plans");
    let formats: Vec<ContactSheetRasterExportFormat> =
        plans.iter().map(|plan| plan.format).collect();
    assert_eq!(plans.len(), 2);
    assert!(formats.contains(&ContactSheetRasterExportFormat::Png));
    assert!(formats.contains(&ContactSheetRasterExportFormat::Jpg));

    let event_count_after = reconnected
        .count_events(export_event_family::CONTACT_SHEET_RASTER_EXPORT_PLANNED)
        .await
        .expect("count raster export planned events after");
    assert_eq!(
        event_count_after,
        event_count_before + 2,
        "one event per newly planned format; idempotent repeat emits none"
    );
    assert!(
        event_family::ALL.contains(&export_event_family::CONTACT_SHEET_RASTER_EXPORT_PLANNED),
        "parent atelier event registry must expose raster contact-sheet export planning"
    );
}

#[tokio::test]
async fn atelier_character_documents_preserve_types_versions_and_raw_text() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_character_documents_preserve_types_versions_and_raw_text: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-docs-{}", Uuid::new_v4()),
            display_name: "Documented Subject".to_string(),
        })
        .await
        .expect("create character for document proof");

    let note_body = "  raw note line one\n\nline two with *markdown* left untouched  ";
    let note_v1 = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Daily note".to_string(),
            body_raw_text: note_body.to_string(),
            tags: vec!["research".to_string(), "research".to_string()],
            author: "mt-038-author".to_string(),
        })
        .await
        .expect("create note document");
    let story_v1 = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Story,
            title: "Opening beat".to_string(),
            body_raw_text: "story body\nwith dialogue".to_string(),
            tags: vec!["story".to_string()],
            author: "mt-038-author".to_string(),
        })
        .await
        .expect("create story document");
    let moodboard_v1 = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Moodboard,
            title: "Palette notes".to_string(),
            body_raw_text: "moodboard doc raw text: #112233 / silk / backlight".to_string(),
            tags: vec!["moodboard".to_string()],
            author: "mt-038-author".to_string(),
        })
        .await
        .expect("create moodboard document");

    assert_eq!(note_v1.version_seq, 1);
    assert_eq!(note_v1.body_raw_text, note_body);
    assert_eq!(story_v1.version_seq, 1);
    assert_eq!(moodboard_v1.version_seq, 1);

    let note_doc = store
        .get_character_document(note_v1.document_id)
        .await
        .expect("get note document metadata");
    assert_eq!(note_doc.character_internal_id, character.internal_id);
    assert_eq!(note_doc.doc_type, CharacterDocumentType::Note);
    assert_eq!(note_doc.title, "Daily note");
    assert_eq!(note_doc.tags, vec!["research".to_string()]);
    assert_eq!(note_doc.current_version_id, note_v1.version_id);
    assert_eq!(note_doc.current_version_seq, 1);

    let note_v2_body = "second version keeps raw spacing\n  indented line\n";
    let note_v2 = store
        .append_character_document_version(
            note_v1.document_id,
            &AppendCharacterDocumentVersion {
                title: "Daily note revised".to_string(),
                body_raw_text: note_v2_body.to_string(),
                tags: vec!["research".to_string(), "revision".to_string()],
                author: "mt-038-editor".to_string(),
            },
        )
        .await
        .expect("append note document version");
    assert_eq!(note_v2.version_seq, 2);
    assert_eq!(note_v2.parent_version_id, Some(note_v1.version_id));
    assert_eq!(note_v2.body_raw_text, note_v2_body);

    let history = store
        .character_document_history(note_v1.document_id)
        .await
        .expect("document version history");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].version_id, note_v1.version_id);
    assert_eq!(history[1].version_id, note_v2.version_id);

    let all_docs = store
        .list_character_documents(character.internal_id, None)
        .await
        .expect("list all character documents");
    assert_eq!(all_docs.len(), 3);
    let story_docs = store
        .list_character_documents(character.internal_id, Some(CharacterDocumentType::Story))
        .await
        .expect("list story documents");
    assert_eq!(story_docs.len(), 1);
    assert_eq!(story_docs[0].document_id, story_v1.document_id);

    let reconnected = connected_store(&url).await;
    let latest = reconnected
        .latest_character_document_version(note_v1.document_id)
        .await
        .expect("latest document version after reconnect")
        .expect("note document has latest version");
    assert_eq!(latest.version_id, note_v2.version_id);
    assert_eq!(latest.body_raw_text, note_v2_body);

    for document_id in [
        note_v1.document_id,
        story_v1.document_id,
        moodboard_v1.document_id,
    ] {
        let created_count = reconnected
            .count_events_for_aggregate(
                documents_event_family::CHARACTER_DOCUMENT_CREATED,
                "atelier_character_document",
                &document_id.to_string(),
            )
            .await
            .expect("count document created events for aggregate");
        assert_eq!(created_count, 1);
    }
    let note_version_events = reconnected
        .count_events_for_aggregate(
            documents_event_family::CHARACTER_DOCUMENT_VERSION_APPENDED,
            "atelier_character_document",
            &note_v1.document_id.to_string(),
        )
        .await
        .expect("count document version events for aggregate");
    assert_eq!(note_version_events, 1);
    assert!(
        event_family::ALL.contains(&documents_event_family::CHARACTER_DOCUMENT_CREATED),
        "parent atelier event registry must expose character document creation"
    );
    assert!(
        event_family::ALL.contains(&documents_event_family::CHARACTER_DOCUMENT_VERSION_APPENDED),
        "parent atelier event registry must expose character document versioning"
    );
}

#[tokio::test]
async fn atelier_story_cards_and_beats_round_trip_order_text_and_ids() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_story_cards_and_beats_round_trip_order_text_and_ids: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-story-cards-{}", Uuid::new_v4()),
            display_name: "Story Card Subject".to_string(),
        })
        .await
        .expect("create character for story card proof");
    let story_doc = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Story,
            title: "Storyboard root".to_string(),
            body_raw_text: "root story document".to_string(),
            tags: vec!["story".to_string()],
            author: "mt-039-author".to_string(),
        })
        .await
        .expect("create story document for cards");

    let card_one_text = "card one raw text\n  indented shot";
    let card_one = store
        .add_story_card(&NewStoryCard {
            story_document_id: story_doc.document_id,
            title: "Card one".to_string(),
            body_raw_text: card_one_text.to_string(),
            tags: vec!["setup".to_string()],
        })
        .await
        .expect("add first story card");
    let card_two_text = "card two raw text\nwith exact punctuation!";
    let card_two = store
        .add_story_card(&NewStoryCard {
            story_document_id: story_doc.document_id,
            title: "Card two".to_string(),
            body_raw_text: card_two_text.to_string(),
            tags: vec!["payoff".to_string()],
        })
        .await
        .expect("add second story card");
    assert_eq!(card_one.seq, 1);
    assert_eq!(card_two.seq, 2);
    assert_eq!(card_one.body_raw_text, card_one_text);
    assert_eq!(card_two.body_raw_text, card_two_text);

    let beat_one_text = "Beat one: establish the look.";
    let beat_one = store
        .add_story_beat(&NewStoryBeat {
            story_document_id: story_doc.document_id,
            card_id: Some(card_one.card_id),
            beat_text: beat_one_text.to_string(),
        })
        .await
        .expect("add first story beat");
    let beat_two_text = "Beat two: hold on the reaction.\nNo normalization.";
    let beat_two = store
        .add_story_beat(&NewStoryBeat {
            story_document_id: story_doc.document_id,
            card_id: Some(card_two.card_id),
            beat_text: beat_two_text.to_string(),
        })
        .await
        .expect("add second story beat");
    assert_eq!(beat_one.seq, 1);
    assert_eq!(beat_two.seq, 2);
    assert_eq!(beat_one.beat_text, beat_one_text);
    assert_eq!(beat_two.beat_text, beat_two_text);

    let cards = store
        .list_story_cards(story_doc.document_id)
        .await
        .expect("list story cards");
    assert_eq!(
        cards.iter().map(|card| card.card_id).collect::<Vec<_>>(),
        vec![card_one.card_id, card_two.card_id]
    );
    assert_eq!(cards[0].body_raw_text, card_one_text);
    assert_eq!(cards[1].body_raw_text, card_two_text);

    let beats = store
        .list_story_beats(story_doc.document_id)
        .await
        .expect("list story beats");
    assert_eq!(
        beats.iter().map(|beat| beat.beat_id).collect::<Vec<_>>(),
        vec![beat_one.beat_id, beat_two.beat_id]
    );
    assert_eq!(beats[0].card_id, Some(card_one.card_id));
    assert_eq!(beats[1].card_id, Some(card_two.card_id));

    let reconnected = connected_store(&url).await;
    let cards_after = reconnected
        .list_story_cards(story_doc.document_id)
        .await
        .expect("list story cards after reconnect");
    let beats_after = reconnected
        .list_story_beats(story_doc.document_id)
        .await
        .expect("list story beats after reconnect");
    assert_eq!(cards_after[0].card_id, card_one.card_id);
    assert_eq!(cards_after[1].card_id, card_two.card_id);
    assert_eq!(beats_after[0].beat_id, beat_one.beat_id);
    assert_eq!(beats_after[1].beat_id, beat_two.beat_id);
    assert_eq!(beats_after[1].beat_text, beat_two_text);

    let card_events = reconnected
        .count_events_for_aggregate(
            documents_event_family::STORY_CARD_ADDED,
            "atelier_character_document",
            &story_doc.document_id.to_string(),
        )
        .await
        .expect("count story card events for aggregate");
    let beat_events = reconnected
        .count_events_for_aggregate(
            documents_event_family::STORY_BEAT_ADDED,
            "atelier_character_document",
            &story_doc.document_id.to_string(),
        )
        .await
        .expect("count story beat events for aggregate");
    assert_eq!(card_events, 2);
    assert_eq!(beat_events, 2);
    assert!(
        event_family::ALL.contains(&documents_event_family::STORY_CARD_ADDED),
        "parent atelier event registry must expose story card additions"
    );
    assert!(
        event_family::ALL.contains(&documents_event_family::STORY_BEAT_ADDED),
        "parent atelier event registry must expose story beat additions"
    );
}

#[tokio::test]
async fn atelier_story_cards_and_beats_reject_wrong_document_scope() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_story_cards_and_beats_reject_wrong_document_scope: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-story-guard-{}", Uuid::new_v4()),
            display_name: "Story Guard Subject".to_string(),
        })
        .await
        .expect("create character for story guard proof");
    let note_doc = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Non-story document".to_string(),
            body_raw_text: "note only".to_string(),
            tags: vec![],
            author: "mt-039-author".to_string(),
        })
        .await
        .expect("create note document");
    let story_one = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Story,
            title: "Story one".to_string(),
            body_raw_text: "story one".to_string(),
            tags: vec![],
            author: "mt-039-author".to_string(),
        })
        .await
        .expect("create first story document");
    let story_two = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Story,
            title: "Story two".to_string(),
            body_raw_text: "story two".to_string(),
            tags: vec![],
            author: "mt-039-author".to_string(),
        })
        .await
        .expect("create second story document");

    let non_story_card = store
        .add_story_card(&NewStoryCard {
            story_document_id: note_doc.document_id,
            title: "Invalid card".to_string(),
            body_raw_text: "must not attach to note".to_string(),
            tags: vec![],
        })
        .await
        .expect_err("API must reject card attached to a note document");
    assert!(
        non_story_card
            .to_string()
            .contains("must be a story document"),
        "unexpected non-story card error: {non_story_card}"
    );

    let non_story_beat = store
        .add_story_beat(&NewStoryBeat {
            story_document_id: note_doc.document_id,
            card_id: None,
            beat_text: "must not attach to note".to_string(),
        })
        .await
        .expect_err("API must reject beat attached to a note document");
    assert!(
        non_story_beat
            .to_string()
            .contains("must be a story document"),
        "unexpected non-story beat error: {non_story_beat}"
    );

    let card = store
        .add_story_card(&NewStoryCard {
            story_document_id: story_one.document_id,
            title: "Story one card".to_string(),
            body_raw_text: "story one card text".to_string(),
            tags: vec![],
        })
        .await
        .expect("create story one card");
    let cross_story_beat = store
        .add_story_beat(&NewStoryBeat {
            story_document_id: story_two.document_id,
            card_id: Some(card.card_id),
            beat_text: "must not cross story documents".to_string(),
        })
        .await
        .expect_err("API must reject beat linked to a card from another story");
    assert!(
        cross_story_beat.to_string().contains("does not belong"),
        "unexpected cross-story beat error: {cross_story_beat}"
    );

    let direct_non_story_card = sqlx::query(
        r#"INSERT INTO atelier_story_card
             (story_document_id, seq, title, body_raw_text)
           VALUES ($1, 1, 'invalid direct card', 'must be rejected')"#,
    )
    .bind(note_doc.document_id)
    .execute(store.pool())
    .await;
    assert!(
        direct_non_story_card.is_err(),
        "database trigger must reject direct card rows for non-story documents"
    );

    let direct_cross_story_beat = sqlx::query(
        r#"INSERT INTO atelier_story_beat
             (story_document_id, card_id, seq, beat_text)
           VALUES ($1, $2, 1, 'must be rejected')"#,
    )
    .bind(story_two.document_id)
    .bind(card.card_id)
    .execute(store.pool())
    .await;
    assert!(
        direct_cross_story_beat.is_err(),
        "database trigger must reject direct beat rows whose card belongs to another story"
    );
}

#[tokio::test]
async fn atelier_story_cards_and_beats_parallel_writes_keep_dense_order() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_story_cards_and_beats_parallel_writes_keep_dense_order: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-story-parallel-{}", Uuid::new_v4()),
            display_name: "Story Parallel Subject".to_string(),
        })
        .await
        .expect("create character for story parallel proof");
    let story_doc = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Story,
            title: "Parallel story".to_string(),
            body_raw_text: "parallel story document".to_string(),
            tags: vec![],
            author: "mt-039-author".to_string(),
        })
        .await
        .expect("create story document for parallel proof");

    let mut card_handles = Vec::new();
    for index in 0..8 {
        let store = store.clone();
        let story_document_id = story_doc.document_id;
        card_handles.push(tokio::spawn(async move {
            store
                .add_story_card(&NewStoryCard {
                    story_document_id,
                    title: format!("Parallel card {index}"),
                    body_raw_text: format!("parallel card raw text {index}"),
                    tags: vec![],
                })
                .await
                .expect("parallel card insert")
                .seq
        }));
    }
    let mut card_seqs = Vec::new();
    for handle in card_handles {
        card_seqs.push(handle.await.expect("parallel card task joins"));
    }
    card_seqs.sort_unstable();
    assert_eq!(card_seqs, (1..=8).collect::<Vec<_>>());

    let mut beat_handles = Vec::new();
    for index in 0..8 {
        let store = store.clone();
        let story_document_id = story_doc.document_id;
        beat_handles.push(tokio::spawn(async move {
            store
                .add_story_beat(&NewStoryBeat {
                    story_document_id,
                    card_id: None,
                    beat_text: format!("parallel beat raw text {index}"),
                })
                .await
                .expect("parallel beat insert")
                .seq
        }));
    }
    let mut beat_seqs = Vec::new();
    for handle in beat_handles {
        beat_seqs.push(handle.await.expect("parallel beat task joins"));
    }
    beat_seqs.sort_unstable();
    assert_eq!(beat_seqs, (1..=8).collect::<Vec<_>>());
}

#[tokio::test]
async fn atelier_moodboard_schema_layer_model_round_trips_full_structure() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_moodboard_schema_layer_model_round_trips_full_structure: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-moodboard-{}", Uuid::new_v4()),
            display_name: "Moodboard Subject".to_string(),
        })
        .await
        .expect("create character for moodboard proof");
    let moodboard_doc = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Moodboard,
            title: "Visual Direction".to_string(),
            body_raw_text: "moodboard source shell text stays separate".to_string(),
            tags: vec!["moodboard".to_string()],
            author: "mt-042-author".to_string(),
        })
        .await
        .expect("create moodboard document");
    let note_doc = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Not a moodboard".to_string(),
            body_raw_text: "note".to_string(),
            tags: vec![],
            author: "mt-042-author".to_string(),
        })
        .await
        .expect("create non-moodboard document");

    let asset_id = fresh_asset(&store).await;
    let board_id = Uuid::new_v4();
    let base_layer_id = Uuid::new_v4();
    let overlay_layer_id = Uuid::new_v4();
    let image_id = Uuid::new_v4();
    let text_id = Uuid::new_v4();
    let shape_id = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    let folder_id = Uuid::new_v4();
    let guide_id = Uuid::new_v4();
    let history_id = Uuid::new_v4();
    let raw_moodboard_json = serde_json::to_string_pretty(&serde_json::json!({
        "schema_id": MOODBOARD_SCHEMA_ID,
        "schema_version": 1,
        "moodboard_id": board_id,
        "name": "Golden hour direction",
        "description": "Reference board with grouped layers and connectors",
        "canvas": {
            "width": 3840,
            "height": 2160,
            "background_color": "#101010"
        },
        "layers": [
            {
                "layer_id": base_layer_id,
                "name": "Base references",
                "order": 1,
                "visible": true,
                "locked": false,
                "opacity": 1.0,
                "parent_layer_id": null
            },
            {
                "layer_id": overlay_layer_id,
                "name": "Notes and connectors",
                "order": 2,
                "visible": true,
                "locked": false,
                "opacity": 0.85,
                "parent_layer_id": base_layer_id
            }
        ],
        "images": [
            {
                "element_id": image_id,
                "layer_id": base_layer_id,
                "asset_id": asset_id,
                "source": "local",
                "url": null,
                "position": { "x": 120.0, "y": 80.0 },
                "size": { "width": 640.0, "height": 480.0 },
                "rotation": 0.0,
                "opacity": 1.0,
                "flags": { "hero_reference": true }
            }
        ],
        "text": [
            {
                "element_id": text_id,
                "layer_id": overlay_layer_id,
                "content": "warm backlight, low contrast",
                "font": "Inter",
                "font_size": 32.0,
                "color": "#f5d38a",
                "position": { "x": 840.0, "y": 120.0 },
                "rotation": -2.0,
                "flags": { "operator_note": true }
            }
        ],
        "shapes": [
            {
                "element_id": shape_id,
                "layer_id": overlay_layer_id,
                "shape_type": "rectangle",
                "position": { "x": 96.0, "y": 64.0 },
                "size": { "width": 700.0, "height": 520.0 },
                "rotation": 0.0,
                "fill": "#00000000",
                "stroke": "#f5d38a",
                "stroke_width": 4.0,
                "flags": { "callout": true }
            }
        ],
        "connectors": [
            {
                "connector_id": connector_id,
                "layer_id": overlay_layer_id,
                "from_element_id": text_id,
                "to_element_id": image_id,
                "points": [
                    { "x": 820.0, "y": 160.0 },
                    { "x": 760.0, "y": 220.0 }
                ],
                "style": { "stroke": "#f5d38a", "arrow": "end" }
            }
        ],
        "folders": [
            {
                "folder_id": folder_id,
                "name": "hero refs",
                "collapsed": false,
                "children": [image_id, text_id, shape_id, connector_id]
            }
        ],
        "guides": [
            {
                "guide_id": guide_id,
                "axis": "vertical",
                "position": 1920.0,
                "locked": true,
                "label": "center"
            }
        ],
        "flags": {
            "locked": false,
            "archived": false,
            "operator_reviewed": true
        },
        "style": {
            "dominant_colors": ["#101010", "#f5d38a"],
            "mood_keywords": ["golden", "intimate", "cinematic"],
            "style_description": "Golden backlight with warm highlights",
            "suggested_presets": [Uuid::new_v4()]
        },
        "history": [
            {
                "history_id": history_id,
                "at": "2026-06-08T09:00:00Z",
                "actor": "mt-042-test",
                "operation": "create",
                "summary": "created complete schema v1 moodboard"
            }
        ]
    }))
    .expect("serialize moodboard fixture");

    let snapshot = store
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: moodboard_doc.document_id,
            raw_json_text: raw_moodboard_json.clone(),
            author: "mt-042-author".to_string(),
        })
        .await
        .expect("record moodboard snapshot");
    assert_eq!(snapshot.document_id, moodboard_doc.document_id);
    assert_eq!(snapshot.document_version_id, moodboard_doc.version_id);
    assert_eq!(snapshot.schema_id, MOODBOARD_SCHEMA_ID);
    assert_eq!(snapshot.schema_version, 1);
    assert_eq!(snapshot.raw_json_text, raw_moodboard_json);
    assert_eq!(snapshot.moodboard.moodboard_id, board_id);
    assert_eq!(snapshot.moodboard.layers.len(), 2);
    assert_eq!(snapshot.moodboard.images[0].asset_id, Some(asset_id));
    assert_eq!(
        snapshot.moodboard.text[0].content,
        "warm backlight, low contrast"
    );
    assert_eq!(snapshot.moodboard.shapes[0].shape_type, "rectangle");
    assert_eq!(snapshot.moodboard.connectors[0].from_element_id, text_id);
    assert_eq!(snapshot.moodboard.folders[0].children.len(), 4);
    assert_eq!(snapshot.moodboard.guides[0].axis, "vertical");
    assert!(snapshot.moodboard.flags.operator_reviewed);
    assert_eq!(snapshot.moodboard.style.dominant_colors.len(), 2);
    assert_eq!(snapshot.moodboard.history[0].operation, "create");

    let reconnected = connected_store(&url).await;
    let loaded = reconnected
        .latest_moodboard_snapshot(moodboard_doc.document_id)
        .await
        .expect("load moodboard snapshot")
        .expect("moodboard snapshot exists");
    assert_eq!(loaded.raw_json_text, snapshot.raw_json_text);
    assert_eq!(loaded.moodboard, snapshot.moodboard);
    assert_eq!(loaded.moodboard_json, snapshot.moodboard_json);

    let duplicate = reconnected
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: moodboard_doc.document_id,
            raw_json_text: snapshot.raw_json_text.clone(),
            author: "mt-042-author".to_string(),
        })
        .await
        .expect("idempotent duplicate moodboard snapshot");
    assert_eq!(duplicate.snapshot_id, snapshot.snapshot_id);

    let snapshot_events = reconnected
        .count_events_for_aggregate(
            moodboard_event_family::MOODBOARD_SNAPSHOT_RECORDED,
            "atelier_character_document",
            &moodboard_doc.document_id.to_string(),
        )
        .await
        .expect("count moodboard snapshot events");
    assert_eq!(snapshot_events, 1);
    assert!(
        event_family::ALL.contains(&moodboard_event_family::MOODBOARD_SNAPSHOT_RECORDED),
        "parent event registry must expose moodboard snapshot records"
    );
    let event_payload: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_character_document'
             AND aggregate_id = $2
           ORDER BY created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(moodboard_event_family::MOODBOARD_SNAPSHOT_RECORDED)
    .bind(moodboard_doc.document_id.to_string())
    .fetch_one(reconnected.pool())
    .await
    .expect("read moodboard event payload");
    assert_eq!(
        event_payload["schema_id"],
        serde_json::json!(MOODBOARD_SCHEMA_ID)
    );
    assert_eq!(event_payload["schema_version"], serde_json::json!(1));
    assert_eq!(event_payload["layer_count"], serde_json::json!(2));
    assert_eq!(event_payload["image_count"], serde_json::json!(1));
    assert_eq!(event_payload["text_count"], serde_json::json!(1));
    assert_eq!(event_payload["shape_count"], serde_json::json!(1));
    assert_eq!(event_payload["connector_count"], serde_json::json!(1));
    assert_eq!(event_payload["folder_count"], serde_json::json!(1));
    assert_eq!(event_payload["guide_count"], serde_json::json!(1));
    assert_eq!(event_payload["history_count"], serde_json::json!(1));
    assert!(event_payload.get("document_version_id_ref").is_some());
    assert!(event_payload.get("content_sha256").is_some());
    let payload_text = event_payload.to_string();
    assert!(!payload_text.contains(&snapshot.raw_json_text));
    assert!(!payload_text.contains(&asset_id.to_string()));

    let mut missing_history: serde_json::Value =
        serde_json::from_str(&snapshot.raw_json_text).expect("fixture parses");
    missing_history
        .as_object_mut()
        .expect("moodboard fixture is object")
        .remove("history");
    let invalid_missing_history = reconnected
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: moodboard_doc.document_id,
            raw_json_text: serde_json::to_string(&missing_history).expect("serialize invalid"),
            author: "mt-042-author".to_string(),
        })
        .await;
    assert!(
        invalid_missing_history.is_err(),
        "schema must reject moodboards missing required history"
    );

    let mut bad_layer_ref: serde_json::Value =
        serde_json::from_str(&snapshot.raw_json_text).expect("fixture parses");
    bad_layer_ref["images"][0]["layer_id"] = serde_json::json!(Uuid::new_v4());
    let invalid_layer_ref = reconnected
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: moodboard_doc.document_id,
            raw_json_text: serde_json::to_string(&bad_layer_ref).expect("serialize invalid"),
            author: "mt-042-author".to_string(),
        })
        .await;
    assert!(
        invalid_layer_ref.is_err(),
        "layer model must reject elements pointing at unknown layers"
    );

    let wrong_doc_type = reconnected
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: note_doc.document_id,
            raw_json_text: snapshot.raw_json_text.clone(),
            author: "mt-042-author".to_string(),
        })
        .await;
    assert!(
        wrong_doc_type.is_err(),
        "moodboard snapshots must attach only to moodboard character documents"
    );

    let direct_sql_invalid = sqlx::query(
        r#"INSERT INTO atelier_moodboard
             (document_id, document_version_id, schema_id, schema_version, raw_json_text,
              moodboard_json, content_sha256, author)
           VALUES ($1, $2, $3, 1, '{"schema_id":"hsk.atelier.moodboard@1"}',
                   '{"schema_id":"hsk.atelier.moodboard@1"}'::jsonb,
                   '0000000000000000000000000000000000000000000000000000000000000000',
                   'mt-042-author')"#,
    )
    .bind(moodboard_doc.document_id)
    .bind(moodboard_doc.version_id)
    .bind(MOODBOARD_SCHEMA_ID)
    .execute(reconnected.pool())
    .await;
    assert!(
        direct_sql_invalid.is_err(),
        "database must reject direct SQL rows missing the required moodboard structure"
    );

    let mut desync_insert_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin direct SQL desync insert probe");
    let direct_sql_desync_insert = sqlx::query(
        r#"INSERT INTO atelier_moodboard
             (document_id, document_version_id, schema_id, schema_version, raw_json_text,
              moodboard_json, content_sha256, author)
           VALUES ($1, $2, $3, 1, 'not-json',
                   $4,
                   '0000000000000000000000000000000000000000000000000000000000000000',
                   'mt-042-author')"#,
    )
    .bind(moodboard_doc.document_id)
    .bind(moodboard_doc.version_id)
    .bind(MOODBOARD_SCHEMA_ID)
    .bind(&snapshot.moodboard_json)
    .execute(&mut *desync_insert_tx)
    .await;
    let _ = desync_insert_tx.rollback().await;
    assert!(
        direct_sql_desync_insert.is_err(),
        "database must reject direct SQL rows where raw JSON, JSONB projection, and hash diverge"
    );

    let mut desync_update_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin direct SQL desync update probe");
    let direct_sql_desync_update = sqlx::query(
        r#"UPDATE atelier_moodboard
           SET raw_json_text = 'not-json'
           WHERE snapshot_id = $1"#,
    )
    .bind(snapshot.snapshot_id)
    .execute(&mut *desync_update_tx)
    .await;
    let _ = desync_update_tx.rollback().await;
    assert!(
        direct_sql_desync_update.is_err(),
        "database must reject updates that break raw JSON/projection/hash round-trip integrity"
    );

    let mut doc_type_drift_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard document type drift probe");
    let doc_type_drift = sqlx::query(
        r#"UPDATE atelier_character_document
           SET doc_type = 'note'
           WHERE document_id = $1"#,
    )
    .bind(moodboard_doc.document_id)
    .execute(&mut *doc_type_drift_tx)
    .await;
    let _ = doc_type_drift_tx.rollback().await;
    assert!(
        doc_type_drift.is_err(),
        "database must reject changing a document with moodboard snapshots away from moodboard type"
    );

    let mut version_drift_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard version ownership drift probe");
    let version_drift = sqlx::query(
        r#"UPDATE atelier_character_document_version
           SET document_id = $1
           WHERE version_id = $2"#,
    )
    .bind(note_doc.document_id)
    .bind(moodboard_doc.version_id)
    .execute(&mut *version_drift_tx)
    .await;
    let _ = version_drift_tx.rollback().await;
    assert!(
        version_drift.is_err(),
        "database must reject moving a version used by a moodboard snapshot to another document"
    );
}

fn minimal_moodboard_fixture(layer_id: Uuid, text_id: Uuid) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "schema_id": MOODBOARD_SCHEMA_ID,
        "schema_version": 1,
        "moodboard_id": Uuid::new_v4(),
        "name": "Exportable moodboard",
        "description": "Compact fixture for operation and export contract proof",
        "canvas": {
            "width": 1920.0,
            "height": 1080.0,
            "background_color": "#202020"
        },
        "layers": [
            {
                "layer_id": layer_id,
                "name": "Direction notes",
                "order": 1,
                "visible": true,
                "locked": false,
                "opacity": 1.0,
                "parent_layer_id": null
            }
        ],
        "images": [],
        "text": [
            {
                "element_id": text_id,
                "layer_id": layer_id,
                "content": "preserve operation/export state",
                "font": "Inter",
                "font_size": 28.0,
                "color": "#f8f8f2",
                "position": { "x": 96.0, "y": 120.0 },
                "rotation": 0.0,
                "flags": { "operator_note": true }
            }
        ],
        "shapes": [],
        "connectors": [],
        "folders": [],
        "guides": [],
        "flags": {
            "locked": false,
            "archived": false,
            "operator_reviewed": false
        },
        "style": {
            "dominant_colors": ["#202020", "#f8f8f2"],
            "mood_keywords": ["contract", "export"],
            "style_description": "Minimal moodboard fixture for contract tests",
            "suggested_presets": []
        },
        "history": [
            {
                "history_id": Uuid::new_v4(),
                "at": "2026-06-08T10:00:00Z",
                "actor": "mt-043-test",
                "operation": "create",
                "summary": "created compact moodboard fixture"
            }
        ]
    }))
    .expect("serialize minimal moodboard fixture")
}

async fn assert_uuid_update_rejected(store: &AtelierStore, sql: &str, id: Uuid, reason: &str) {
    let mut tx = store
        .pool()
        .begin()
        .await
        .expect("begin direct SQL rejection probe");
    let result = sqlx::query(sql).bind(id).execute(&mut *tx).await;
    let _ = tx.rollback().await;
    assert!(result.is_err(), "{reason}");
}

#[tokio::test]
async fn atelier_moodboard_operations_and_export_hooks_produce_receipts_without_gov_outputs() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_moodboard_operations_and_export_hooks_produce_receipts_without_gov_outputs: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-moodboard-ops-{}", Uuid::new_v4()),
            display_name: "Moodboard Ops Subject".to_string(),
        })
        .await
        .expect("create character for moodboard operation/export proof");
    let moodboard_doc = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Moodboard,
            title: "Operation/export board".to_string(),
            body_raw_text: "moodboard operation/export source shell".to_string(),
            tags: vec!["moodboard".to_string(), "export".to_string()],
            author: "mt-043-author".to_string(),
        })
        .await
        .expect("create moodboard document for operation/export proof");
    let layer_id = Uuid::new_v4();
    let text_id = Uuid::new_v4();
    let snapshot = store
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: moodboard_doc.document_id,
            raw_json_text: minimal_moodboard_fixture(layer_id, text_id),
            author: "mt-043-author".to_string(),
        })
        .await
        .expect("record source moodboard snapshot for operation/export proof");

    let operation_events_before = store
        .count_events_for_aggregate(
            moodboard_event_family::MOODBOARD_OPERATION_RECORDED,
            "atelier_moodboard",
            &snapshot.snapshot_id.to_string(),
        )
        .await
        .expect("count moodboard operation events before");
    let export_events_before = store
        .count_events_for_aggregate(
            moodboard_event_family::MOODBOARD_EXPORT_REQUESTED,
            "atelier_moodboard",
            &snapshot.snapshot_id.to_string(),
        )
        .await
        .expect("count moodboard export events before");

    let operation = store
        .record_moodboard_operation(&NewMoodboardOperation {
            snapshot_id: snapshot.snapshot_id,
            operation_kind: MoodboardOperationKind::LayerReordered,
            operation_payload: serde_json::json!({
                "layer_id": layer_id,
                "from_order": 1,
                "to_order": 2,
                "reason": "operator reordered visual priority"
            }),
            actor: "mt-043-operator".to_string(),
        })
        .await
        .expect("record moodboard operation receipt");
    assert_eq!(operation.snapshot_id, snapshot.snapshot_id);
    assert_eq!(operation.document_id, moodboard_doc.document_id);
    assert_eq!(operation.document_version_id, moodboard_doc.version_id);
    assert_eq!(
        operation.operation_kind,
        MoodboardOperationKind::LayerReordered
    );
    assert_eq!(
        operation.receipt_json["schema"],
        serde_json::json!("hsk.atelier.moodboard_operation_receipt@1")
    );
    assert_eq!(
        operation.receipt_json["snapshot_id"],
        serde_json::json!(snapshot.snapshot_id)
    );
    assert_eq!(
        operation.receipt_json["source_content_sha256"],
        serde_json::json!(snapshot.content_sha256)
    );

    let png_export = store
        .request_moodboard_export(&NewMoodboardExportRequest {
            snapshot_id: snapshot.snapshot_id,
            format: MoodboardExportFormat::Png,
            label: Some("operator-preview".to_string()),
            requested_by: "mt-043-operator".to_string(),
        })
        .await
        .expect("request planned PNG moodboard export");
    let pdf_export = store
        .request_moodboard_export(&NewMoodboardExportRequest {
            snapshot_id: snapshot.snapshot_id,
            format: MoodboardExportFormat::Pdf,
            label: Some("operator-preview".to_string()),
            requested_by: "mt-043-operator".to_string(),
        })
        .await
        .expect("request planned PDF moodboard export");
    let png_again = store
        .request_moodboard_export(&NewMoodboardExportRequest {
            snapshot_id: snapshot.snapshot_id,
            format: MoodboardExportFormat::Png,
            label: Some("repeat-label-does-not-make-fake-output".to_string()),
            requested_by: "mt-043-repeat".to_string(),
        })
        .await
        .expect("planned PNG moodboard export remains idempotent");

    assert_eq!(png_export.export_id, png_again.export_id);
    assert_eq!(png_export.status, MoodboardExportStatus::Planned);
    assert_eq!(pdf_export.status, MoodboardExportStatus::Planned);
    assert_eq!(png_export.document_id, moodboard_doc.document_id);
    assert_eq!(png_export.document_version_id, moodboard_doc.version_id);
    assert_eq!(
        png_export.manifest_json["schema"],
        serde_json::json!("hsk.atelier.moodboard_export_manifest@1")
    );
    assert_eq!(png_export.manifest_json["format"], serde_json::json!("png"));
    assert_eq!(pdf_export.manifest_json["format"], serde_json::json!("pdf"));
    assert_eq!(
        png_export.manifest_json["status"],
        serde_json::json!("planned")
    );
    assert_eq!(
        png_export.manifest_json["source_content_sha256"],
        serde_json::json!(snapshot.content_sha256)
    );
    assert_eq!(
        png_export.receipt_json["schema"],
        serde_json::json!("hsk.atelier.moodboard_export_receipt@1")
    );
    assert_eq!(
        png_export.receipt_json["output_artifact"],
        serde_json::json!("not_produced")
    );

    let export_columns: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::BIGINT
           FROM information_schema.columns
           WHERE table_schema = ANY(current_schemas(false))
             AND table_name = 'atelier_moodboard_export_request'
             AND column_name IN (
                'artifact_ref',
                'output_path',
                'output_dir',
                'pack_path',
                'content_hash',
                'byte_len'
             )"#,
    )
    .fetch_one(store.pool())
    .await
    .expect("inspect moodboard export columns");
    assert_eq!(
        export_columns, 0,
        "moodboard export hooks must not model fake rendered artifacts or filesystem output paths"
    );
    let export_text = serde_json::json!({
        "manifest": png_export.manifest_json,
        "receipt": png_export.receipt_json
    })
    .to_string()
    .to_ascii_lowercase();
    assert!(
        !export_text.contains(".gov"),
        "moodboard export manifest/receipt must not point at .GOV outputs"
    );
    assert!(
        !export_text.contains("raw_json_text"),
        "moodboard export manifest/receipt must not duplicate raw moodboard JSON"
    );

    let reconnected = connected_store(&url).await;
    let operations = reconnected
        .list_moodboard_operations(snapshot.snapshot_id)
        .await
        .expect("list persisted moodboard operation receipts");
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].operation_id, operation.operation_id);
    let exports = reconnected
        .list_moodboard_export_requests(snapshot.snapshot_id)
        .await
        .expect("list persisted moodboard export requests");
    assert_eq!(exports.len(), 2);
    assert!(exports
        .iter()
        .any(|export| export.format == MoodboardExportFormat::Png));
    assert!(exports
        .iter()
        .any(|export| export.format == MoodboardExportFormat::Pdf));

    let mut gov_manifest_update_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard export .GOV manifest drift probe");
    let mut gov_manifest = png_export.manifest_json.clone();
    gov_manifest["output"]["path"] = serde_json::json!("D:\\Projects\\.GOV\\bad-output.png");
    let gov_manifest_update = sqlx::query(
        r#"UPDATE atelier_moodboard_export_request
           SET manifest_json = $1
           WHERE export_id = $2"#,
    )
    .bind(&gov_manifest)
    .bind(png_export.export_id)
    .execute(&mut *gov_manifest_update_tx)
    .await;
    let _ = gov_manifest_update_tx.rollback().await;
    assert!(
        gov_manifest_update.is_err(),
        "database must reject direct SQL moodboard export manifests that reference .GOV outputs"
    );

    let mut operation_hash_drift_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard operation payload hash drift probe");
    let operation_hash_drift = sqlx::query(
        r#"UPDATE atelier_moodboard_operation_receipt
           SET operation_payload_sha256 =
               '0000000000000000000000000000000000000000000000000000000000000000'
           WHERE operation_id = $1"#,
    )
    .bind(operation.operation_id)
    .execute(&mut *operation_hash_drift_tx)
    .await;
    let _ = operation_hash_drift_tx.rollback().await;
    assert!(
        operation_hash_drift.is_err(),
        "database must reject direct SQL operation receipt hash drift"
    );

    let mut operation_actor_drift_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard operation actor drift probe");
    let mut actor_drift_receipt = operation.receipt_json.clone();
    actor_drift_receipt["actor"] = serde_json::json!("drifted-actor");
    let operation_actor_drift = sqlx::query(
        r#"UPDATE atelier_moodboard_operation_receipt
           SET receipt_json = $1
           WHERE operation_id = $2"#,
    )
    .bind(&actor_drift_receipt)
    .bind(operation.operation_id)
    .execute(&mut *operation_actor_drift_tx)
    .await;
    let _ = operation_actor_drift_tx.rollback().await;
    assert!(
        operation_actor_drift.is_err(),
        "database must reject direct SQL operation receipt actor drift"
    );

    let mut operation_actor_missing_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard operation missing actor probe");
    let operation_actor_missing = sqlx::query(
        r#"UPDATE atelier_moodboard_operation_receipt
           SET receipt_json = receipt_json - 'actor'
           WHERE operation_id = $1"#,
    )
    .bind(operation.operation_id)
    .execute(&mut *operation_actor_missing_tx)
    .await;
    let _ = operation_actor_missing_tx.rollback().await;
    assert!(
        operation_actor_missing.is_err(),
        "database must reject direct SQL operation receipts missing actor"
    );

    let mut operation_id_drift_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard operation id drift probe");
    let operation_id_drift = sqlx::query(
        r#"UPDATE atelier_moodboard_operation_receipt
           SET operation_id = $1
           WHERE operation_id = $2"#,
    )
    .bind(Uuid::new_v4())
    .bind(operation.operation_id)
    .execute(&mut *operation_id_drift_tx)
    .await;
    let _ = operation_id_drift_tx.rollback().await;
    assert!(
        operation_id_drift.is_err(),
        "database must reject direct SQL operation_id drift away from receipt_json"
    );

    let mut operation_id_missing_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard operation missing operation_id probe");
    let operation_id_missing = sqlx::query(
        r#"UPDATE atelier_moodboard_operation_receipt
           SET receipt_json = receipt_json - 'operation_id'
           WHERE operation_id = $1"#,
    )
    .bind(operation.operation_id)
    .execute(&mut *operation_id_missing_tx)
    .await;
    let _ = operation_id_missing_tx.rollback().await;
    assert!(
        operation_id_missing.is_err(),
        "database must reject direct SQL operation receipts missing operation_id"
    );

    let mut export_requester_drift_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard export requested_by drift probe");
    let mut requested_by_drift_receipt = png_export.receipt_json.clone();
    requested_by_drift_receipt["requested_by"] = serde_json::json!("drifted-requester");
    let export_requester_drift = sqlx::query(
        r#"UPDATE atelier_moodboard_export_request
           SET receipt_json = $1
           WHERE export_id = $2"#,
    )
    .bind(&requested_by_drift_receipt)
    .bind(png_export.export_id)
    .execute(&mut *export_requester_drift_tx)
    .await;
    let _ = export_requester_drift_tx.rollback().await;
    assert!(
        export_requester_drift.is_err(),
        "database must reject direct SQL export receipt requested_by drift"
    );

    let mut export_requester_missing_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard export missing requested_by probe");
    let export_requester_missing = sqlx::query(
        r#"UPDATE atelier_moodboard_export_request
           SET receipt_json = receipt_json - 'requested_by'
           WHERE export_id = $1"#,
    )
    .bind(png_export.export_id)
    .execute(&mut *export_requester_missing_tx)
    .await;
    let _ = export_requester_missing_tx.rollback().await;
    assert!(
        export_requester_missing.is_err(),
        "database must reject direct SQL export receipts missing requested_by"
    );

    let mut export_output_missing_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard export missing output_artifact probe");
    let export_output_missing = sqlx::query(
        r#"UPDATE atelier_moodboard_export_request
           SET receipt_json = receipt_json - 'output_artifact'
           WHERE export_id = $1"#,
    )
    .bind(png_export.export_id)
    .execute(&mut *export_output_missing_tx)
    .await;
    let _ = export_output_missing_tx.rollback().await;
    assert!(
        export_output_missing.is_err(),
        "database must reject direct SQL export receipts missing output_artifact"
    );

    let mut export_id_drift_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard export id drift probe");
    let export_id_drift = sqlx::query(
        r#"UPDATE atelier_moodboard_export_request
           SET export_id = $1
           WHERE export_id = $2"#,
    )
    .bind(Uuid::new_v4())
    .bind(png_export.export_id)
    .execute(&mut *export_id_drift_tx)
    .await;
    let _ = export_id_drift_tx.rollback().await;
    assert!(
        export_id_drift.is_err(),
        "database must reject direct SQL export_id drift away from manifest/receipt JSON"
    );

    let mut export_manifest_id_missing_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard export manifest missing export_id probe");
    let export_manifest_id_missing = sqlx::query(
        r#"UPDATE atelier_moodboard_export_request
           SET manifest_json = manifest_json - 'export_id'
           WHERE export_id = $1"#,
    )
    .bind(png_export.export_id)
    .execute(&mut *export_manifest_id_missing_tx)
    .await;
    let _ = export_manifest_id_missing_tx.rollback().await;
    assert!(
        export_manifest_id_missing.is_err(),
        "database must reject direct SQL export manifests missing export_id"
    );

    let mut export_manifest_status_missing_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard export manifest missing status probe");
    let export_manifest_status_missing = sqlx::query(
        r#"UPDATE atelier_moodboard_export_request
           SET manifest_json = manifest_json - 'status'
           WHERE export_id = $1"#,
    )
    .bind(png_export.export_id)
    .execute(&mut *export_manifest_status_missing_tx)
    .await;
    let _ = export_manifest_status_missing_tx.rollback().await;
    assert!(
        export_manifest_status_missing.is_err(),
        "database must reject direct SQL export manifests missing status"
    );

    let mut export_manifest_output_missing_tx = reconnected
        .pool()
        .begin()
        .await
        .expect("begin moodboard export manifest missing output artifact probe");
    let export_manifest_output_missing = sqlx::query(
        r#"UPDATE atelier_moodboard_export_request
           SET manifest_json = manifest_json #- '{output,artifact}'
           WHERE export_id = $1"#,
    )
    .bind(png_export.export_id)
    .execute(&mut *export_manifest_output_missing_tx)
    .await;
    let _ = export_manifest_output_missing_tx.rollback().await;
    assert!(
        export_manifest_output_missing.is_err(),
        "database must reject direct SQL export manifests missing output artifact marker"
    );

    assert_uuid_update_rejected(
        &reconnected,
        r#"UPDATE atelier_moodboard_operation_receipt
           SET receipt_json = receipt_json - 'schema'
           WHERE operation_id = $1"#,
        operation.operation_id,
        "database must reject direct SQL operation receipts missing schema",
    )
    .await;
    assert_uuid_update_rejected(
        &reconnected,
        r#"UPDATE atelier_moodboard_operation_receipt
           SET receipt_json = receipt_json - 'status'
           WHERE operation_id = $1"#,
        operation.operation_id,
        "database must reject direct SQL operation receipts missing status",
    )
    .await;
    assert_uuid_update_rejected(
        &reconnected,
        r#"UPDATE atelier_moodboard_operation_receipt
           SET receipt_json = receipt_json - 'source_schema_id'
           WHERE operation_id = $1"#,
        operation.operation_id,
        "database must reject direct SQL operation receipts missing source_schema_id",
    )
    .await;
    assert_uuid_update_rejected(
        &reconnected,
        r#"UPDATE atelier_moodboard_operation_receipt
           SET receipt_json = receipt_json - 'source_content_sha256'
           WHERE operation_id = $1"#,
        operation.operation_id,
        "database must reject direct SQL operation receipts missing source_content_sha256",
    )
    .await;
    assert_uuid_update_rejected(
        &reconnected,
        r#"UPDATE atelier_moodboard_export_request
           SET manifest_json = manifest_json - 'schema'
           WHERE export_id = $1"#,
        png_export.export_id,
        "database must reject direct SQL export manifests missing schema",
    )
    .await;
    assert_uuid_update_rejected(
        &reconnected,
        r#"UPDATE atelier_moodboard_export_request
           SET manifest_json = manifest_json - 'source_schema_id'
           WHERE export_id = $1"#,
        png_export.export_id,
        "database must reject direct SQL export manifests missing source_schema_id",
    )
    .await;
    assert_uuid_update_rejected(
        &reconnected,
        r#"UPDATE atelier_moodboard_export_request
           SET manifest_json = manifest_json - 'source_content_sha256'
           WHERE export_id = $1"#,
        png_export.export_id,
        "database must reject direct SQL export manifests missing source_content_sha256",
    )
    .await;
    assert_uuid_update_rejected(
        &reconnected,
        r#"UPDATE atelier_moodboard_export_request
           SET manifest_json = jsonb_set(manifest_json, '{counts}', '{}'::jsonb)
           WHERE export_id = $1"#,
        png_export.export_id,
        "database must reject direct SQL export manifest counts drift",
    )
    .await;
    assert_uuid_update_rejected(
        &reconnected,
        r#"UPDATE atelier_moodboard_export_request
           SET receipt_json = receipt_json - 'schema'
           WHERE export_id = $1"#,
        png_export.export_id,
        "database must reject direct SQL export receipts missing schema",
    )
    .await;
    assert_uuid_update_rejected(
        &reconnected,
        r#"UPDATE atelier_moodboard_export_request
           SET receipt_json = receipt_json - 'source_content_sha256'
           WHERE export_id = $1"#,
        png_export.export_id,
        "database must reject direct SQL export receipts missing source_content_sha256",
    )
    .await;

    let operation_events_after = reconnected
        .count_events_for_aggregate(
            moodboard_event_family::MOODBOARD_OPERATION_RECORDED,
            "atelier_moodboard",
            &snapshot.snapshot_id.to_string(),
        )
        .await
        .expect("count moodboard operation events after");
    let export_events_after = reconnected
        .count_events_for_aggregate(
            moodboard_event_family::MOODBOARD_EXPORT_REQUESTED,
            "atelier_moodboard",
            &snapshot.snapshot_id.to_string(),
        )
        .await
        .expect("count moodboard export events after");
    assert_eq!(operation_events_after, operation_events_before + 1);
    assert_eq!(
        export_events_after,
        export_events_before + 2,
        "one event per newly planned export format; idempotent repeat emits none"
    );
    assert!(
        event_family::ALL.contains(&moodboard_event_family::MOODBOARD_OPERATION_RECORDED),
        "parent event registry must expose moodboard operation receipts"
    );
    assert!(
        event_family::ALL.contains(&moodboard_event_family::MOODBOARD_EXPORT_REQUESTED),
        "parent event registry must expose moodboard export requests"
    );

    let operation_payload: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_moodboard'
             AND aggregate_id = $2
           ORDER BY created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(moodboard_event_family::MOODBOARD_OPERATION_RECORDED)
    .bind(snapshot.snapshot_id.to_string())
    .fetch_one(reconnected.pool())
    .await
    .expect("read moodboard operation event payload");
    let operation_payload_text = operation_payload.to_string();
    assert!(operation_payload.get("operation_payload_sha256").is_some());
    assert!(!operation_payload_text.contains(&snapshot.raw_json_text));
    assert!(!operation_payload_text.contains(&layer_id.to_string()));
}

#[tokio::test]
async fn atelier_bracket_links_and_backlinks_rebuild_without_touching_source_text() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_bracket_links_and_backlinks_rebuild_without_touching_source_text: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let source_character = store
        .create_character(&NewCharacter {
            public_id: format!("char-link-source-{}", Uuid::new_v4()),
            display_name: "Link Source".to_string(),
        })
        .await
        .expect("create source character");
    let target_character = store
        .create_character(&NewCharacter {
            public_id: format!("char-link-target-{}", Uuid::new_v4()),
            display_name: "Link Target".to_string(),
        })
        .await
        .expect("create target character");
    let _uuid_shaped_public_id_character = store
        .create_character(&NewCharacter {
            public_id: target_character.internal_id.to_string(),
            display_name: "Ambiguous Public Id".to_string(),
        })
        .await
        .expect("create character with UUID-shaped public id for precedence proof");
    let story = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: source_character.internal_id,
            doc_type: CharacterDocumentType::Story,
            title: "Linked Story".to_string(),
            body_raw_text: "story source".to_string(),
            tags: vec![],
            author: "mt-041-author".to_string(),
        })
        .await
        .expect("create story document");
    let unlinked_story = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: source_character.internal_id,
            doc_type: CharacterDocumentType::Story,
            title: "Unlinked Story".to_string(),
            body_raw_text: "valid target absent from source".to_string(),
            tags: vec![],
            author: "mt-041-author".to_string(),
        })
        .await
        .expect("create unlinked story document");
    let moodboard = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: source_character.internal_id,
            doc_type: CharacterDocumentType::Moodboard,
            title: "Linked Moodboard".to_string(),
            body_raw_text: "moodboard source".to_string(),
            tags: vec![],
            author: "mt-041-author".to_string(),
        })
        .await
        .expect("create moodboard document");
    let target_note = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: source_character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Linked Note".to_string(),
            body_raw_text: "note target".to_string(),
            tags: vec![],
            author: "mt-041-author".to_string(),
        })
        .await
        .expect("create target note document");
    let asset_id = fresh_asset(&store).await;
    let target_note_marker_id = target_note.document_id.to_string().to_ascii_uppercase();
    let story_marker_id = story.document_id.to_string().to_ascii_uppercase();
    let moodboard_marker_id = moodboard.document_id.to_string().to_ascii_uppercase();
    let asset_marker_id = asset_id.to_string().to_ascii_uppercase();
    let character_marker_id = target_character
        .internal_id
        .to_string()
        .to_ascii_uppercase();
    let source_body = format!(
        "Keep exact text [[character:{}|Target Char]] -> [[document:{}|Note]] -> [[story:{}]] -> [[moodboard:{}|Mood]] -> [[image:{}]].",
        character_marker_id,
        target_note_marker_id,
        story_marker_id,
        moodboard_marker_id,
        asset_marker_id
    );
    let note = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: source_character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Bracket Link Source".to_string(),
            body_raw_text: source_body.clone(),
            tags: vec!["links".to_string()],
            author: "mt-041-author".to_string(),
        })
        .await
        .expect("create source note");

    let rebuilt = store
        .rebuild_bracket_links_for_character_document(note.document_id)
        .await
        .expect("rebuild bracket link projections");
    assert_eq!(rebuilt.len(), 5);
    assert_eq!(rebuilt[0].seq, 1);
    assert_eq!(rebuilt[0].target_kind, BracketLinkTargetKind::Character);
    assert_eq!(
        rebuilt[0].target_id,
        target_character.internal_id.to_string()
    );
    assert_eq!(rebuilt[0].target_label.as_deref(), Some("Target Char"));
    assert_eq!(rebuilt[1].target_kind, BracketLinkTargetKind::Document);
    assert_eq!(rebuilt[1].target_id, target_note.document_id.to_string());
    assert_eq!(rebuilt[2].target_kind, BracketLinkTargetKind::Story);
    assert_eq!(rebuilt[2].target_id, story.document_id.to_string());
    assert_eq!(rebuilt[3].target_kind, BracketLinkTargetKind::Moodboard);
    assert_eq!(rebuilt[3].target_id, moodboard.document_id.to_string());
    assert_eq!(rebuilt[4].target_kind, BracketLinkTargetKind::Image);
    assert_eq!(rebuilt[4].target_id, asset_id.to_string());

    let persisted_body = store
        .latest_character_document_version(note.document_id)
        .await
        .expect("reload latest document version")
        .expect("latest document version exists");
    assert_eq!(
        persisted_body.body_raw_text, source_body,
        "bracket projection rebuild must not alter source text"
    );

    let reconnected = connected_store(&url).await;
    let outbound = reconnected
        .list_bracket_links_from_document(note.document_id)
        .await
        .expect("list outbound bracket links");
    assert_eq!(outbound, rebuilt);
    let story_backlinks = reconnected
        .list_backlinks_to(BracketLinkTargetKind::Story, &story.document_id.to_string())
        .await
        .expect("list story backlinks");
    assert_eq!(story_backlinks.len(), 1);
    assert_eq!(story_backlinks[0].source_document_id, note.document_id);
    let story_backlinks_by_uppercase_id = reconnected
        .list_backlinks_to(BracketLinkTargetKind::Story, &story_marker_id)
        .await
        .expect("list story backlinks by uppercase UUID");
    assert_eq!(story_backlinks_by_uppercase_id, story_backlinks);
    let character_backlinks_by_public_id = reconnected
        .list_backlinks_to(
            BracketLinkTargetKind::Character,
            &target_character.public_id,
        )
        .await
        .expect("list character backlinks by public id");
    let character_backlinks_by_internal_id = reconnected
        .list_backlinks_to(
            BracketLinkTargetKind::Character,
            &target_character.internal_id.to_string(),
        )
        .await
        .expect("list character backlinks by internal id");
    let character_backlinks_by_uppercase_internal_id = reconnected
        .list_backlinks_to(BracketLinkTargetKind::Character, &character_marker_id)
        .await
        .expect("list character backlinks by uppercase internal id");
    assert_eq!(character_backlinks_by_public_id.len(), 1);
    assert_eq!(character_backlinks_by_internal_id.len(), 1);
    assert_eq!(character_backlinks_by_uppercase_internal_id.len(), 1);
    assert_eq!(
        character_backlinks_by_public_id[0],
        character_backlinks_by_internal_id[0]
    );
    assert_eq!(
        character_backlinks_by_uppercase_internal_id[0],
        character_backlinks_by_internal_id[0]
    );

    let rebuilt_again = reconnected
        .rebuild_bracket_links_for_character_document(note.document_id)
        .await
        .expect("rebuild projections idempotently");
    assert_eq!(rebuilt_again.len(), 5);
    let outbound_after = reconnected
        .list_bracket_links_from_document(note.document_id)
        .await
        .expect("list outbound bracket links after idempotent rebuild");
    assert_eq!(outbound_after, rebuilt_again);

    let rebuild_events = reconnected
        .count_events_for_aggregate(
            links_event_family::BRACKET_LINKS_REBUILT,
            "atelier_character_document",
            &note.document_id.to_string(),
        )
        .await
        .expect("count bracket link rebuild events");
    assert_eq!(rebuild_events, 2);
    assert!(
        event_family::ALL.contains(&links_event_family::BRACKET_LINKS_REBUILT),
        "parent event registry must expose bracket-link rebuilds"
    );
    let event_payload_rows = sqlx::query(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_character_document'
             AND aggregate_id = $2
           ORDER BY created_at_utc ASC"#,
    )
    .bind(links_event_family::BRACKET_LINKS_REBUILT)
    .bind(note.document_id.to_string())
    .fetch_all(reconnected.pool())
    .await
    .expect("read bracket link event payloads");
    assert_eq!(event_payload_rows.len(), 2);
    for row in event_payload_rows {
        let payload: serde_json::Value = row.get("payload");
        assert_eq!(payload["source_doc_type"], serde_json::json!("note"));
        assert_eq!(payload["link_count"], serde_json::json!(5));
        assert!(payload.get("source_document_id_ref").is_some());
        assert!(payload.get("source_version_id_ref").is_some());
        let payload_text = payload.to_string();
        let forbidden_payload_values = [
            source_body.clone(),
            target_character.public_id.clone(),
            target_character.internal_id.to_string(),
            target_note.document_id.to_string(),
            story.document_id.to_string(),
            moodboard.document_id.to_string(),
            asset_id.to_string(),
        ];
        for raw_value in forbidden_payload_values {
            assert!(
                !payload_text.contains(&raw_value),
                "bracket link event payload must not leak raw value {raw_value:?}: {payload}"
            );
        }
    }

    let padded_target = sqlx::query(
        r#"INSERT INTO atelier_bracket_link_projection
             (source_document_id, source_version_id, source_doc_type, seq,
              raw_marker, target_kind, target_id, target_label)
           VALUES ($1, $2, 'note', 99,
                   '[[story:padded]]', 'story', $3, 'bad')"#,
    )
    .bind(note.document_id)
    .bind(note.version_id)
    .bind(format!("{}\n", story.document_id))
    .execute(store.pool())
    .await;
    assert!(
        padded_target.is_err(),
        "database must reject padded target ids that would break backlink lookup"
    );

    let missing_asset_id = Uuid::new_v4();
    let missing_target = sqlx::query(
        r#"INSERT INTO atelier_bracket_link_projection
             (source_document_id, source_version_id, source_doc_type, seq,
              raw_marker, target_kind, target_id, target_label)
           VALUES ($1, $2, 'note', 100,
                   $3, 'image', $4, NULL)"#,
    )
    .bind(note.document_id)
    .bind(note.version_id)
    .bind(format!("[[image:{missing_asset_id}]]"))
    .bind(missing_asset_id.to_string())
    .execute(store.pool())
    .await;
    assert!(
        missing_target.is_err(),
        "database must reject projection rows with nonexistent targets"
    );

    let absent_from_source = sqlx::query(
        r#"INSERT INTO atelier_bracket_link_projection
             (source_document_id, source_version_id, source_doc_type, seq,
              raw_marker, target_kind, target_id, target_label)
           VALUES ($1, $2, 'note', 102,
                   $3, 'story', $4, NULL)"#,
    )
    .bind(note.document_id)
    .bind(note.version_id)
    .bind(format!("[[story:{}]]", unlinked_story.document_id))
    .bind(unlinked_story.document_id.to_string())
    .execute(store.pool())
    .await;
    assert!(
        absent_from_source.is_err(),
        "database must reject valid target rows not backed by the source text"
    );

    let mismatched_marker = sqlx::query(
        r#"INSERT INTO atelier_bracket_link_projection
             (source_document_id, source_version_id, source_doc_type, seq,
              raw_marker, target_kind, target_id, target_label)
           VALUES ($1, $2, 'note', 101,
                   $3, 'image', $4, NULL)"#,
    )
    .bind(note.document_id)
    .bind(note.version_id)
    .bind(format!("[[story:{}]]", story.document_id))
    .bind(asset_id.to_string())
    .execute(store.pool())
    .await;
    assert!(
        mismatched_marker.is_err(),
        "database must reject raw marker and target_kind mismatch"
    );

    let uppercase_target_update = sqlx::query(
        r#"UPDATE atelier_bracket_link_projection
           SET target_id = $1
           WHERE link_id = $2"#,
    )
    .bind(story_marker_id)
    .bind(outbound_after[2].link_id)
    .execute(store.pool())
    .await;
    assert!(
        uppercase_target_update.is_err(),
        "database must reject noncanonical UUID target ids even when raw_marker uses uppercase text"
    );

    let mismatched_label_update = sqlx::query(
        r#"UPDATE atelier_bracket_link_projection
           SET target_label = 'Wrong Label'
           WHERE link_id = $1"#,
    )
    .bind(outbound_after[0].link_id)
    .execute(store.pool())
    .await;
    assert!(
        mismatched_label_update.is_err(),
        "database must reject labels that are not rebuildable from the raw source marker"
    );

    let tab_label_raw_marker = format!("[[story:{}|\tTabbed Label\t]]", story.document_id);
    let tab_label_note = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: source_character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Bracket Link Tab Label Source".to_string(),
            body_raw_text: format!("Normalize label {tab_label_raw_marker}."),
            tags: vec!["links".to_string()],
            author: "mt-041-author".to_string(),
        })
        .await
        .expect("create tab-label source note");
    let tab_label_current_rebuildable: bool = sqlx::query_scalar(
        r#"SELECT atelier_bracket_link_projection_is_current_rebuildable(
               $1, $2, 'note', 1, $3, 'story', $4, $5
           )"#,
    )
    .bind(tab_label_note.document_id)
    .bind(tab_label_note.version_id)
    .bind(&tab_label_raw_marker)
    .bind(story.document_id.to_string())
    .bind("Tabbed Label")
    .fetch_one(store.pool())
    .await
    .expect("query tab-label current rebuildability");
    assert!(
        tab_label_current_rebuildable,
        "database current rebuildability must normalize label whitespace like Rust"
    );
    let tab_label_rebuilt = store
        .rebuild_bracket_links_for_character_document(tab_label_note.document_id)
        .await
        .expect("rebuild tab-label projection");
    assert_eq!(
        tab_label_rebuilt[0].target_label.as_deref(),
        Some("Tabbed Label")
    );

    let blank_label_raw_marker = format!("[[story:{}|\t \t]]", story.document_id);
    let blank_label_note = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: source_character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Bracket Link Blank Label Source".to_string(),
            body_raw_text: format!("Normalize blank label {blank_label_raw_marker}."),
            tags: vec!["links".to_string()],
            author: "mt-041-author".to_string(),
        })
        .await
        .expect("create blank-label source note");
    let blank_label_current_rebuildable: bool = sqlx::query_scalar(
        r#"SELECT atelier_bracket_link_projection_is_current_rebuildable(
               $1, $2, 'note', 1, $3, 'story', $4, $5
           )"#,
    )
    .bind(blank_label_note.document_id)
    .bind(blank_label_note.version_id)
    .bind(&blank_label_raw_marker)
    .bind(story.document_id.to_string())
    .bind(Option::<String>::None)
    .fetch_one(store.pool())
    .await
    .expect("query blank-label current rebuildability");
    assert!(
        blank_label_current_rebuildable,
        "database current rebuildability must normalize whitespace-only labels to NULL like Rust"
    );
    let blank_label_rebuilt = store
        .rebuild_bracket_links_for_character_document(blank_label_note.document_id)
        .await
        .expect("rebuild blank-label projection");
    assert_eq!(blank_label_rebuilt[0].target_label, None);

    let bracket_label_raw_marker = format!("[[story:{}|Act [draft]]", story.document_id);
    let bracket_label_note = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: source_character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Bracket Link Bracket Label Source".to_string(),
            body_raw_text: format!("Preserve bracket label {bracket_label_raw_marker}."),
            tags: vec!["links".to_string()],
            author: "mt-041-author".to_string(),
        })
        .await
        .expect("create bracket-label source note");
    let bracket_label_current_rebuildable: bool = sqlx::query_scalar(
        r#"SELECT atelier_bracket_link_projection_is_current_rebuildable(
               $1, $2, 'note', 1, $3, 'story', $4, $5
           )"#,
    )
    .bind(bracket_label_note.document_id)
    .bind(bracket_label_note.version_id)
    .bind(&bracket_label_raw_marker)
    .bind(story.document_id.to_string())
    .bind("Act [draft")
    .fetch_one(store.pool())
    .await
    .expect("query bracket-label current rebuildability");
    assert!(
        bracket_label_current_rebuildable,
        "database current rebuildability must allow single brackets that the Rust parser accepts"
    );
    let bracket_label_rebuilt = store
        .rebuild_bracket_links_for_character_document(bracket_label_note.document_id)
        .await
        .expect("rebuild bracket-label projection");
    assert_eq!(
        bracket_label_rebuilt[0].target_label.as_deref(),
        Some("Act [draft")
    );

    let malformed_current_first_marker = format!("[[story:{}]]", story.document_id);
    let malformed_version = store
        .append_character_document_version(
            note.document_id,
            &AppendCharacterDocumentVersion {
                title: "Bracket Link Source".to_string(),
                body_raw_text: format!(
                    "Broken source {malformed_current_first_marker} then [[story:{}",
                    story.document_id
                ),
                tags: vec!["links".to_string()],
                author: "mt-041-author".to_string(),
            },
        )
        .await
        .expect("append malformed source text version");
    let malformed_rebuild = store
        .rebuild_bracket_links_for_character_document(note.document_id)
        .await;
    assert!(
        malformed_rebuild.is_err(),
        "malformed bracket-like source text must reject before deleting prior projections"
    );
    let outbound_after_malformed = store
        .list_bracket_links_from_document(note.document_id)
        .await
        .expect("list outbound links after malformed rebuild attempt");
    assert_eq!(outbound_after_malformed, outbound_after);
    let malformed_current_projection_update = sqlx::query(
        r#"UPDATE atelier_bracket_link_projection
           SET source_version_id = $1,
               raw_marker = $2,
               target_kind = 'story',
               target_id = $3,
               target_label = NULL
           WHERE link_id = $4"#,
    )
    .bind(malformed_version.version_id)
    .bind(&malformed_current_first_marker)
    .bind(story.document_id.to_string())
    .bind(outbound_after[0].link_id)
    .execute(store.pool())
    .await;
    assert!(
        malformed_current_projection_update.is_err(),
        "database must reject direct SQL rows from a current source that has a later malformed marker"
    );
    let stale_projection_update = sqlx::query(
        r#"UPDATE atelier_bracket_link_projection
           SET source_version_id = source_version_id
           WHERE link_id = $1"#,
    )
    .bind(outbound_after[0].link_id)
    .execute(store.pool())
    .await;
    assert!(
        stale_projection_update.is_err(),
        "database must reject direct SQL writes to historical projections after current version advances"
    );
    let rebuild_events_after_malformed = store
        .count_events_for_aggregate(
            links_event_family::BRACKET_LINKS_REBUILT,
            "atelier_character_document",
            &note.document_id.to_string(),
        )
        .await
        .expect("count bracket link rebuild events after malformed source");
    assert_eq!(rebuild_events_after_malformed, 2);
}

#[tokio::test]
async fn atelier_character_relationships_crud_endpoint_validation_and_graph_projection() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_character_relationships_crud_endpoint_validation_and_graph_projection: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let source = store
        .create_character(&NewCharacter {
            public_id: format!("char-rel-source-{}", Uuid::new_v4()),
            display_name: "Relationship Source".to_string(),
        })
        .await
        .expect("create relationship source character");
    let target = store
        .create_character(&NewCharacter {
            public_id: format!("char-rel-target-{}", Uuid::new_v4()),
            display_name: "Relationship Target".to_string(),
        })
        .await
        .expect("create relationship target character");
    let other = store
        .create_character(&NewCharacter {
            public_id: format!("char-rel-other-{}", Uuid::new_v4()),
            display_name: "Unrelated Character".to_string(),
        })
        .await
        .expect("create unrelated character");

    let relationship = store
        .create_character_relationship(&NewCharacterRelationship {
            source_character_id: source.internal_id,
            target_character_id: target.internal_id,
            relationship_kind: " mentor ".to_string(),
            label: Some(" Mentor ".to_string()),
            notes: Some(" keeps canon aligned ".to_string()),
        })
        .await
        .expect("create character relationship");
    assert_eq!(relationship.source_character_id, source.internal_id);
    assert_eq!(relationship.target_character_id, target.internal_id);
    assert_eq!(relationship.relationship_kind, "mentor");
    assert_eq!(relationship.label.as_deref(), Some("Mentor"));
    assert_eq!(relationship.notes, "keeps canon aligned");

    let duplicate = store
        .create_character_relationship(&NewCharacterRelationship {
            source_character_id: source.internal_id,
            target_character_id: target.internal_id,
            relationship_kind: "mentor".to_string(),
            label: Some("Second label".to_string()),
            notes: None,
        })
        .await;
    assert!(
        duplicate.is_err(),
        "create_character_relationship must not silently upsert duplicate edges"
    );

    let fetched = store
        .get_character_relationship(relationship.relationship_id)
        .await
        .expect("fetch character relationship");
    assert_eq!(fetched, relationship);
    let source_relationships = store
        .list_character_relationships(source.internal_id)
        .await
        .expect("list source relationships");
    assert_eq!(source_relationships.len(), 1);
    assert_eq!(
        source_relationships[0].relationship_id,
        relationship.relationship_id
    );
    let other_relationships = store
        .list_character_relationships(other.internal_id)
        .await
        .expect("list unrelated relationships");
    assert!(
        other_relationships.is_empty(),
        "unrelated characters must not inherit relationship edges"
    );

    let self_edge = store
        .create_character_relationship(&NewCharacterRelationship {
            source_character_id: source.internal_id,
            target_character_id: source.internal_id,
            relationship_kind: "self".to_string(),
            label: None,
            notes: None,
        })
        .await;
    assert!(
        self_edge.is_err(),
        "relationship endpoints must be distinct before storage"
    );
    let missing_endpoint = store
        .create_character_relationship(&NewCharacterRelationship {
            source_character_id: source.internal_id,
            target_character_id: Uuid::new_v4(),
            relationship_kind: "missing".to_string(),
            label: None,
            notes: None,
        })
        .await;
    assert!(
        missing_endpoint.is_err(),
        "relationship endpoints must reference existing characters"
    );

    let source_graph = store
        .character_relationship_graph(source.internal_id)
        .await
        .expect("project relationship graph from source");
    assert_eq!(source_graph.anchor_character_id, source.internal_id);
    assert_eq!(source_graph.edges.len(), 1);
    assert_eq!(
        source_graph.edges[0].relationship_id,
        relationship.relationship_id
    );
    assert_eq!(
        source_graph.edges[0].source_character_id,
        source.internal_id
    );
    assert_eq!(
        source_graph.edges[0].target_character_id,
        target.internal_id
    );
    assert_eq!(source_graph.edges[0].relationship_kind, "mentor");
    assert_eq!(source_graph.edges[0].label.as_deref(), Some("Mentor"));
    assert_eq!(source_graph.nodes.len(), 2);
    assert!(source_graph
        .nodes
        .iter()
        .any(|node| node.character_internal_id == source.internal_id
            && node.public_id == source.public_id
            && node.display_name == source.display_name));
    assert!(source_graph
        .nodes
        .iter()
        .any(|node| node.character_internal_id == target.internal_id
            && node.public_id == target.public_id
            && node.display_name == target.display_name));
    let target_graph = store
        .character_relationship_graph(target.internal_id)
        .await
        .expect("project relationship graph from inbound target");
    assert_eq!(target_graph.edges.len(), 1);
    assert_eq!(
        target_graph.edges[0].relationship_id, relationship.relationship_id,
        "relationship graph must include inbound edges for the anchor"
    );

    let updated = store
        .update_character_relationship(
            relationship.relationship_id,
            &UpdateCharacterRelationship {
                relationship_kind: " ally ".to_string(),
                label: Some(" Trusted Ally ".to_string()),
                notes: Some(" updated notes ".to_string()),
            },
        )
        .await
        .expect("update relationship metadata");
    assert_eq!(updated.relationship_id, relationship.relationship_id);
    assert_eq!(updated.relationship_kind, "ally");
    assert_eq!(updated.label.as_deref(), Some("Trusted Ally"));
    assert_eq!(updated.notes, "updated notes");
    let updated_graph = store
        .character_relationship_graph(source.internal_id)
        .await
        .expect("project graph after relationship update");
    assert_eq!(updated_graph.edges.len(), 1);
    assert_eq!(updated_graph.edges[0].relationship_kind, "ally");
    assert_eq!(
        updated_graph.edges[0].label.as_deref(),
        Some("Trusted Ally")
    );

    let direct_self_insert = sqlx::query(
        r#"INSERT INTO atelier_character_relationship
             (source_character_id, target_character_id, relationship_kind, label, notes)
           VALUES ($1, $1, 'invalid', NULL, '')"#,
    )
    .bind(source.internal_id)
    .execute(store.pool())
    .await;
    assert!(
        direct_self_insert.is_err(),
        "database guard must reject direct SQL self-relationships"
    );
    let direct_padded_kind_insert = sqlx::query(
        r#"INSERT INTO atelier_character_relationship
             (source_character_id, target_character_id, relationship_kind, label, notes)
           VALUES ($1, $2, ' padded ', NULL, '')"#,
    )
    .bind(source.internal_id)
    .bind(other.internal_id)
    .execute(store.pool())
    .await;
    assert!(
        direct_padded_kind_insert.is_err(),
        "database guard must reject padded relationship kinds"
    );

    let deleted = store
        .delete_character_relationship(relationship.relationship_id)
        .await
        .expect("delete relationship");
    assert_eq!(deleted.relationship_id, relationship.relationship_id);
    let deleted_lookup = store
        .get_character_relationship(relationship.relationship_id)
        .await;
    assert!(
        deleted_lookup.is_err(),
        "deleted relationships must not remain fetchable"
    );
    let graph_after_delete = store
        .character_relationship_graph(source.internal_id)
        .await
        .expect("project graph after relationship delete");
    assert!(graph_after_delete.edges.is_empty());
    assert_eq!(
        graph_after_delete.nodes.len(),
        1,
        "graph projection retains the anchor node after all edges are deleted"
    );
    assert_eq!(
        graph_after_delete.nodes[0].character_internal_id,
        source.internal_id
    );

    for family in relationships_event_family::ALL {
        assert!(
            event_family::ALL.contains(family),
            "parent event registry must expose {family}"
        );
    }
    assert_eq!(
        store
            .count_events_for_aggregate(
                relationships_event_family::CHARACTER_RELATIONSHIP_CREATED,
                "atelier_character_relationship",
                &relationship.relationship_id.to_string(),
            )
            .await
            .expect("count relationship created events"),
        1,
        "create emits exactly one relationship event"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                relationships_event_family::CHARACTER_RELATIONSHIP_UPDATED,
                "atelier_character_relationship",
                &relationship.relationship_id.to_string(),
            )
            .await
            .expect("count relationship updated events"),
        1,
        "update emits exactly one relationship event"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                relationships_event_family::CHARACTER_RELATIONSHIP_DELETED,
                "atelier_character_relationship",
                &relationship.relationship_id.to_string(),
            )
            .await
            .expect("count relationship deleted events"),
        1,
        "delete emits exactly one relationship event"
    );
    let relationship_event_payload_rows = sqlx::query(
        r#"SELECT payload
           FROM atelier_event
           WHERE aggregate_type = 'atelier_character_relationship'
             AND aggregate_id = $1
           ORDER BY created_at_utc ASC"#,
    )
    .bind(relationship.relationship_id.to_string())
    .fetch_all(store.pool())
    .await
    .expect("read relationship event payloads");
    assert_eq!(relationship_event_payload_rows.len(), 3);
    for row in relationship_event_payload_rows {
        let payload: serde_json::Value = row.get("payload");
        let payload_text = payload.to_string();
        for raw_value in [
            source.internal_id.to_string(),
            target.internal_id.to_string(),
            source.public_id.clone(),
            target.public_id.clone(),
            source.display_name.clone(),
            target.display_name.clone(),
            "Mentor".to_string(),
            "Trusted Ally".to_string(),
            "keeps canon aligned".to_string(),
            "updated notes".to_string(),
        ] {
            assert!(
                !payload_text.contains(&raw_value),
                "relationship event payload must not leak raw value {raw_value:?}: {payload}"
            );
        }
    }
}

#[tokio::test]
async fn atelier_character_scripts_preserve_refs_without_executable_authority() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_character_scripts_preserve_refs_without_executable_authority: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-script-{}", Uuid::new_v4()),
            display_name: "Script Subject".to_string(),
        })
        .await
        .expect("create character for script proof");
    let provenance_ref = format!("source://atelier/image-script/{}", Uuid::new_v4());
    let initial_usage_ref = format!("usage://atelier/contact-sheet/{}", Uuid::new_v4());
    let script_body = "lookbook sourcing script\npose: standing\nnegative: blurry";

    let script = store
        .create_character_script(&NewCharacterScript {
            character_internal_id: character.internal_id,
            name: "Lookbook image sourcing".to_string(),
            script_body_raw_text: script_body.to_string(),
            provenance_refs: vec![provenance_ref.clone()],
            usage_refs: vec![initial_usage_ref.clone()],
            created_by: "mt-040-author".to_string(),
        })
        .await
        .expect("create character script");
    assert_eq!(script.character_internal_id, character.internal_id);
    assert_eq!(script.script_body_raw_text, script_body);
    assert_eq!(script.provenance_refs, vec![provenance_ref.clone()]);
    assert_eq!(script.usage_refs, vec![initial_usage_ref.clone()]);
    assert_eq!(
        script.authority_mode,
        CharacterScriptAuthorityMode::DataOnly
    );
    assert!(
        !script.hidden_executable_authority,
        "script records must not become hidden executable authority"
    );

    let usage_ref = format!("usage://atelier/export/{}", Uuid::new_v4());
    let updated = store
        .record_character_script_usage(script.script_id, &usage_ref, "mt-040-usage")
        .await
        .expect("record script usage ref");
    assert_eq!(
        updated.usage_refs,
        vec![initial_usage_ref.clone(), usage_ref.clone()]
    );
    let duplicate = store
        .record_character_script_usage(script.script_id, &usage_ref, "mt-040-usage")
        .await
        .expect("duplicate usage ref is idempotent");
    assert_eq!(
        duplicate.usage_refs,
        vec![initial_usage_ref.clone(), usage_ref.clone()]
    );

    let reconnected = connected_store(&url).await;
    let persisted = reconnected
        .get_character_script(script.script_id)
        .await
        .expect("get character script after reconnect");
    assert_eq!(persisted.script_id, script.script_id);
    assert_eq!(persisted.script_body_raw_text, script_body);
    assert_eq!(persisted.provenance_refs, vec![provenance_ref]);
    assert_eq!(persisted.usage_refs, vec![initial_usage_ref, usage_ref]);
    let scripts = reconnected
        .list_character_scripts(character.internal_id)
        .await
        .expect("list character scripts");
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].script_id, script.script_id);

    let create_events = reconnected
        .count_events_for_aggregate(
            scripts_event_family::CHARACTER_SCRIPT_CREATED,
            "atelier_character_script",
            &script.script_id.to_string(),
        )
        .await
        .expect("count character script create events");
    let usage_events = reconnected
        .count_events_for_aggregate(
            scripts_event_family::CHARACTER_SCRIPT_USAGE_RECORDED,
            "atelier_character_script",
            &script.script_id.to_string(),
        )
        .await
        .expect("count character script usage events");
    assert_eq!(create_events, 1);
    assert_eq!(usage_events, 1);
    let event_payloads = sqlx::query(
        r#"SELECT ae.payload AS atelier_payload, kel.payload AS kernel_payload
           FROM atelier_event ae
           JOIN kernel_event_ledger kel
             ON kel.event_id = ae.kernel_event_id
            AND kel.event_sequence = ae.kernel_event_sequence
           WHERE ae.event_family = $1
             AND ae.aggregate_type = 'atelier_character_script'
             AND ae.aggregate_id = $2"#,
    )
    .bind(scripts_event_family::CHARACTER_SCRIPT_CREATED)
    .bind(script.script_id.to_string())
    .fetch_all(reconnected.pool())
    .await
    .expect("read character script event payloads");
    assert_eq!(event_payloads.len(), 1);
    let atelier_payload: serde_json::Value = event_payloads[0].get("atelier_payload");
    let kernel_payload: serde_json::Value = event_payloads[0].get("kernel_payload");
    for payload in [
        &atelier_payload,
        kernel_payload
            .get("atelier_payload")
            .expect("kernel mirror contains atelier payload"),
    ] {
        assert_eq!(payload["provenance_ref_count"], 1);
        assert_eq!(payload["usage_ref_count"], 1);
        assert_eq!(payload["authority_mode"], "data_only");
        assert_eq!(payload["hidden_executable_authority"], false);
        assert!(payload.get("script_name").is_none());
        assert!(payload.get("script_body_raw_text").is_none());
        assert!(payload.get("provenance_refs").is_none());
        assert!(payload.get("usage_refs").is_none());
        for raw_value in [
            "Lookbook image sourcing",
            script_body,
            "mt-040-author",
            "source://atelier/image-script/",
            "usage://atelier/contact-sheet/",
        ] {
            assert!(
                !payload.to_string().contains(raw_value),
                "script event payload must not leak raw value {raw_value:?}: {payload}"
            );
        }
    }
    assert!(
        event_family::ALL.contains(&scripts_event_family::CHARACTER_SCRIPT_CREATED),
        "parent event registry must expose character script creation"
    );
    assert!(
        event_family::ALL.contains(&scripts_event_family::CHARACTER_SCRIPT_USAGE_RECORDED),
        "parent event registry must expose character script usage"
    );

    let executable_authority = sqlx::query(
        r#"INSERT INTO atelier_character_script
             (character_internal_id, script_name, script_body_raw_text,
              provenance_refs_json, usage_refs_json, authority_mode,
              hidden_executable_authority, created_by)
           VALUES ($1, 'bad executable script', 'run this',
                   '[]'::jsonb, '[]'::jsonb, 'executable', TRUE, 'mt-040-test')"#,
    )
    .bind(character.internal_id)
    .execute(store.pool())
    .await;
    assert!(
        executable_authority.is_err(),
        "database must reject executable or hidden-authority script records"
    );

    let malformed_refs = sqlx::query(
        r#"INSERT INTO atelier_character_script
             (character_internal_id, script_name, script_body_raw_text,
              provenance_refs_json, usage_refs_json, authority_mode,
              hidden_executable_authority, created_by)
           VALUES ($1, 'bad ref shape', 'do not lose refs',
                   '["source://atelier/image-script/ok", 7]'::jsonb,
                   '[{"not":"a-ref"}]'::jsonb,
                   'data_only', FALSE, 'mt-040-test')"#,
    )
    .bind(character.internal_id)
    .execute(store.pool())
    .await;
    assert!(
        malformed_refs.is_err(),
        "database must reject non-string refs instead of letting readers silently drop them"
    );

    let padded_refs = sqlx::query(
        r#"INSERT INTO atelier_character_script
             (character_internal_id, script_name, script_body_raw_text,
              provenance_refs_json, usage_refs_json, authority_mode,
              hidden_executable_authority, created_by)
           VALUES ($1, 'bad padded ref', 'do not persist unreadable refs',
                   $2::jsonb, $3::jsonb,
                   'data_only', FALSE, 'mt-040-test')"#,
    )
    .bind(character.internal_id)
    .bind(serde_json::json!([
        "\tsource://atelier/image-script/padded"
    ]))
    .bind(serde_json::json!([
        "usage://atelier/contact-sheet/padded\n"
    ]))
    .execute(store.pool())
    .await;
    assert!(
        padded_refs.is_err(),
        "database must reject tab/newline-padded refs that the Rust reader would reject"
    );
}

#[tokio::test]
async fn atelier_contact_sheet_legacy_schema_is_repaired_on_ensure_schema() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_contact_sheet_legacy_schema_is_repaired_on_ensure_schema: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let asset = fresh_asset(&store).await;
    let sheet = store
        .create_contact_sheet(
            &format!("legacy-schema-sheet-{}", Uuid::new_v4()),
            "manual",
            None,
            &[asset],
            &["schema-repair".to_string()],
            None,
            None,
        )
        .await
        .expect("create contact sheet");

    let legacy_schema = ["c", "kc.contact_sheet@1"].concat();
    let mut legacy_manifest = sheet.manifest.clone();
    legacy_manifest["schema"] = serde_json::Value::String(legacy_schema.clone());
    sqlx::query("UPDATE atelier_contact_sheet SET manifest = $2 WHERE sheet_id = $1")
        .bind(sheet.sheet_id)
        .bind(&legacy_manifest)
        .execute(store.pool())
        .await
        .expect("seed legacy contact-sheet schema");

    let legacy = store
        .get_contact_sheet(sheet.sheet_id)
        .await
        .expect("read legacy contact sheet");
    assert_eq!(
        legacy.manifest.get("schema").and_then(|v| v.as_str()),
        Some(legacy_schema.as_str()),
        "test setup must prove a legacy persisted schema exists before repair"
    );

    store
        .ensure_schema()
        .await
        .expect("ensure_schema repairs legacy contact-sheet schema");
    let repaired = store
        .get_contact_sheet(sheet.sheet_id)
        .await
        .expect("read repaired contact sheet");
    assert_eq!(
        repaired.manifest.get("schema").and_then(|v| v.as_str()),
        Some("hsk.atelier.contact_sheet@1"),
        "legacy contact-sheet schemas are backfilled to the Handshake namespace"
    );
    let repaired_text = repaired.manifest.to_string().to_ascii_lowercase();
    assert!(
        !repaired_text.contains(&["c", "kc."].concat())
            && !repaired_text.contains(&["cast", "kit"].concat()),
        "repaired contact-sheet manifest must not retain legacy namespace text"
    );
    let legacy_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_contact_sheet WHERE manifest->>'schema' = $1",
    )
    .bind(&legacy_schema)
    .fetch_one(store.pool())
    .await
    .expect("count legacy contact-sheet schemas");
    assert_eq!(
        legacy_count, 0,
        "ensure_schema must leave no legacy contact-sheet manifest schemas in the live database"
    );
}

#[tokio::test]
async fn atelier_global_search_returns_snippets_and_jump_targets_without_sqlite_fts() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_global_search_returns_snippets_and_jump_targets_without_sqlite_fts: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let needle = format!("globalsearch{}", Uuid::new_v4().simple());
    let unicode_marker = format!("éclairglobal{}", Uuid::new_v4().simple());
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-global-search-{}", Uuid::new_v4()),
            display_name: "Global Search Subject".to_string(),
        })
        .await
        .expect("create character for global search");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: format!("Sheet field: searchable sheet marker {needle}"),
            author: "mt-045-author".to_string(),
            tool: Some("mt-045-test".to_string()),
        })
        .await
        .expect("append searchable sheet version");
    let note = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: format!("Searchable note {needle}"),
            body_raw_text: format!(
                "Note body carries jump-target proof {needle}. Non-ASCII search proof {unicode_marker}."
            ),
            tags: vec!["global-search".to_string()],
            author: "mt-045-author".to_string(),
        })
        .await
        .expect("create searchable note document");
    let moodboard_doc = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Moodboard,
            title: "Searchable Moodboard Shell".to_string(),
            body_raw_text: "moodboard shell without the search marker".to_string(),
            tags: vec!["moodboard".to_string()],
            author: "mt-045-author".to_string(),
        })
        .await
        .expect("create moodboard document for global search");
    let moodboard_json = serde_json::to_string(&serde_json::json!({
        "schema_id": MOODBOARD_SCHEMA_ID,
        "schema_version": 1,
        "moodboard_id": Uuid::new_v4(),
        "name": format!("Searchable board {needle}"),
        "description": format!("Moodboard snapshot description {needle}"),
        "canvas": {
            "width": 1024.0,
            "height": 768.0,
            "background_color": "#ffffff"
        },
        "layers": [],
        "images": [],
        "text": [],
        "shapes": [],
        "connectors": [],
        "folders": [],
        "guides": [],
        "flags": {
            "locked": false,
            "archived": false,
            "operator_reviewed": true
        },
        "style": {
            "dominant_colors": [],
            "mood_keywords": [needle.clone()],
            "style_description": format!("Global search style {needle}"),
            "suggested_presets": []
        },
        "history": [
            {
                "history_id": Uuid::new_v4(),
                "at": "2026-06-08T09:00:00Z",
                "actor": "mt-045-test",
                "operation": "create",
                "summary": format!("Created searchable moodboard {needle}")
            }
        ]
    }))
    .expect("serialize global search moodboard fixture");
    let moodboard = store
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: moodboard_doc.document_id,
            raw_json_text: moodboard_json,
            author: "mt-045-author".to_string(),
        })
        .await
        .expect("record searchable moodboard snapshot");
    let image_artifact =
        atelier_pg_support::write_native_media_artifact(format!("mt-045-{needle}").as_bytes());
    let image = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: image_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: image_artifact.byte_len,
            source_provenance: Some(format!("searchable image provenance {needle}")),
            artifact_ref: image_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize searchable image asset");

    let hits = store
        .global_search(&needle.to_ascii_uppercase(), 20)
        .await
        .expect("run global search");
    assert_eq!(
        hits.len(),
        4,
        "unique global search marker should hit sheet, note, moodboard snapshot, and image only"
    );
    for hit in &hits {
        assert!(
            hit.jump_target.starts_with("atelier://"),
            "jump target must use a stable Handshake route: {hit:?}"
        );
        assert!(
            !hit.jump_target.to_ascii_lowercase().contains("sqlite"),
            "jump target must not route through SQLite/FTS: {hit:?}"
        );
        assert!(
            hit.snippet.to_ascii_lowercase().contains(&needle),
            "search snippets must include the matched term: {hit:?}"
        );
        assert!(
            hit.snippet.len() <= 180,
            "snippet should stay bounded for operator/model surfaces: {hit:?}"
        );
    }
    assert!(hits.iter().any(|hit| {
        hit.target_kind == "sheet"
            && hit.target_id == sheet.version_id.to_string()
            && hit.jump_target
                == format!(
                    "atelier://sheet/{}/{}",
                    character.internal_id, sheet.version_id
                )
    }));
    assert!(hits.iter().any(|hit| {
        hit.target_kind == "note"
            && hit.target_id == note.document_id.to_string()
            && hit.jump_target == format!("atelier://document/{}", note.document_id)
    }));
    assert!(hits.iter().any(|hit| {
        hit.target_kind == "moodboard_snapshot"
            && hit.target_id == moodboard.snapshot_id.to_string()
            && hit.jump_target == format!("atelier://moodboard/{}", moodboard.snapshot_id)
    }));
    assert!(hits.iter().any(|hit| {
        hit.target_kind == "image"
            && hit.target_id == image.asset_id.to_string()
            && hit.jump_target == format!("atelier://image/{}", image.asset_id)
    }));
    let limited = store
        .global_search(&needle, 2)
        .await
        .expect("run limited global search");
    assert_eq!(limited.len(), 2, "global search applies caller hit limits");
    let unicode_hits = store
        .global_search(&unicode_marker.to_uppercase(), 10)
        .await
        .expect("run mixed-case non-ASCII global search");
    assert_eq!(
        unicode_hits.len(),
        1,
        "mixed-case non-ASCII search should find the note containing its exact marker"
    );
    let unicode_hit = &unicode_hits[0];
    assert_eq!(unicode_hit.target_kind, "note");
    assert_eq!(unicode_hit.target_id, note.document_id.to_string());
    assert!(
        unicode_hit.snippet.to_lowercase().contains(&unicode_marker),
        "non-ASCII search snippets must include the matched marker: {unicode_hit:?}"
    );
    assert!(
        store.global_search("   ", 10).await.is_err(),
        "blank global search queries must be rejected"
    );
}

#[tokio::test]
async fn atelier_lens_search_filters_default_tier1_and_sfw_hard_drop_without_mutation() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_lens_search_filters_default_tier1_and_sfw_hard_drop_without_mutation: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let needle = format!("lensfilter{}", Uuid::new_v4().simple());
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-lens-filter-{}", Uuid::new_v4()),
            display_name: "Lens Filter Subject".to_string(),
        })
        .await
        .expect("create character for lens filter search");

    let tier1_sfw_body =
        format!("Lens proof {needle} lens_extraction_tier=tier1 content_tier=sfw source-alpha");
    let tier2_sfw_body =
        format!("Lens proof {needle} lens_extraction_tier=tier2 content_tier=sfw source-beta");
    let tier1_adult_body = format!(
        "Lens proof {needle} lens_extraction_tier=tier1 content_tier=adult_explicit source-gamma"
    );

    let tier1_sfw = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Tier1 SFW lens note".to_string(),
            body_raw_text: tier1_sfw_body.clone(),
            tags: vec!["lens-search".to_string()],
            author: "mt-046-author".to_string(),
        })
        .await
        .expect("create tier1 sfw note");
    let tier2_sfw = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Tier2 SFW lens note".to_string(),
            body_raw_text: tier2_sfw_body.clone(),
            tags: vec!["lens-search".to_string()],
            author: "mt-046-author".to_string(),
        })
        .await
        .expect("create tier2 sfw note");
    let tier1_adult = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Note,
            title: "Tier1 adult lens note".to_string(),
            body_raw_text: tier1_adult_body.clone(),
            tags: vec!["lens-search".to_string()],
            author: "mt-046-author".to_string(),
        })
        .await
        .expect("create tier1 adult note");

    let event_count_before_search: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM atelier_event")
        .fetch_one(store.pool())
        .await
        .expect("count atelier events before read-only lens search");

    let default_hits = store
        .global_search(&needle, 10)
        .await
        .expect("run default lens search");
    assert_eq!(
        default_hits.len(),
        2,
        "default Lens search must expose Tier1 candidates only"
    );
    assert!(
        default_hits.iter().all(|hit| {
            hit.extraction_tier == LensExtractionTier::Tier1 && hit.view_mode == LensViewMode::Nsfw
        }),
        "default hits should be Tier1 under NSFW view mode: {default_hits:?}"
    );
    assert!(default_hits.iter().any(|hit| {
        hit.target_kind == "note"
            && hit.target_id == tier1_sfw.document_id.to_string()
            && hit.content_tier == Some(LensContentTier::Sfw)
    }));
    assert!(default_hits.iter().any(|hit| {
        hit.target_kind == "note"
            && hit.target_id == tier1_adult.document_id.to_string()
            && hit.content_tier == Some(LensContentTier::AdultExplicit)
    }));
    assert!(
        default_hits
            .iter()
            .all(|hit| hit.target_id != tier2_sfw.document_id.to_string()),
        "Tier2 candidates must be hidden by the Tier1 default"
    );

    let sfw_tier1_hits = store
        .global_search_with_lens_filters(
            &needle,
            10,
            LensSearchFilters {
                extraction_tier: LensExtractionTier::Tier1,
                view_mode: LensViewMode::Sfw,
            },
        )
        .await
        .expect("run tier1 sfw lens search");
    assert_eq!(
        sfw_tier1_hits.len(),
        1,
        "SFW view mode must hard-drop adult and unknown candidates"
    );
    assert_eq!(
        sfw_tier1_hits[0].target_id,
        tier1_sfw.document_id.to_string()
    );
    assert_eq!(sfw_tier1_hits[0].content_tier, Some(LensContentTier::Sfw));
    assert_eq!(sfw_tier1_hits[0].view_mode, LensViewMode::Sfw);

    let sfw_tier2_hits = store
        .global_search_with_lens_filters(
            &needle,
            10,
            LensSearchFilters {
                extraction_tier: LensExtractionTier::Tier2,
                view_mode: LensViewMode::Sfw,
            },
        )
        .await
        .expect("run tier2 sfw lens search");
    assert_eq!(
        sfw_tier2_hits.len(),
        2,
        "Tier2 Lens search should include Tier1 plus Tier2 SFW candidates"
    );
    assert!(sfw_tier2_hits.iter().any(|hit| {
        hit.target_id == tier1_sfw.document_id.to_string()
            && hit.extraction_tier == LensExtractionTier::Tier1
            && hit.content_tier == Some(LensContentTier::Sfw)
    }));
    assert!(sfw_tier2_hits.iter().any(|hit| {
        hit.target_id == tier2_sfw.document_id.to_string()
            && hit.extraction_tier == LensExtractionTier::Tier2
            && hit.content_tier == Some(LensContentTier::Sfw)
    }));
    assert!(
        sfw_tier2_hits
            .iter()
            .all(|hit| hit.target_id != tier1_adult.document_id.to_string()),
        "SFW view mode must hard-drop adult Tier1 rows instead of relabeling them"
    );

    let event_count_after_search: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM atelier_event")
        .fetch_one(store.pool())
        .await
        .expect("count atelier events after read-only lens search");
    assert_eq!(
        event_count_after_search, event_count_before_search,
        "Lens search filters are read-only and must not emit mutation events"
    );
    assert_eq!(
        store
            .latest_character_document_version(tier1_sfw.document_id)
            .await
            .expect("read tier1 sfw document after search")
            .expect("tier1 sfw document exists")
            .body_raw_text,
        tier1_sfw_body,
        "SFW filtering must not mutate SFW source text"
    );
    assert_eq!(
        store
            .latest_character_document_version(tier2_sfw.document_id)
            .await
            .expect("read tier2 sfw document after search")
            .expect("tier2 sfw document exists")
            .body_raw_text,
        tier2_sfw_body,
        "extraction-tier filtering must not mutate hidden Tier2 source text"
    );
    assert_eq!(
        store
            .latest_character_document_version(tier1_adult.document_id)
            .await
            .expect("read tier1 adult document after search")
            .expect("tier1 adult document exists")
            .body_raw_text,
        tier1_adult_body,
        "SFW hard-drop must not rewrite adult source text"
    );
}

#[tokio::test]
async fn atelier_saved_searches_reproduce_filters_and_retrieval_projection() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_saved_searches_reproduce_filters_and_retrieval_projection: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let marker = format!("saved-search-{}", Uuid::new_v4());
    let keep_artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-keep").as_bytes());
    let excluded_artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-excluded").as_bytes());
    let adult_artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-adult").as_bytes());
    let wrong_color_artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-wrong-color").as_bytes());
    let outside_scope_artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-outside").as_bytes());

    let keep = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: keep_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: keep_artifact.byte_len,
            source_provenance: Some(format!("{marker} content_tier=sfw keep")),
            artifact_ref: keep_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize saved-search keep asset");
    let excluded = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: excluded_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: excluded_artifact.byte_len,
            source_provenance: Some(format!("{marker} content_tier=sfw excluded")),
            artifact_ref: excluded_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize saved-search excluded asset");
    let adult = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: adult_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: adult_artifact.byte_len,
            source_provenance: Some(format!("{marker} content_tier=adult_explicit adult")),
            artifact_ref: adult_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize saved-search adult asset");
    let wrong_color = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: wrong_color_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: wrong_color_artifact.byte_len,
            source_provenance: Some(format!("{marker} content_tier=sfw wrong-color")),
            artifact_ref: wrong_color_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize saved-search wrong-color asset");
    let outside_scope = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: outside_scope_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: outside_scope_artifact.byte_len,
            source_provenance: Some(format!("{marker} content_tier=sfw outside-scope")),
            artifact_ref: outside_scope_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize saved-search outside-scope asset");

    let collection = store
        .create_collection(&handshake_core::atelier::collections::NewCollection {
            name: format!("Saved Search Collection {marker}"),
            notes: "scope fixture".to_string(),
            tags: vec!["saved-search".to_string()],
            character_internal_id: None,
            sheet_version_id: None,
        })
        .await
        .expect("create saved-search scope collection");
    store
        .add_images_to_collection(
            collection.collection_id,
            &[
                keep.asset_id,
                excluded.asset_id,
                adult.asset_id,
                wrong_color.asset_id,
            ],
        )
        .await
        .expect("add scoped assets to collection");

    for asset_id in [
        keep.asset_id,
        excluded.asset_id,
        adult.asset_id,
        wrong_color.asset_id,
        outside_scope.asset_id,
    ] {
        store
            .tag_media_asset(asset_id, "Portrait", "mt-047-test")
            .await
            .expect("tag saved-search fixture asset");
    }
    store
        .tag_media_asset(excluded.asset_id, "reject", "mt-047-test")
        .await
        .expect("tag excluded saved-search fixture asset");
    store
        .bulk_update_media_review_metadata(
            &[
                handshake_core::atelier::MediaReviewMetadataUpdate {
                    asset_id: keep.asset_id,
                    favorite: true,
                    rating: 5,
                    frontpage: false,
                    carousel: false,
                    notes: Some("keep".to_string()),
                    review_status: "approved".to_string(),
                },
                handshake_core::atelier::MediaReviewMetadataUpdate {
                    asset_id: excluded.asset_id,
                    favorite: true,
                    rating: 5,
                    frontpage: false,
                    carousel: false,
                    notes: Some("excluded".to_string()),
                    review_status: "approved".to_string(),
                },
                handshake_core::atelier::MediaReviewMetadataUpdate {
                    asset_id: adult.asset_id,
                    favorite: true,
                    rating: 5,
                    frontpage: false,
                    carousel: false,
                    notes: Some("adult".to_string()),
                    review_status: "approved".to_string(),
                },
                handshake_core::atelier::MediaReviewMetadataUpdate {
                    asset_id: wrong_color.asset_id,
                    favorite: true,
                    rating: 5,
                    frontpage: false,
                    carousel: false,
                    notes: Some("wrong color".to_string()),
                    review_status: "approved".to_string(),
                },
                handshake_core::atelier::MediaReviewMetadataUpdate {
                    asset_id: outside_scope.asset_id,
                    favorite: true,
                    rating: 5,
                    frontpage: false,
                    carousel: false,
                    notes: Some("outside scope".to_string()),
                    review_status: "approved".to_string(),
                },
            ],
            "mt-047-reviewer",
        )
        .await
        .expect("write saved-search review metadata");
    for (asset_id, color) in [
        (keep.asset_id, "#11aa22"),
        (excluded.asset_id, "#11aa22"),
        (adult.asset_id, "#11aa22"),
        (wrong_color.asset_id, "#2244ff"),
        (outside_scope.asset_id, "#11aa22"),
    ] {
        store
            .upsert_similarity_projection(
                asset_id,
                Some("0123456789abcdef"),
                serde_json::json!({
                    "algorithm": "mt047-fixture",
                    "dominant": [{"hex": color, "count": 10, "ratio": 1.0}]
                }),
            )
            .await
            .expect("write saved-search color projection");
    }

    let saved = store
        .save_saved_search(&NewSavedSearch {
            name: format!("MT-047 saved search {marker}"),
            filters: SavedSearchFilters {
                include_tags: vec!["Portrait".to_string()],
                exclude_tags: vec!["reject".to_string()],
                min_rating: Some(4),
                favorite: Some(true),
                color_hex: Some("#11AA22".to_string()),
                scope: SavedSearchScope::Collection(collection.collection_id),
                view_mode: LensViewMode::Sfw,
            },
            created_by: "mt-047-author".to_string(),
        })
        .await
        .expect("save reusable saved search");
    let reloaded = store
        .get_saved_search(saved.saved_search_id)
        .await
        .expect("reload saved search")
        .expect("saved search exists");
    assert_eq!(reloaded.filters.include_tags, vec!["portrait"]);
    assert_eq!(reloaded.filters.exclude_tags, vec!["reject"]);
    assert_eq!(reloaded.filters.color_hex.as_deref(), Some("#11aa22"));
    assert_eq!(
        reloaded.filters.scope,
        SavedSearchScope::Collection(collection.collection_id)
    );
    assert_eq!(reloaded.filters.view_mode, LensViewMode::Sfw);

    let projection = store
        .run_saved_search(saved.saved_search_id, 20)
        .await
        .expect("run saved search projection");
    assert_eq!(
        projection.len(),
        1,
        "saved search should reproduce tag include/exclude, rating, favorite, color, scope, and SFW ViewMode"
    );
    assert_eq!(projection[0].asset_id, keep.asset_id);
    assert_eq!(projection[0].tags, vec!["portrait"]);
    assert_eq!(projection[0].rating, 5);
    assert!(projection[0].favorite);
    assert_eq!(projection[0].matched_color_hex.as_deref(), Some("#11aa22"));
    assert_eq!(projection[0].content_tier, Some(LensContentTier::Sfw));
    assert_eq!(projection[0].view_mode, LensViewMode::Sfw);
    assert_eq!(
        projection[0].jump_target,
        format!("atelier://image/{}", keep.asset_id)
    );

    let view_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_saved_search_retrieval_projection WHERE saved_search_id = $1",
    )
    .bind(saved.saved_search_id)
    .fetch_one(store.pool())
    .await
    .expect("count saved-search retrieval projection rows");
    assert_eq!(
        view_count, 1,
        "database projection should match the API projection"
    );
    let event_count = store
        .count_events_for_aggregate(
            search_event_family::SAVED_SEARCH_UPSERTED,
            "atelier_saved_search",
            &saved.saved_search_id.to_string(),
        )
        .await
        .expect("count saved-search EventLedger upsert events");
    assert_eq!(
        event_count, 1,
        "saving a saved search must emit one EventLedger event"
    );
}

#[tokio::test]
async fn atelier_search_tags_rules_and_similarity() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_search_tags_rules_and_similarity: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    // --- ensure_tag dedups identical (normalized) text ---
    let tag_text = format!("Blonde-{}", Uuid::new_v4());
    let tag1 = store.ensure_tag(&tag_text).await.expect("ensure tag");
    let tag2 = store
        .ensure_tag(&format!("  {}  ", tag_text.to_uppercase()))
        .await
        .expect("ensure tag again (different case/whitespace)");
    assert_eq!(
        tag1.tag_id, tag2.tag_id,
        "identical normalized text dedups to the same tag row"
    );
    assert_eq!(
        tag1.text,
        tag_text.to_ascii_lowercase(),
        "tag text is normalized"
    );

    // --- tag_character then list_character_tags ---
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-search-{}", Uuid::new_v4()),
            display_name: "Search Subject".to_string(),
        })
        .await
        .expect("create character");
    store
        .tag_character(character.internal_id, &tag_text, TagType::Manual)
        .await
        .expect("tag character manually");
    let manual_tags = store
        .list_character_tags(character.internal_id)
        .await
        .expect("list character tags");
    assert!(
        manual_tags
            .iter()
            .any(|t| t.text == tag_text.to_ascii_lowercase() && t.tag_type == TagType::Manual),
        "the manual tag is attached to the character"
    );

    // --- create_tag_rule then recompute_derived_tags emits derived tags ---
    let field_id = format!("hair-{}", Uuid::new_v4());
    let emit = format!("derived-blonde-{}", Uuid::new_v4());
    store
        .create_tag_rule(&NewTagRule {
            source_field_id: field_id.clone(),
            match_type: MatchType::Contains,
            pattern: "blonde".to_string(),
            emit_tag: emit.clone(),
            enabled: true,
        })
        .await
        .expect("create tag rule");

    let mut values = HashMap::new();
    values.insert(field_id.clone(), "long blonde hair".to_string());
    let derived = store
        .recompute_derived_tags(character.internal_id, &values)
        .await
        .expect("recompute derived tags");
    let emit_norm = emit.to_ascii_lowercase();
    assert!(
        derived.contains(&emit_norm),
        "matching rule emits its derived tag: {derived:?} should contain {emit_norm}"
    );
    let after = store
        .list_character_tags(character.internal_id)
        .await
        .expect("list tags after recompute");
    assert!(
        after
            .iter()
            .any(|t| t.text == emit_norm && t.tag_type == TagType::Derived),
        "derived tag is persisted with Derived provenance"
    );

    // --- upsert_similarity_projection then find_similar_assets by dHash ---
    let asset_near = fresh_asset(&store).await;
    let asset_far = fresh_asset(&store).await;
    // Target hash and a near hash differing by a single bit (distance 1).
    // Use a run-unique target so a persistent live DB cannot fill the 50-hit
    // cap with older rows that share the same fixed demo hash.
    let target_seed = Uuid::new_v4().as_u128() as u64;
    let target_hash = format!("{target_seed:016x}");
    let near_hash = format!("{:016x}", target_seed ^ 1);
    let far_hash = format!("{:016x}", !target_seed);

    store
        .upsert_similarity_projection(
            asset_near,
            Some(&near_hash),
            serde_json::json!({ "dominant": ["#000000"] }),
        )
        .await
        .expect("project near asset");
    store
        .upsert_similarity_projection(
            asset_far,
            Some(&far_hash),
            serde_json::json!({ "dominant": ["#ffffff"] }),
        )
        .await
        .expect("project far asset");

    // Search within a tight threshold: the near asset (distance 1) is a hit,
    // the far asset (distance 64) is excluded.
    let hits = store
        .find_similar_assets(&target_hash, 4, 50, None)
        .await
        .expect("find similar assets");
    assert!(
        hits.iter().any(|h| h.asset_internal_id == asset_near),
        "the near (1-bit) asset is returned within the threshold"
    );
    assert!(
        !hits.iter().any(|h| h.asset_internal_id == asset_far),
        "the far (64-bit) asset is excluded by the threshold"
    );
    let near_hit = hits
        .iter()
        .find(|h| h.asset_internal_id == asset_near)
        .expect("near hit present");
    assert_eq!(near_hit.distance, 1, "Hamming distance is exactly 1");
}

#[tokio::test]
async fn atelier_ai_tag_suggestions_are_reviewable_proposals_not_auto_truth() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_ai_tag_suggestions_are_reviewable_proposals_not_auto_truth: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-ai-suggest-{}", Uuid::new_v4()),
            display_name: "AI Suggestion Subject".to_string(),
        })
        .await
        .expect("create character for AI tag suggestion");
    let asset_id = fresh_asset(&store).await;

    let suggested = store
        .record_ai_tag_suggestion(&NewAiTagSuggestion {
            character_internal_id: character.internal_id,
            asset_id: Some(asset_id),
            tag_text: "  Cinematic Lighting  ".to_string(),
            confidence: Some(0.87),
            model_receipt_ref: format!("receipt://atelier/model/{}", Uuid::new_v4()),
            tool_receipt_ref: format!("receipt://atelier/tool/{}", Uuid::new_v4()),
            suggested_by: "mt-021-model".to_string(),
        })
        .await
        .expect("record AI tag suggestion");
    assert_eq!(suggested.status, AiTagSuggestionStatus::Proposed);
    assert_eq!(suggested.tag_text, "cinematic lighting");
    assert_eq!(suggested.asset_id, Some(asset_id));
    assert_eq!(
        store
            .list_character_tags(character.internal_id)
            .await
            .expect("list tags before applying suggestion")
            .len(),
        0,
        "recording an AI suggestion must not auto-apply a tag"
    );

    let rejected = store
        .record_ai_tag_suggestion(&NewAiTagSuggestion {
            character_internal_id: character.internal_id,
            asset_id: None,
            tag_text: "wrong tag".to_string(),
            confidence: Some(0.2),
            model_receipt_ref: format!("receipt://atelier/model/{}", Uuid::new_v4()),
            tool_receipt_ref: format!("receipt://atelier/tool/{}", Uuid::new_v4()),
            suggested_by: "mt-021-model".to_string(),
        })
        .await
        .expect("record rejected AI tag suggestion");
    let rejected = store
        .reject_ai_tag_suggestion(&AiTagSuggestionDecision {
            suggestion_id: rejected.suggestion_id,
            decided_by: "mt-021-reviewer".to_string(),
            reason: Some("not visually present".to_string()),
        })
        .await
        .expect("reject AI tag suggestion");
    assert_eq!(rejected.status, AiTagSuggestionStatus::Rejected);
    assert!(
        store
            .apply_ai_tag_suggestion(rejected.suggestion_id, "mt-021-reviewer")
            .await
            .is_err(),
        "rejected suggestions must not be applied"
    );

    let accepted = store
        .accept_ai_tag_suggestion(&AiTagSuggestionDecision {
            suggestion_id: suggested.suggestion_id,
            decided_by: "mt-021-reviewer".to_string(),
            reason: Some("matches image".to_string()),
        })
        .await
        .expect("accept AI tag suggestion");
    assert_eq!(accepted.status, AiTagSuggestionStatus::Accepted);
    let accept_payload: serde_json::Value = sqlx::query(
        r#"SELECT ae.payload
           FROM atelier_event ae
           JOIN kernel_event_ledger kel
             ON kel.event_id = ae.kernel_event_id
            AND kel.event_sequence = ae.kernel_event_sequence
           WHERE ae.event_family = $1
             AND ae.aggregate_type = 'atelier_ai_tag_suggestion'
             AND ae.aggregate_id = $2
             AND kel.source_component = 'atelier'
           ORDER BY ae.created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(search_event_family::AI_TAG_SUGGESTION_ACCEPTED)
    .bind(accepted.suggestion_id.to_string())
    .fetch_one(store.pool())
    .await
    .expect("read accepted AI suggestion event payload")
    .get("payload");
    assert!(
        accept_payload.get("decision_reason").is_none(),
        "raw AI suggestion decision reason must not be emitted into EventLedger payload"
    );
    let reason_ref = accept_payload
        .get("decision_reason_ref")
        .and_then(serde_json::Value::as_str)
        .expect("decision reason is represented by a ref hash");
    assert_ne!(
        reason_ref, "matches image",
        "decision_reason_ref must not contain raw reason text"
    );
    let applied = store
        .apply_ai_tag_suggestion(accepted.suggestion_id, "mt-021-reviewer")
        .await
        .expect("apply accepted AI tag suggestion");
    assert_eq!(applied.status, AiTagSuggestionStatus::Applied);
    assert!(applied.applied_tag_id.is_some());

    let tags = store
        .list_character_tags(character.internal_id)
        .await
        .expect("list tags after accepted suggestion applied");
    assert!(
        tags.iter()
            .any(|tag| tag.text == "cinematic lighting" && tag.tag_type == TagType::Manual),
        "accepted AI suggestion applies as an explicit reviewed manual tag"
    );
    let proposals = store
        .list_ai_tag_suggestions_for_character(character.internal_id)
        .await
        .expect("list AI tag suggestions");
    assert_eq!(
        proposals.len(),
        2,
        "accepted/applied and rejected suggestions remain auditable proposals"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                search_event_family::AI_TAG_SUGGESTION_APPLIED,
                "atelier_ai_tag_suggestion",
                &applied.suggestion_id.to_string(),
            )
            .await
            .expect("count AI suggestion applied event"),
        1,
        "applying an accepted suggestion writes EventLedger evidence"
    );
}

#[tokio::test]
async fn atelier_ai_tag_suggestions_require_model_and_tool_receipt_refs() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_ai_tag_suggestions_require_model_and_tool_receipt_refs: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-ai-receipt-{}", Uuid::new_v4()),
            display_name: "AI Receipt Subject".to_string(),
        })
        .await
        .expect("create character for AI receipt validation");
    let asset_id = fresh_asset(&store).await;
    let before_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_ai_tag_suggestion WHERE character_internal_id = $1",
    )
    .bind(character.internal_id)
    .fetch_one(store.pool())
    .await
    .expect("count AI tag suggestions before invalid receipt refs");
    let before_events: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM atelier_event ae
           JOIN atelier_ai_tag_suggestion suggestion
             ON suggestion.suggestion_id::text = ae.aggregate_id
           WHERE ae.event_family = $1
             AND ae.aggregate_type = 'atelier_ai_tag_suggestion'
             AND suggestion.character_internal_id = $2"#,
    )
    .bind(search_event_family::AI_TAG_SUGGESTION_RECORDED)
    .bind(character.internal_id)
    .fetch_one(store.pool())
    .await
    .expect("count AI suggestion events for this character before invalid receipt refs");

    let invalid_model = store
        .record_ai_tag_suggestion(&NewAiTagSuggestion {
            character_internal_id: character.internal_id,
            asset_id: Some(asset_id),
            tag_text: "bad model ref".to_string(),
            confidence: Some(0.5),
            model_receipt_ref: "model-worker-output-1".to_string(),
            tool_receipt_ref: format!("receipt://atelier/tool/{}", Uuid::new_v4()),
            suggested_by: "mt-021-model".to_string(),
        })
        .await
        .expect_err("plain model output ids must not satisfy model_receipt_ref");
    assert!(
        invalid_model.to_string().contains("model_receipt_ref"),
        "unexpected invalid model receipt error: {invalid_model}"
    );

    let invalid_tool = store
        .record_ai_tag_suggestion(&NewAiTagSuggestion {
            character_internal_id: character.internal_id,
            asset_id: Some(asset_id),
            tag_text: "bad tool ref".to_string(),
            confidence: Some(0.5),
            model_receipt_ref: format!("receipt://atelier/model/{}", Uuid::new_v4()),
            tool_receipt_ref: "tool-output-1".to_string(),
            suggested_by: "mt-021-model".to_string(),
        })
        .await
        .expect_err("plain tool output ids must not satisfy tool_receipt_ref");
    assert!(
        invalid_tool.to_string().contains("tool_receipt_ref"),
        "unexpected invalid tool receipt error: {invalid_tool}"
    );

    let after_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_ai_tag_suggestion WHERE character_internal_id = $1",
    )
    .bind(character.internal_id)
    .fetch_one(store.pool())
    .await
    .expect("count AI tag suggestions after invalid receipt refs");
    assert_eq!(
        after_rows, before_rows,
        "invalid AI receipt refs must not persist proposal rows"
    );
    let after_events: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM atelier_event ae
           JOIN atelier_ai_tag_suggestion suggestion
             ON suggestion.suggestion_id::text = ae.aggregate_id
           WHERE ae.event_family = $1
             AND ae.aggregate_type = 'atelier_ai_tag_suggestion'
             AND suggestion.character_internal_id = $2"#,
    )
    .bind(search_event_family::AI_TAG_SUGGESTION_RECORDED)
    .bind(character.internal_id)
    .fetch_one(store.pool())
    .await
    .expect("count AI suggestion events for this character after invalid receipt refs");
    assert_eq!(
        after_events, before_events,
        "invalid AI receipt refs must not emit proposal events"
    );
}

#[tokio::test]
async fn atelier_exports_request_result_idempotency_and_manifest() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_exports_request_result_idempotency_and_manifest: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    // --- create a character + an append-only sheet version (foundation) ---
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-export-{}", Uuid::new_v4()),
            display_name: "Export Subject".to_string(),
        })
        .await
        .expect("create character");
    let version = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "name: Export Subject".to_string(),
            author: "operator".to_string(),
            tool: None,
        })
        .await
        .expect("append sheet version");

    // --- request_sheet_export pinned to that version ---
    let request = store
        .request_sheet_export(&NewExportRequest {
            character_internal_id: character.internal_id,
            sheet_version_id: version.version_id,
            format: ExportFormat::Markdown,
            label: Some("share pack".to_string()),
            requested_by: "operator".to_string(),
        })
        .await
        .expect("request sheet export");
    assert_eq!(request.format, ExportFormat::Markdown);

    // --- record_export_result, then re-record identical content_hash is idempotent ---
    let artifact_ref = format!("artifact://atelier/export/{}", Uuid::new_v4());
    let content_hash = format!("sha256-{}", Uuid::new_v4());
    let result1 = store
        .record_export_result(request.export_id, &artifact_ref, &content_hash, 2048)
        .await
        .expect("record export result");
    let result2 = store
        .record_export_result(request.export_id, &artifact_ref, &content_hash, 2048)
        .await
        .expect("re-record identical export result");
    assert_eq!(
        result1.result_id, result2.result_id,
        "re-recording identical (export_id, content_hash) returns the same result"
    );

    // --- add_manifest_entry seq increments ---
    let entry1 = store
        .add_manifest_entry(
            request.export_id,
            ManifestItemKind::Sheet,
            &artifact_ref,
            "sheet/character.md",
        )
        .await
        .expect("add sheet manifest entry");
    assert_eq!(entry1.seq, 1, "first manifest entry is seq 1");

    let media_ref = format!("artifact://atelier/media/{}", Uuid::new_v4());
    let entry2 = store
        .add_manifest_entry(
            request.export_id,
            ManifestItemKind::Media,
            &media_ref,
            "images/a.png",
        )
        .await
        .expect("add media manifest entry");
    assert_eq!(entry2.seq, 2, "second manifest entry is seq 2 (increments)");

    // --- export_manifest lists in seq order ---
    let manifest = store
        .export_manifest(request.export_id)
        .await
        .expect("read export manifest");
    assert_eq!(manifest.len(), 2, "two manifest entries recorded");
    assert_eq!(manifest[0].seq, 1, "manifest ordered ascending by seq");
    assert_eq!(manifest[0].kind, ManifestItemKind::Sheet);
    assert_eq!(manifest[1].seq, 2);
    assert_eq!(manifest[1].kind, ManifestItemKind::Media);
}

#[tokio::test]
async fn atelier_share_pack_subset_manifest_includes_usage_readme_and_rejects_gov_paths() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_share_pack_subset_manifest_includes_usage_readme_and_rejects_gov_paths: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let marker = format!("share-pack-{}", Uuid::new_v4());

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-{marker}"),
            display_name: "Share Pack Subject".to_string(),
        })
        .await
        .expect("create share-pack character");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "name: Share Pack Subject".to_string(),
            author: "operator".to_string(),
            tool: None,
        })
        .await
        .expect("append share-pack sheet");
    let export = store
        .request_sheet_export(&NewExportRequest {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            format: ExportFormat::Markdown,
            label: Some("safe subset share pack".to_string()),
            requested_by: "mt-071-exporter".to_string(),
        })
        .await
        .expect("request share-pack export");

    let sheet_artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-sheet").as_bytes());
    store
        .record_export_result(
            export.export_id,
            &sheet_artifact.artifact_ref,
            &sheet_artifact.content_hash,
            sheet_artifact.byte_len,
        )
        .await
        .expect("record share-pack sheet result");

    let selected_artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-selected").as_bytes());
    let unselected_artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-unselected").as_bytes());
    let selected = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: selected_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: selected_artifact.byte_len,
            source_provenance: Some(format!("{marker} selected")),
            artifact_ref: selected_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize selected media");
    let unselected = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: unselected_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: unselected_artifact.byte_len,
            source_provenance: Some(format!("{marker} unselected")),
            artifact_ref: unselected_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize unselected media");
    let readme_artifact = atelier_pg_support::write_native_media_artifact(
        format!("{marker}-usage-readme").as_bytes(),
    );

    let build = store
        .build_share_pack_manifest(&SharePackBuildRequest {
            export_id: export.export_id,
            selector: SharePackSubsetSelector {
                include_sheet: true,
                media_asset_ids: vec![selected.asset_id],
            },
            usage_readme: SharePackUsageReadmeArtifact {
                artifact_ref: readme_artifact.artifact_ref.clone(),
                content_hash: readme_artifact.content_hash.clone(),
                byte_len: readme_artifact.byte_len,
            },
            requested_by: "mt-071-exporter".to_string(),
        })
        .await
        .expect("build safe subset share-pack manifest");
    assert_eq!(build.entries.len(), 3, "sheet + selected media + README");
    assert_eq!(build.selected_media_count, 1);
    assert!(
        build
            .entries
            .iter()
            .any(|entry| entry.kind == ManifestItemKind::UsageReadme
                && entry.pack_path == "README.md"),
        "share pack includes a usage README manifest item"
    );

    let manifest = store
        .export_manifest(export.export_id)
        .await
        .expect("read built share-pack manifest");
    assert_eq!(manifest.len(), 3);
    assert!(
        manifest.iter().any(|entry| {
            entry.kind == ManifestItemKind::Media
                && entry.artifact_ref == selected_artifact.artifact_ref
                && entry.pack_path.contains(&selected.asset_id.to_string())
        }),
        "subset selector includes the selected media asset"
    );
    assert!(
        !manifest.iter().any(|entry| {
            entry.artifact_ref == unselected_artifact.artifact_ref
                || entry.pack_path.contains(&unselected.asset_id.to_string())
        }),
        "subset selector excludes unselected media"
    );
    assert!(
        manifest
            .iter()
            .all(|entry| !entry.pack_path.contains(' ') && !entry.pack_path.contains(".GOV")),
        "share-pack pack paths are portable no-space paths outside .GOV"
    );

    let gov_artifact = store
        .add_manifest_entry(
            export.export_id,
            ManifestItemKind::Media,
            "artifact://.GOV/artifacts/L1/00000000-0000-0000-0000-000000000000/payload",
            "images/gov.png",
        )
        .await
        .expect_err(".GOV artifact refs are rejected");
    assert!(gov_artifact.to_string().contains("artifact_ref"));

    let gov_pack_path = store
        .add_manifest_entry(
            export.export_id,
            ManifestItemKind::Media,
            &selected_artifact.artifact_ref,
            ".GOV/images/leak.png",
        )
        .await
        .expect_err(".GOV pack paths are rejected");
    assert!(gov_pack_path.to_string().contains("pack_path"));
}

fn llm_evidence_source_anchor(marker: &str) -> LlmEvidenceSourceAnchor {
    LlmEvidenceSourceAnchor {
        source_id: format!("source-{marker}"),
        source_path: format!("source-index/{marker}.json"),
        source_range: "lines:1-12".to_string(),
        content_hash: format!("sha256-{marker}"),
    }
}

fn llm_evidence_file(
    kind: LlmEvidencePackFileKind,
    pack_path: &str,
    payload: &str,
    marker: &str,
    redaction_required: bool,
    redacted: bool,
) -> LlmEvidencePackFile {
    let artifact = atelier_pg_support::write_native_media_artifact(payload.as_bytes());
    LlmEvidencePackFile {
        kind,
        pack_path: pack_path.to_string(),
        artifact_ref: artifact.artifact_ref,
        content_hash: artifact.content_hash,
        byte_len: artifact.byte_len,
        source_anchors: vec![llm_evidence_source_anchor(marker)],
        redaction_required,
        redacted,
    }
}

#[test]
fn atelier_llm_evidence_pack_contract_is_strict_deterministic_and_redaction_aware() {
    let marker = Uuid::new_v4().to_string();
    let files = vec![
        llm_evidence_file(
            LlmEvidencePackFileKind::SourceIndex,
            "source-index.json",
            "source-index",
            &marker,
            false,
            false,
        ),
        llm_evidence_file(
            LlmEvidencePackFileKind::Readme,
            "README.md",
            "readme",
            &marker,
            false,
            false,
        ),
        llm_evidence_file(
            LlmEvidencePackFileKind::Evidence,
            "evidence.json",
            "redacted-evidence",
            &marker,
            true,
            true,
        ),
        llm_evidence_file(
            LlmEvidencePackFileKind::RedactionReport,
            "redactions.json",
            "redactions",
            &marker,
            false,
            false,
        ),
    ];

    let manifest = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        "mt-072-evidence-exporter".to_string(),
        files,
    )
    .expect("build strict LLM evidence-pack manifest");
    validate_llm_evidence_pack_manifest(&manifest)
        .expect("valid evidence-pack manifest passes strict validation");
    assert_eq!(manifest.schema_id, LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID);
    assert_eq!(
        manifest
            .files
            .iter()
            .map(|file| file.pack_path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "README.md",
            "evidence.json",
            "redactions.json",
            "source-index.json"
        ],
        "manifest files are sorted into deterministic model-consumable order"
    );
    assert!(
        manifest
            .files
            .iter()
            .all(|file| !file.source_anchors.is_empty()),
        "every file must carry source anchors"
    );
    assert!(
        manifest.files.iter().any(|file| {
            file.kind == LlmEvidencePackFileKind::Evidence
                && file.redaction_required
                && file.redacted
        }),
        "sensitive evidence must carry explicit redaction flags"
    );

    let missing_required = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        "mt-072-evidence-exporter".to_string(),
        manifest
            .files
            .iter()
            .filter(|file| file.kind != LlmEvidencePackFileKind::RedactionReport)
            .cloned()
            .collect(),
    )
    .expect_err("missing required redactions.json is rejected");
    assert!(missing_required.to_string().contains("redactions.json"));

    let mut missing_anchor_files = manifest.files.clone();
    missing_anchor_files[1].source_anchors.clear();
    let missing_anchor = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        "mt-072-evidence-exporter".to_string(),
        missing_anchor_files,
    )
    .expect_err("files without source anchors are rejected");
    assert!(missing_anchor.to_string().contains("source_anchors"));

    let mut unredacted_sensitive_files = manifest.files.clone();
    unredacted_sensitive_files[1].redacted = false;
    let unredacted_sensitive = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        "mt-072-evidence-exporter".to_string(),
        unredacted_sensitive_files,
    )
    .expect_err("redaction-required evidence without redacted flag is rejected");
    assert!(unredacted_sensitive.to_string().contains("redacted"));

    let mut gov_pack_files = manifest.files.clone();
    gov_pack_files[1].pack_path = ".GOV/evidence.json".to_string();
    let gov_pack = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        "mt-072-evidence-exporter".to_string(),
        gov_pack_files,
    )
    .expect_err(".GOV pack paths are rejected");
    assert!(gov_pack.to_string().contains("pack_path"));
}

#[tokio::test]
async fn atelier_web_portfolio_export_records_portable_manifest_contract() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_web_portfolio_export_records_portable_manifest_contract: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let marker = format!("web-portfolio-{}", Uuid::new_v4());

    let hero_artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-hero").as_bytes());
    let detail_artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-detail").as_bytes());

    let hero = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: hero_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: hero_artifact.byte_len,
            source_provenance: Some(format!("{marker} hero")),
            artifact_ref: hero_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize web portfolio hero asset");
    let detail = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: detail_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: detail_artifact.byte_len,
            source_provenance: Some(format!("{marker} detail")),
            artifact_ref: detail_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize web portfolio detail asset");

    let collection = store
        .create_collection(&NewCollection {
            name: format!("Web Portfolio {marker}"),
            notes: "MT-048 source collection".to_string(),
            tags: vec!["web-portfolio".to_string()],
            character_internal_id: None,
            sheet_version_id: None,
        })
        .await
        .expect("create web portfolio collection");
    store
        .add_images_to_collection(collection.collection_id, &[hero.asset_id, detail.asset_id])
        .await
        .expect("add web portfolio members");

    let slug = format!("web-portfolio-{}", Uuid::new_v4().simple());
    let request = store
        .request_web_portfolio_export(&NewWebPortfolioExportRequest {
            source_collection_id: collection.collection_id,
            slug: format!("  {slug}  "),
            title: "Portfolio Contract Proof".to_string(),
            requested_by: "mt-048-exporter".to_string(),
        })
        .await
        .expect("request portable web portfolio export");
    assert_eq!(request.source_collection_id, collection.collection_id);
    assert_eq!(request.slug, slug, "slug is trimmed to its portable token");

    let manifest_payload = format!("{marker}-manifest").into_bytes();
    let manifest_artifact = atelier_pg_support::write_native_media_artifact(&manifest_payload);
    let result = store
        .record_web_portfolio_export_result(
            request.portfolio_export_id,
            &manifest_artifact.artifact_ref,
            &manifest_artifact.content_hash,
            manifest_artifact.byte_len,
            &[
                WebPortfolioManifestItem {
                    asset_id: hero.asset_id,
                    artifact_ref: hero_artifact.artifact_ref.clone(),
                    pack_path: format!("images/{}-hero.png", hero.asset_id),
                    content_hash: hero_artifact.content_hash.clone(),
                    byte_len: hero_artifact.byte_len,
                },
                WebPortfolioManifestItem {
                    asset_id: detail.asset_id,
                    artifact_ref: detail_artifact.artifact_ref.clone(),
                    pack_path: format!("images/{}-detail.png", detail.asset_id),
                    content_hash: detail_artifact.content_hash.clone(),
                    byte_len: detail_artifact.byte_len,
                },
            ],
        )
        .await
        .expect("record web portfolio ArtifactStore result and manifest");

    assert_eq!(result.portfolio_export_id, request.portfolio_export_id);
    assert_eq!(result.artifact_ref, manifest_artifact.artifact_ref);
    assert_eq!(result.content_hash, manifest_artifact.content_hash);
    assert_eq!(result.byte_len, manifest_artifact.byte_len);
    assert_eq!(
        result.manifest_json["schema_id"],
        serde_json::json!(WEB_PORTFOLIO_MANIFEST_SCHEMA_ID)
    );
    assert_eq!(result.manifest_json["slug"], serde_json::json!(slug));
    assert_eq!(
        result.manifest_json["source_collection_id"],
        serde_json::json!(collection.collection_id)
    );
    assert_eq!(
        result.manifest_json["output"]["artifact_ref"],
        serde_json::json!(result.artifact_ref)
    );
    assert_eq!(
        result.manifest_json["output"]["content_hash"],
        serde_json::json!(result.content_hash)
    );
    assert_eq!(
        result.manifest_json["output"]["byte_len"],
        serde_json::json!(result.byte_len)
    );

    let items = result.manifest_json["items"]
        .as_array()
        .expect("manifest items are an array");
    assert_eq!(items.len(), 2);
    for item in items {
        let pack_path = item["pack_path"]
            .as_str()
            .expect("manifest item has pack path");
        assert!(
            !pack_path.contains(' '),
            "web portfolio pack paths use no-space naming"
        );
        assert!(
            item["artifact_ref"]
                .as_str()
                .expect("manifest item has artifact ref")
                .starts_with("artifact://"),
            "web portfolio items are ArtifactStore-backed"
        );
        assert!(
            item["content_hash"]
                .as_str()
                .expect("manifest item has checksum")
                .len()
                >= 32,
            "web portfolio manifest items carry checksums"
        );
    }

    let reloaded = store
        .get_web_portfolio_export_result(request.portfolio_export_id)
        .await
        .expect("get web portfolio result")
        .expect("web portfolio result exists");
    assert_eq!(reloaded.manifest_json, result.manifest_json);

    let bad_slug = store
        .request_web_portfolio_export(&NewWebPortfolioExportRequest {
            source_collection_id: collection.collection_id,
            slug: "bad slug".to_string(),
            title: "Bad Slug".to_string(),
            requested_by: "mt-048-exporter".to_string(),
        })
        .await
        .expect_err("web portfolio slugs reject blank spaces");
    assert!(bad_slug.to_string().contains("slug"));

    let bad_path = store
        .record_web_portfolio_export_result(
            request.portfolio_export_id,
            &manifest_artifact.artifact_ref,
            &manifest_artifact.content_hash,
            manifest_artifact.byte_len,
            &[WebPortfolioManifestItem {
                asset_id: hero.asset_id,
                artifact_ref: hero_artifact.artifact_ref.clone(),
                pack_path: "images/bad path.png".to_string(),
                content_hash: hero_artifact.content_hash.clone(),
                byte_len: hero_artifact.byte_len,
            }],
        )
        .await
        .expect_err("web portfolio pack paths reject blank spaces");
    assert!(bad_path.to_string().contains("pack_path"));

    let bad_ref = store
        .record_web_portfolio_export_result(
            request.portfolio_export_id,
            "D:\\Projects\\.GOV\\portfolio.json",
            &manifest_artifact.content_hash,
            manifest_artifact.byte_len,
            &[WebPortfolioManifestItem {
                asset_id: hero.asset_id,
                artifact_ref: hero_artifact.artifact_ref.clone(),
                pack_path: format!("images/{}-hero.png", hero.asset_id),
                content_hash: hero_artifact.content_hash.clone(),
                byte_len: hero_artifact.byte_len,
            }],
        )
        .await
        .expect_err("web portfolio result refs reject machine-local .GOV paths");
    assert!(bad_ref.to_string().contains("artifact_ref"));

    assert!(event_family::ALL.contains(&export_event_family::WEB_PORTFOLIO_EXPORT_REQUESTED));
    assert!(event_family::ALL.contains(&export_event_family::WEB_PORTFOLIO_EXPORT_RENDERED));
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::WEB_PORTFOLIO_EXPORT_REQUESTED,
                "atelier_web_portfolio_export_request",
                &request.portfolio_export_id.to_string(),
            )
            .await
            .expect("count web portfolio request event"),
        1
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::WEB_PORTFOLIO_EXPORT_RENDERED,
                "atelier_web_portfolio_export_request",
                &request.portfolio_export_id.to_string(),
            )
            .await
            .expect("count web portfolio rendered event"),
        1
    );
}

#[tokio::test]
async fn atelier_backup_manifest_records_versions_checksums_and_restore_preflight_refuses_newer() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_backup_manifest_records_versions_checksums_and_restore_preflight_refuses_newer: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let marker = format!("backup-manifest-{}", Uuid::new_v4());
    let artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-backup").as_bytes());

    let backup = store
        .record_backup_manifest(&NewBackupManifest {
            app_version: "1.2.3".to_string(),
            spec_version: "2026.06.08".to_string(),
            schema_version: 3,
            artifact_ref: artifact.artifact_ref.clone(),
            content_hash: artifact.content_hash.clone(),
            byte_len: artifact.byte_len,
            files: vec![
                BackupManifestFile {
                    logical_path: "manifest/atelier.json".to_string(),
                    content_hash: artifact.content_hash.clone(),
                    byte_len: artifact.byte_len,
                },
                BackupManifestFile {
                    logical_path: "checksums/media.json".to_string(),
                    content_hash: "0123456789abcdef0123456789abcdef".to_string(),
                    byte_len: 128,
                },
            ],
            created_by: "mt-049-backup".to_string(),
        })
        .await
        .expect("record backup manifest with version traceability");

    assert_eq!(
        backup.manifest_json["schema_id"],
        serde_json::json!(BACKUP_MANIFEST_SCHEMA_ID)
    );
    assert_eq!(
        backup.manifest_json["app_version"],
        serde_json::json!("1.2.3")
    );
    assert_eq!(
        backup.manifest_json["spec_version"],
        serde_json::json!("2026.06.08")
    );
    assert_eq!(backup.manifest_json["schema_version"], serde_json::json!(3));
    assert_eq!(
        backup.manifest_json["artifact"]["content_hash"],
        serde_json::json!(artifact.content_hash)
    );
    assert_eq!(
        backup.manifest_json["files"]
            .as_array()
            .expect("backup manifest files array")
            .len(),
        2
    );
    assert!(
        backup.manifest_hash.len() >= 32,
        "backup manifest row carries a manifest checksum"
    );

    let accepted = store
        .preflight_backup_restore(&BackupRestorePreflightRequest {
            backup_id: backup.backup_id,
            current_app_version: "1.2.3".to_string(),
            current_spec_version: "2026.06.08".to_string(),
            current_schema_version: 3,
            requested_by: "mt-049-restore".to_string(),
        })
        .await
        .expect("same-version backup restore preflight is accepted");
    assert_eq!(accepted.status, BackupRestorePreflightStatus::Accepted);
    assert!(accepted.refusal_reason.is_none());

    let newer_app_artifact = atelier_pg_support::write_native_media_artifact(
        format!("{marker}-newer-app-backup").as_bytes(),
    );
    let newer_app_backup = store
        .record_backup_manifest(&NewBackupManifest {
            app_version: "9.0.0".to_string(),
            spec_version: "2026.06.08".to_string(),
            schema_version: 3,
            artifact_ref: newer_app_artifact.artifact_ref.clone(),
            content_hash: newer_app_artifact.content_hash.clone(),
            byte_len: newer_app_artifact.byte_len,
            files: vec![BackupManifestFile {
                logical_path: "manifest/atelier.json".to_string(),
                content_hash: newer_app_artifact.content_hash.clone(),
                byte_len: newer_app_artifact.byte_len,
            }],
            created_by: "mt-049-backup".to_string(),
        })
        .await
        .expect("record newer-app backup manifest");
    let refused_app = store
        .preflight_backup_restore(&BackupRestorePreflightRequest {
            backup_id: newer_app_backup.backup_id,
            current_app_version: "1.2.3".to_string(),
            current_spec_version: "2026.06.08".to_string(),
            current_schema_version: 3,
            requested_by: "mt-049-restore".to_string(),
        })
        .await
        .expect("newer-app restore preflight returns a refusal record");
    assert_eq!(refused_app.status, BackupRestorePreflightStatus::Refused);
    assert!(
        refused_app
            .refusal_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("newer app")),
        "newer app backups are refused before restore"
    );

    let newer_schema_artifact = atelier_pg_support::write_native_media_artifact(
        format!("{marker}-newer-schema-backup").as_bytes(),
    );
    let newer_schema_backup = store
        .record_backup_manifest(&NewBackupManifest {
            app_version: "1.2.3".to_string(),
            spec_version: "2026.06.08".to_string(),
            schema_version: 4,
            artifact_ref: newer_schema_artifact.artifact_ref.clone(),
            content_hash: newer_schema_artifact.content_hash.clone(),
            byte_len: newer_schema_artifact.byte_len,
            files: vec![BackupManifestFile {
                logical_path: "manifest/atelier.json".to_string(),
                content_hash: newer_schema_artifact.content_hash.clone(),
                byte_len: newer_schema_artifact.byte_len,
            }],
            created_by: "mt-049-backup".to_string(),
        })
        .await
        .expect("record newer-schema backup manifest");
    let refused_schema = store
        .preflight_backup_restore(&BackupRestorePreflightRequest {
            backup_id: newer_schema_backup.backup_id,
            current_app_version: "1.2.3".to_string(),
            current_spec_version: "2026.06.08".to_string(),
            current_schema_version: 3,
            requested_by: "mt-049-restore".to_string(),
        })
        .await
        .expect("newer-schema restore preflight returns a refusal record");
    assert_eq!(refused_schema.status, BackupRestorePreflightStatus::Refused);
    assert!(
        refused_schema
            .refusal_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("newer schema")),
        "newer schema backups are refused before restore"
    );

    assert!(event_family::ALL.contains(&export_event_family::BACKUP_MANIFEST_RECORDED));
    assert!(event_family::ALL.contains(&export_event_family::BACKUP_RESTORE_PREFLIGHT_RECORDED));
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::BACKUP_MANIFEST_RECORDED,
                "atelier_backup_manifest",
                &backup.backup_id.to_string(),
            )
            .await
            .expect("count backup manifest event"),
        1
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::BACKUP_RESTORE_PREFLIGHT_RECORDED,
                "atelier_backup_manifest",
                &newer_app_backup.backup_id.to_string(),
            )
            .await
            .expect("count newer app preflight event"),
        1
    );
}

#[tokio::test]
async fn atelier_annotation_sequence_update_count_and_remove() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_annotation_sequence_update_count_and_remove: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    // --- materialize a media asset to annotate ---
    let asset_id = fresh_asset(&store).await;

    // --- add_media_annotation seq increments (1, 2) ---
    let ann1 = store
        .add_media_annotation(&NewMediaAnnotation {
            asset_id,
            kind: AnnotationKind::Point,
            label: Some("focus".to_string()),
            note: "left eye".to_string(),
            geometry: serde_json::json!({ "x": 0.25, "y": 0.40 }),
            author: "operator".to_string(),
        })
        .await
        .expect("add first annotation");
    assert_eq!(ann1.seq, 1, "first annotation is seq 1");

    let ann2 = store
        .add_media_annotation(&NewMediaAnnotation {
            asset_id,
            kind: AnnotationKind::Box,
            label: Some("wardrobe".to_string()),
            note: "jacket".to_string(),
            geometry: serde_json::json!({ "x": 0.1, "y": 0.1, "w": 0.3, "h": 0.4 }),
            author: "operator".to_string(),
        })
        .await
        .expect("add second annotation");
    assert_eq!(ann2.seq, 2, "second annotation is seq 2 (increments)");

    // --- list in paint/export order, get, update note ---
    let listed = store
        .list_media_annotations(asset_id)
        .await
        .expect("list annotations");
    assert_eq!(listed.len(), 2, "both annotations present in order");
    assert_eq!(listed[0].annotation_id, ann1.annotation_id);
    assert_eq!(listed[1].annotation_id, ann2.annotation_id);

    let fetched = store
        .get_media_annotation(ann1.annotation_id)
        .await
        .expect("get annotation");
    assert_eq!(fetched.note, "left eye");

    let updated = store
        .update_media_annotation_note(ann1.annotation_id, "right eye", Some("focus-2"))
        .await
        .expect("update annotation note");
    assert_eq!(updated.note, "right eye", "note is updated in place");
    assert_eq!(updated.label.as_deref(), Some("focus-2"), "label updated");
    assert_eq!(
        updated.geometry, fetched.geometry,
        "geometry is immutable on note update"
    );

    // --- count ---
    let count_before = store
        .count_media_annotations(asset_id)
        .await
        .expect("count annotations");
    assert_eq!(count_before, 2, "two annotations on the asset");

    // --- remove returns the asset id and decrements the count ---
    let removed_asset = store
        .remove_media_annotation(ann2.annotation_id)
        .await
        .expect("remove annotation");
    assert_eq!(
        removed_asset, asset_id,
        "remove returns the parent asset id"
    );
    let count_after = store
        .count_media_annotations(asset_id)
        .await
        .expect("count annotations after remove");
    assert_eq!(count_after, 1, "removal decrements the annotation count");
}

#[tokio::test]
async fn atelier_reset_modes_preserve_original_media_and_adopt_orphan_manifest() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_reset_modes_preserve_original_media_and_adopt_orphan_manifest: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let marker = format!("reset-orphan-{}", Uuid::new_v4());
    let artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-original").as_bytes());
    let asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some(format!("test-source:{marker}")),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize original media before reset");

    let preference_key = "view-defaults.asset-grid-density";
    store
        .set_preference(&SetPreference {
            scope: PreferenceScope::Global,
            key: preference_key.to_string(),
            value_type: PreferenceType::String,
            value: "dense".to_string(),
            redacted: false,
        })
        .await
        .expect("seed resettable preference");
    let preference_reset = store
        .record_atelier_reset(&AtelierResetRequest {
            mode: AtelierResetMode::PreferencesOnly,
            requested_by: "mt-050-reset".to_string(),
            reason: format!("{marker}-preferences-only"),
        })
        .await
        .expect("record preferences-only reset");
    assert_eq!(preference_reset.mode, AtelierResetMode::PreferencesOnly);
    assert!(
        preference_reset.preferences_deleted_count >= 1,
        "preferences-only reset must delete persisted preference rows"
    );
    assert_eq!(
        store
            .get_preference(PreferenceScope::Global, preference_key)
            .await
            .expect("read preference after reset"),
        None,
        "preferences-only reset removes persisted preference authority rows"
    );
    assert!(
        store
            .get_media_asset_by_hash(&asset.content_hash)
            .await
            .expect("read media after preferences reset")
            .is_some(),
        "preferences-only reset must not delete original media"
    );

    store
        .set_preference(&SetPreference {
            scope: PreferenceScope::Global,
            key: preference_key.to_string(),
            value_type: PreferenceType::String,
            value: "compact".to_string(),
            redacted: false,
        })
        .await
        .expect("seed preference before full reset");
    let full_reset = store
        .record_atelier_reset(&AtelierResetRequest {
            mode: AtelierResetMode::FullPreserveOriginalMedia,
            requested_by: "mt-050-reset".to_string(),
            reason: format!("{marker}-full-preserve-originals"),
        })
        .await
        .expect("record full reset preserving original media");
    assert_eq!(full_reset.mode, AtelierResetMode::FullPreserveOriginalMedia);
    assert!(
        full_reset.original_media_preserved_count >= 1,
        "full reset must preserve original media and record it in a manifest"
    );
    let manifest_id = full_reset
        .orphan_manifest_id
        .expect("full reset creates an orphan manifest");
    let manifest_items = store
        .list_orphan_manifest_items(manifest_id)
        .await
        .expect("list orphan manifest items");
    let original_item = manifest_items
        .iter()
        .find(|item| item.asset_id == asset.asset_id)
        .expect("manifest contains the pre-reset original asset");
    assert_eq!(original_item.content_hash, asset.content_hash);
    assert_eq!(original_item.artifact_ref, asset.artifact_ref);
    assert_portable_artifact_handle("orphan manifest artifact_ref", &original_item.artifact_ref);
    assert_eq!(
        original_item.adoption_status,
        OrphanAdoptionStatus::Orphaned
    );

    let adoption = store
        .adopt_orphan_manifest_item(&OrphanAdoptionRequest {
            manifest_item_id: original_item.manifest_item_id,
            requested_by: "mt-050-adopt".to_string(),
        })
        .await
        .expect("adopt orphaned original media into intake");
    assert_eq!(adoption.manifest_item.asset_id, asset.asset_id);
    assert_eq!(
        adoption.manifest_item.adoption_status,
        OrphanAdoptionStatus::Adopted
    );
    assert_eq!(
        adoption.item.content_hash.as_deref(),
        Some(asset.content_hash.as_str())
    );
    assert_eq!(adoption.item.byte_len, asset.byte_len);
    assert_eq!(
        adoption.item.source_path, asset.artifact_ref,
        "adoption rehydrates the preserved ArtifactStore handle as the intake source"
    );
    assert_portable_artifact_handle("adopted intake source_path", &adoption.item.source_path);
    assert!(
        store
            .get_media_asset_by_hash(&asset.content_hash)
            .await
            .expect("read media after adoption")
            .is_some(),
        "orphan adoption must not consume or delete the original media asset"
    );
}

#[test]
fn atelier_acceptance_constraints_reject_machine_local_roots_and_repo_dist_release() {
    let valid = AtelierAcceptanceConstraints {
        data_root_ref: "data-root:atelier-library".to_string(),
        artifact_root_ref: "artifact-store://.handshake/artifacts".to_string(),
        release_output_ref: "release://atelier/web-portfolio/v1".to_string(),
    };
    valid
        .validate()
        .expect("portable data root, relocatable artifact root, and release ref are accepted");

    for data_root_ref in [
        "C:\\Users\\operator\\atelier",
        "/tmp/atelier",
        "file:///tmp/atelier",
        "data-root:../atelier",
    ] {
        let err = AtelierAcceptanceConstraints {
            data_root_ref: data_root_ref.to_string(),
            ..valid.clone()
        }
        .validate()
        .expect_err("machine-local or traversal data roots are rejected");
        assert!(
            err.to_string().contains("data_root_ref"),
            "error should identify data_root_ref, got {err}"
        );
    }

    for artifact_root_ref in [
        "C:\\Users\\operator\\.handshake\\artifacts",
        "file:///tmp/.handshake/artifacts",
        "artifact://.GOV/artifacts/L1/00000000-0000-0000-0000-000000000000/payload",
    ] {
        let err = AtelierAcceptanceConstraints {
            artifact_root_ref: artifact_root_ref.to_string(),
            ..valid.clone()
        }
        .validate()
        .expect_err("machine-local or governance artifact roots are rejected");
        assert!(
            err.to_string().contains("artifact_root_ref"),
            "error should identify artifact_root_ref, got {err}"
        );
    }

    for release_output_ref in [
        "dist/atelier",
        "repo://dist/atelier",
        "release://atelier/dist/web",
        "release://atelier/../web",
    ] {
        let err = AtelierAcceptanceConstraints {
            release_output_ref: release_output_ref.to_string(),
            ..valid.clone()
        }
        .validate()
        .expect_err("repo dist or traversal release refs are rejected");
        assert!(
            err.to_string().contains("release_output_ref"),
            "error should identify release_output_ref, got {err}"
        );
    }
}

#[tokio::test]
async fn atelier_settings_upsert_scope_redaction_and_delete() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_settings_upsert_scope_redaction_and_delete: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    // --- set a global preference, then re-set the SAME key: UPDATE in place ---
    let key = format!("data-roots.library-root-{}", Uuid::new_v4());
    let pref1 = store
        .set_preference(&SetPreference {
            scope: PreferenceScope::Global,
            key: key.clone(),
            value_type: PreferenceType::Path,
            value: "data-root:library".to_string(),
            redacted: false,
        })
        .await
        .expect("set global preference");
    let pref2 = store
        .set_preference(&SetPreference {
            scope: PreferenceScope::Global,
            key: key.clone(),
            value_type: PreferenceType::Path,
            value: "data-root:library-v2".to_string(),
            redacted: false,
        })
        .await
        .expect("re-set same global key");
    assert_eq!(
        pref1.preference_id, pref2.preference_id,
        "re-setting the same (global, key) UPDATES in place (UNIQUE NULLS NOT DISTINCT)"
    );
    assert_eq!(
        pref2.value, "data-root:library-v2",
        "value updated in place"
    );

    // Confirm exactly one row for this global key (no duplicate from the upsert).
    let globals = store
        .list_preferences(PreferenceScope::Global, false)
        .await
        .expect("list global preferences");
    let matches = globals.iter().filter(|p| p.key == key).count();
    assert_eq!(matches, 1, "global key has exactly one row after re-set");

    // --- set a character-scoped preference with the same key text ---
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-settings-{}", Uuid::new_v4()),
            display_name: "Settings Subject".to_string(),
        })
        .await
        .expect("create character");
    let char_scope = PreferenceScope::Character(character.internal_id);
    let char_pref = store
        .set_preference(&SetPreference {
            scope: char_scope,
            key: key.clone(),
            value_type: PreferenceType::String,
            value: "char-specific".to_string(),
            redacted: false,
        })
        .await
        .expect("set character-scoped preference");
    assert_ne!(
        char_pref.preference_id, pref2.preference_id,
        "the same key in a different scope is a distinct preference row"
    );

    // --- get_preference resolves the right scope ---
    let got_global = store
        .get_preference(PreferenceScope::Global, &key)
        .await
        .expect("get global preference")
        .expect("global preference present");
    assert_eq!(got_global.value, "data-root:library-v2");
    let got_char = store
        .get_preference(char_scope, &key)
        .await
        .expect("get character preference")
        .expect("character preference present");
    assert_eq!(got_char.value, "char-specific");

    // --- defined defaults read as non-null effective preferences without a row ---
    let default_key = "view-defaults.asset-grid-density";
    let effective_default = store
        .get_effective_preference(char_scope, default_key)
        .await
        .expect("read effective default preference");
    assert!(
        effective_default.preference_id.is_none(),
        "unset defined preference should not create a row on read"
    );
    assert_eq!(effective_default.namespace, "view-defaults");
    assert_eq!(effective_default.name, "asset-grid-density");
    assert_eq!(effective_default.value, "comfortable");
    assert_eq!(effective_default.source, PreferenceValueSource::Default);
    assert_eq!(effective_default.revision, 0);

    // --- set/reset produce recoverable receipts with before/after revisions ---
    let set_receipt = store
        .set_preference_with_receipt(&SetPreference {
            scope: char_scope,
            key: default_key.to_string(),
            value_type: PreferenceType::String,
            value: "dense".to_string(),
            redacted: false,
        })
        .await
        .expect("set preference with receipt");
    assert_eq!(set_receipt.event_family, "atelier.preference.set");
    assert_eq!(set_receipt.revision_before, None);
    assert_eq!(set_receipt.revision_after, 1);
    assert_eq!(set_receipt.source_after, PreferenceValueSource::Operator);
    assert_eq!(set_receipt.value_after, "dense");
    assert_eq!(set_receipt.preference.namespace, "view-defaults");
    assert_eq!(set_receipt.preference.name, "asset-grid-density");
    assert_eq!(
        set_receipt.preference.default_value.as_deref(),
        Some("comfortable")
    );

    let reset_receipt = store
        .reset_preference_to_default(char_scope, default_key)
        .await
        .expect("reset preference to default");
    assert_eq!(
        reset_receipt.event_family,
        "atelier.preference.reset_to_default"
    );
    assert_eq!(
        reset_receipt.preference.preference_id, set_receipt.preference.preference_id,
        "reset preserves the same authority row instead of deleting provenance"
    );
    assert_eq!(reset_receipt.revision_before, Some(1));
    assert_eq!(reset_receipt.revision_after, 2);
    assert_eq!(reset_receipt.value_before.as_deref(), Some("dense"));
    assert_eq!(reset_receipt.value_after, "comfortable");
    assert_eq!(reset_receipt.source_after, PreferenceValueSource::Default);

    let effective_after_reset = store
        .get_effective_preference(char_scope, default_key)
        .await
        .expect("read effective reset preference");
    assert_eq!(
        effective_after_reset.preference_id,
        Some(set_receipt.preference.preference_id)
    );
    assert_eq!(effective_after_reset.value, "comfortable");
    assert_eq!(effective_after_reset.source, PreferenceValueSource::Default);
    assert_eq!(effective_after_reset.revision, 2);

    let projection = store
        .list_preference_projection(char_scope, true)
        .await
        .expect("list settings projection");
    assert!(
        projection
            .iter()
            .any(|p| p.key == "feature-toggles.atelier-diagnostics"
                && p.source == PreferenceValueSource::Default
                && p.value == "true"),
        "projection includes registry defaults even when unset"
    );

    // --- retention preferences bind to an explicit governed prune contract ---
    let default_retention = store
        .get_retention_policy_binding(char_scope)
        .await
        .expect("read default retention policy binding");
    assert_eq!(default_retention.policy, RetentionDefaultPolicy::Retain);
    assert_eq!(
        default_retention.value_source,
        PreferenceValueSource::Default
    );
    assert_eq!(default_retention.prune_after_days, None);
    assert!(
        !default_retention.prune_confirmation_required,
        "retain policy must not authorize pruning"
    );
    let retain_confirmation = store
        .confirm_retention_prune(char_scope, "core-data-test")
        .await;
    assert!(
        retain_confirmation.is_err(),
        "retain policy must reject prune confirmation"
    );

    let invalid_retention = store
        .set_preference(&SetPreference {
            scope: char_scope,
            key: "retention.default-policy".to_string(),
            value_type: PreferenceType::String,
            value: "delete-now".to_string(),
            redacted: false,
        })
        .await;
    assert!(
        invalid_retention.is_err(),
        "retention.default-policy must reject ungoverned pruning vocabulary"
    );

    let _retention_set = store
        .set_preference_with_receipt(&SetPreference {
            scope: char_scope,
            key: "retention.default-policy".to_string(),
            value_type: PreferenceType::String,
            value: "prune-after-30d".to_string(),
            redacted: false,
        })
        .await
        .expect("set governed retention policy");
    let prune_binding = store
        .get_retention_policy_binding(char_scope)
        .await
        .expect("read prune retention policy binding");
    assert_eq!(
        prune_binding.policy,
        RetentionDefaultPolicy::PruneAfter30Days
    );
    assert_eq!(prune_binding.prune_after_days, Some(30));
    assert!(
        prune_binding.prune_confirmation_required,
        "pruning policy must require explicit confirmation"
    );
    assert!(
        !prune_binding.automatic_prune_allowed,
        "settings binding must not silently authorize deletion"
    );
    let confirmation = store
        .confirm_retention_prune(char_scope, "core-data-test")
        .await
        .expect("confirm retention prune");
    assert_eq!(
        confirmation.event_family,
        "atelier.preference.retention_prune_confirmed"
    );
    assert_eq!(
        confirmation.binding.policy,
        RetentionDefaultPolicy::PruneAfter30Days
    );
    assert_eq!(confirmation.confirmed_by, "core-data-test");
    let confirmation_events = store
        .count_events_for_aggregate(
            "atelier.preference.retention_prune_confirmed",
            "atelier_preference_retention_policy",
            &confirmation.confirmation_id.to_string(),
        )
        .await
        .expect("count retention confirmation events");
    assert_eq!(
        confirmation_events, 1,
        "retention prune confirmation must be auditable before any prune path can act"
    );

    // --- invalid settings are rejected before storage ---
    for rejected_path in [
        "C:\\Users\\operator\\library",
        "\\\\server\\share\\library",
        "/home/operator/library",
        "~/library",
        "file:///tmp/library",
        "http://localhost:9000/library",
        "data-root:../library",
    ] {
        let local_path_err = store
            .set_preference(&SetPreference {
                scope: PreferenceScope::Global,
                key: format!("data-roots.local-path-{}", Uuid::new_v4()),
                value_type: PreferenceType::Path,
                value: rejected_path.to_string(),
                redacted: false,
            })
            .await;
        assert!(
            local_path_err.is_err(),
            "path preference {rejected_path:?} must be rejected as a machine-local or traversal path"
        );
    }

    let namespace_err = store
        .set_preference(&SetPreference {
            scope: PreferenceScope::Global,
            key: format!("misc.library-root-{}", Uuid::new_v4()),
            value_type: PreferenceType::String,
            value: "value".to_string(),
            redacted: false,
        })
        .await;
    assert!(
        namespace_err.is_err(),
        "preference ids must use an in-scope namespace"
    );

    let secret_key = format!("data-roots.api-token-{}", Uuid::new_v4());
    let secret_err = store
        .set_preference(&SetPreference {
            scope: PreferenceScope::Global,
            key: secret_key.clone(),
            value_type: PreferenceType::String,
            value: "super-secret-token".to_string(),
            redacted: true,
        })
        .await;
    assert!(
        secret_err.is_err(),
        "preference records must not store secrets or redaction-only secret placeholders"
    );
    let rejected_secret = store
        .get_preference(PreferenceScope::Global, &secret_key)
        .await
        .expect("get rejected secret preference");
    assert!(
        rejected_secret.is_none(),
        "rejected secret preference must not leave an authority row"
    );

    // --- delete returns true once, false on a second delete ---
    let deleted = store
        .delete_preference(PreferenceScope::Global, &key)
        .await
        .expect("delete global preference");
    assert!(deleted, "deleting an existing preference returns true");
    let deleted_again = store
        .delete_preference(PreferenceScope::Global, &key)
        .await
        .expect("delete missing preference");
    assert!(
        !deleted_again,
        "deleting a missing preference returns false"
    );
    let after_delete = store
        .get_preference(PreferenceScope::Global, &key)
        .await
        .expect("get after delete");
    assert!(after_delete.is_none(), "deleted preference is gone");
}

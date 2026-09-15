#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! WP-KERNEL-012 MT-150: coverage restored from the suite deleted by 4f92cc25.
//!
//! Recovers the coverage of the deleted `atelier_manual_source_rows_pg_tests.rs`,
//! `atelier_pose_comfy_manual_rows_pg_tests.rs`, and
//! `atelier_comfy_job_registration_event_families_pg_tests.rs` that the live
//! successor suites (`atelier_core_data_tests.rs`, `atelier_pose_tests.rs`,
//! `atelier_comfy_tests.rs`, `atelier_comfy_intake_routing_tests.rs`,
//! `atelier_comfy_job_tests.rs`) do NOT already re-prove: that every
//! `model_manual()` `CommandReference` row's documented `schema_fields` names a
//! REAL field of the runtime value the row documents, not manual prose. Each
//! test below performs only the minimum live operation needed to obtain that
//! runtime value (real store, real artifacts, real event ledger where
//! applicable) and re-checks it against the manual row via
//! `assert_schema_fields_are_real`, PLUS (restored under MT-150 V3-PRE-01,
//! because no successor suite asserts them) the originals' manual-parity
//! checks that the documented `expected_output` / `recovery_steps` /
//! `common_errors` prose names what the runtime really does: schema ids,
//! required pack files, and every documented refusal reproduced live.
//! `mt130_*` additionally restores the original per-aggregate EventLedger
//! counts and the `event_family::ALL` registration of all eight families.

mod atelier_surreal_support;

use chrono::Duration;
use handshake_core::atelier::comfy::{
    comfy_event_family, ComfyBridgeFakeAdapterV1, ComfyJobStatus,
    ComfyOutputRegistrationFailureStatus, ComfyWorkflowHistoryQuery, ComfyWorkflowStatus,
    MediaKind, NewComfyJobRequest, NewComfyOutputRegistrationFailure, NewComfyWorkflowReceipt,
    RoutingIntent, COMFY_WORKFLOW_RECEIPT_SCHEMA,
};
use handshake_core::atelier::event_family;
use handshake_core::atelier::exports::{
    build_llm_evidence_pack_manifest, BackupManifestFile, BackupRestorePreflightRequest,
    BackupRestorePreflightStatus, LlmEvidencePackFile, LlmEvidencePackFileKind,
    LlmEvidenceSourceAnchor, NewBackupManifest, BACKUP_MANIFEST_SCHEMA_ID,
    LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID,
};
use handshake_core::atelier::pose::{
    CalibrationMarkerColors, CalibrationMarkerVisibility, CalibrationState, CanvasSize,
    DetectorStatus, IdentityCropBox, IdentityCropLandmark, IdentityProfileKind,
    NewIdentityCropArtifact, NewIdentityProfile, NewPoseCalibration, NewPoseContextState,
    NewPoseRig, NewPoseSidecar, PoseContextKind, PoseRig, PoseSidecarKind, PoseSidecarStatus,
    BODY_KEYPOINT_COUNT, FACE_KEYPOINT_COUNT, HAND_KEYPOINT_COUNT,
};
use handshake_core::atelier::{AtelierStore, NewCharacter, UrlImageImportRequest};
use handshake_core::model_manual::{model_manual, CommandReference, CommandStatus};
use uuid::Uuid;

/// Shared isolated embedded-store preamble, matching the sibling `atelier_*`
/// suites (`atelier_pose_tests.rs`, `atelier_comfy_tests.rs`, ...).
async fn connected_store() -> (AtelierStore, atelier_surreal_support::AtelierSurrealHarness) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    (harness.atelier.clone(), harness)
}

/// Look up a manual command row by stable id; the manual is the runtime
/// surface Diagnostics serves to no-context models.
fn manual_command(id: &str) -> &'static CommandReference {
    model_manual()
        .command_reference
        .iter()
        .find(|command| command.id == id)
        .unwrap_or_else(|| panic!("model manual must document command {id}"))
}

/// THE guarantee every restored item in this file adds: every documented
/// schema field names a real field of the runtime value (top level, or one
/// level down inside one of the named nested documents). Never satisfiable by
/// manual text alone -- `value`/`nested` must come from a live call.
fn assert_schema_fields_are_real(
    row: &CommandReference,
    value: &serde_json::Value,
    nested: &[&serde_json::Value],
) {
    for field in row.schema_fields {
        let found =
            value.get(field).is_some() || nested.iter().any(|doc| doc.get(field).is_some());
        assert!(
            found,
            "manual row {} documents schema field {field} that the runtime value does not carry",
            row.id
        );
    }
}

/// Materialize a fresh, run-unique character (pose rows FK to atelier_character).
async fn fresh_character(store: &AtelierStore) -> Uuid {
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-manualrows-{}", Uuid::new_v4()),
            display_name: "Manual Rows Subject".to_string(),
        })
        .await
        .expect("create character");
    character.internal_id
}

/// A valid OpenPose keypoint payload: body-18 plus zero-filled face/hands.
fn valid_keypoints() -> serde_json::Value {
    serde_json::json!({
        "people": [{
            "pose_keypoints_2d": vec![0.0_f64; BODY_KEYPOINT_COUNT * 3],
            "face_keypoints_2d": vec![0.0_f64; FACE_KEYPOINT_COUNT * 3],
            "hand_left_keypoints_2d": vec![0.0_f64; HAND_KEYPOINT_COUNT * 3],
            "hand_right_keypoints_2d": vec![0.0_f64; HAND_KEYPOINT_COUNT * 3],
        }]
    })
}

fn artifact_manifest_ref(artifact_ref: &str) -> String {
    artifact_ref.replace("/payload", "/artifact.json")
}

/// The manual row's `common_errors` prose documents the refusal marker the runtime enforces.
fn failure_marker_documented(row: &CommandReference, marker: &str) -> bool {
    row.common_errors.iter().any(|error| error.contains(marker))
}

/// Ingest a fresh, run-unique rig for a character and return it.
async fn fresh_rig(store: &AtelierStore, character_internal_id: Uuid) -> PoseRig {
    store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id,
            source_asset_id: None,
            source_ref: format!("portrait://{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 1024,
                height: 1536,
            },
            detector_provider: "mediapipe.tasks-vision.pose".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: None,
            source_asset_path_ref: None,
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: Some(format!("artifact://atelier/pose/{}", Uuid::new_v4())),
        })
        .await
        .expect("ingest pose rig")
}

/// MT-074: the LLM evidence-pack export manual row documents the real
/// `exports::build_llm_evidence_pack_manifest` schema. Already covered live
/// (persistence + EventLedger) by
/// `atelier_core_data_tests.rs::atelier_llm_evidence_pack_contract_is_strict_deterministic_and_redaction_aware`;
/// this test adds only the manual-vs-runtime schema_fields cross-check, so it
/// stays a pure-function proof (no store) like that sibling test.
#[test]
fn mt074_llm_evidence_pack_manual_row_documents_live_export_surface() {
    let row = manual_command("atelier_build_llm_evidence_pack_manifest");
    assert_eq!(
        row.name, "exports::build_llm_evidence_pack_manifest",
        "manual row must name the real export surface"
    );

    let marker = format!("mt074-manual-{}", Uuid::new_v4().simple());
    let pack_kinds = [
        (LlmEvidencePackFileKind::Readme, "README.md"),
        (LlmEvidencePackFileKind::Evidence, "evidence.json"),
        (LlmEvidencePackFileKind::RedactionReport, "redactions.json"),
        (LlmEvidencePackFileKind::SourceIndex, "source-index.json"),
    ];
    let files: Vec<LlmEvidencePackFile> = pack_kinds
        .into_iter()
        .map(|(kind, pack_path)| {
            let artifact = atelier_surreal_support::write_native_media_artifact(
                format!("{marker}-{pack_path}").as_bytes(),
            );
            LlmEvidencePackFile {
                kind,
                pack_path: pack_path.to_string(),
                artifact_ref: artifact.artifact_ref.clone(),
                content_hash: artifact.content_hash.clone(),
                byte_len: artifact.byte_len,
                source_anchors: vec![LlmEvidenceSourceAnchor {
                    source_id: format!("{marker}-{}", kind.as_token()),
                    source_path: format!("source-index/{marker}.json"),
                    source_range: "lines:1-10".to_string(),
                    content_hash: artifact.content_hash.clone(),
                }],
                redaction_required: false,
                redacted: false,
            }
        })
        .collect();

    let manifest = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        format!("{marker}-requested-by"),
        files.clone(),
    )
    .expect("build strict evidence-pack manifest");

    // The manual's expected_output must quote the schema id the runtime
    // manifest actually carries (not a hand-written approximation).
    assert_eq!(manifest.schema_id, LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID);
    assert!(
        row.expected_output.contains(&manifest.schema_id),
        "manual expected_output must document the runtime schema id {}",
        manifest.schema_id
    );

    // The manual's recovery guidance must name every file the runtime
    // manifest requires, in the runtime's deterministic order.
    let runtime_paths: Vec<&str> = manifest
        .files
        .iter()
        .map(|file| file.pack_path.as_str())
        .collect();
    assert_eq!(
        runtime_paths,
        vec![
            "README.md",
            "evidence.json",
            "redactions.json",
            "source-index.json"
        ],
        "manifest files are in deterministic model-consumable order"
    );
    for pack_path in &runtime_paths {
        assert!(
            row.recovery_steps
                .iter()
                .any(|step| step.contains(pack_path)),
            "manual recovery steps must name required pack file {pack_path}"
        );
    }
    for file in &manifest.files {
        assert!(
            file.artifact_ref.starts_with("artifact://"),
            "evidence-pack files are ArtifactStore-backed, got {}",
            file.artifact_ref
        );
    }

    let manifest_value = serde_json::to_value(&manifest).expect("serialize runtime manifest");
    let first_file = manifest_value["files"][0].clone();
    assert_schema_fields_are_real(row, &manifest_value, &[&first_file]);

    // Every documented common error is a real strict-validation refusal.
    let missing_required = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        format!("{marker}-requested-by"),
        files[..3].to_vec(),
    )
    .expect_err("dropping source-index.json must be refused");
    assert!(
        missing_required
            .to_string()
            .contains("missing required file"),
        "missing-file refusal must be explicit: {missing_required}"
    );
    let mut duplicated = files.clone();
    duplicated.push(files[0].clone());
    let duplicate_err = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        format!("{marker}-requested-by"),
        duplicated,
    )
    .expect_err("duplicate file kind/pack_path must be refused");
    assert!(
        duplicate_err.to_string().contains("duplicate"),
        "duplicate refusal must be explicit: {duplicate_err}"
    );
}

/// MT-075: the backup-manifest and restore-preflight manual rows document the
/// real live-store surfaces. Persistence, checksum, and version-refusal
/// behavior is already covered live by
/// `atelier_core_data_tests.rs::atelier_backup_manifest_records_versions_checksums_and_restore_preflight_refuses_newer`;
/// this test adds only the manual-vs-runtime schema_fields cross-check.
#[tokio::test]
async fn mt075_backup_manual_rows_document_live_backup_and_preflight_surface() {
    let (store, _harness) = connected_store().await;
    let backup_row = manual_command("atelier_record_backup_manifest");
    let preflight_row = manual_command("atelier_backup_restore_preflight");
    assert_eq!(
        backup_row.name, "AtelierStore::record_backup_manifest",
        "backup manual row must name the real store method"
    );
    assert_eq!(
        preflight_row.name, "AtelierStore::preflight_backup_restore",
        "preflight manual row must name the real store method"
    );

    let marker = format!("mt075-manual-{}", Uuid::new_v4().simple());
    let artifact =
        atelier_surreal_support::write_native_media_artifact(format!("{marker}-backup").as_bytes());
    let backup = store
        .record_backup_manifest(&NewBackupManifest {
            app_version: "1.2.3".to_string(),
            spec_version: "2026.06.10".to_string(),
            schema_version: 3,
            artifact_ref: artifact.artifact_ref.clone(),
            content_hash: artifact.content_hash.clone(),
            byte_len: artifact.byte_len,
            files: vec![BackupManifestFile {
                logical_path: "manifest/atelier.json".to_string(),
                content_hash: artifact.content_hash.clone(),
                byte_len: artifact.byte_len,
            }],
            created_by: format!("{marker}-backup-author"),
        })
        .await
        .expect("record backup manifest through the real store");

    // RE-READ from the store; the schema check must run against the re-read
    // row, never the in-memory return value.
    let reread = store
        .get_backup_manifest(backup.backup_id)
        .await
        .expect("re-read backup manifest");
    assert_eq!(reread.backup_id, backup.backup_id);
    assert_eq!(reread.manifest_hash, backup.manifest_hash);
    assert_eq!(
        reread.manifest_json["schema_id"],
        serde_json::json!(BACKUP_MANIFEST_SCHEMA_ID)
    );
    assert!(
        backup_row.expected_output.contains(BACKUP_MANIFEST_SCHEMA_ID),
        "backup manual expected_output must document runtime schema id {BACKUP_MANIFEST_SCHEMA_ID}"
    );
    let backup_value = serde_json::to_value(&reread).expect("serialize re-read backup manifest");
    let manifest_json = reread.manifest_json.clone();
    assert_schema_fields_are_real(backup_row, &backup_value, &[&manifest_json]);

    let accepted = store
        .preflight_backup_restore(&BackupRestorePreflightRequest {
            backup_id: reread.backup_id,
            current_app_version: "1.2.3".to_string(),
            current_spec_version: "2026.06.10".to_string(),
            current_schema_version: 3,
            requested_by: format!("{marker}-restore"),
        })
        .await
        .expect("same-version restore preflight is accepted");
    assert_eq!(accepted.status, BackupRestorePreflightStatus::Accepted);
    assert!(accepted.refusal_reason.is_none());
    let preflight_value = serde_json::to_value(&accepted).expect("serialize runtime preflight record");
    assert_schema_fields_are_real(preflight_row, &preflight_value, &[]);

    // A newer-schema backup is refused with the typed reason the manual's
    // common_errors row documents.
    let newer_artifact = atelier_surreal_support::write_native_media_artifact(
        format!("{marker}-newer-schema-backup").as_bytes(),
    );
    let newer_backup = store
        .record_backup_manifest(&NewBackupManifest {
            app_version: "1.2.3".to_string(),
            spec_version: "2026.06.10".to_string(),
            schema_version: 4,
            artifact_ref: newer_artifact.artifact_ref.clone(),
            content_hash: newer_artifact.content_hash.clone(),
            byte_len: newer_artifact.byte_len,
            files: vec![BackupManifestFile {
                logical_path: "manifest/atelier.json".to_string(),
                content_hash: newer_artifact.content_hash.clone(),
                byte_len: newer_artifact.byte_len,
            }],
            created_by: format!("{marker}-backup-author"),
        })
        .await
        .expect("record newer-schema backup manifest");
    let refused = store
        .preflight_backup_restore(&BackupRestorePreflightRequest {
            backup_id: newer_backup.backup_id,
            current_app_version: "1.2.3".to_string(),
            current_spec_version: "2026.06.10".to_string(),
            current_schema_version: 3,
            requested_by: format!("{marker}-restore"),
        })
        .await
        .expect("newer-schema restore preflight returns a refusal record");
    assert_eq!(refused.status, BackupRestorePreflightStatus::Refused);
    assert!(
        refused
            .refusal_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("newer schema")),
        "newer-schema backups are refused with a typed reason"
    );
    assert!(
        failure_marker_documented(preflight_row, "newer"),
        "preflight manual row must document the newer-version refusal the runtime enforces"
    );
    let refused_value = serde_json::to_value(&refused).expect("serialize refused preflight record");
    assert_schema_fields_are_real(preflight_row, &refused_value, &[]);
}

/// MT-122: the pose context / rig / calibration manual rows document the real
/// live-store surfaces. CRUD, refusal, and EventLedger behavior is already
/// covered live by `atelier_pose_tests.rs` (~1029-1256 for pose context,
/// neighboring rig/calibration coverage in the same file); this test adds
/// only the manual-vs-runtime schema_fields cross-check.
#[tokio::test]
async fn mt122_pose_context_and_rig_manual_rows_document_live_surface() {
    let (store, _harness) = connected_store().await;
    let context_row = manual_command("atelier_set_pose_context_state");
    let rig_row = manual_command("atelier_ingest_pose_rig");
    let calibration_row = manual_command("atelier_set_pose_calibration");
    assert_eq!(context_row.name, "AtelierStore::set_pose_context_state");
    assert_eq!(rig_row.name, "AtelierStore::ingest_pose_rig");
    assert_eq!(calibration_row.name, "AtelierStore::set_pose_calibration");

    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;

    let fetched_rig = store
        .get_pose_rig(rig.rig_id)
        .await
        .expect("re-read pose rig");
    let rig_value = serde_json::to_value(&fetched_rig).expect("serialize re-read rig");
    assert_schema_fields_are_real(rig_row, &rig_value, &[]);

    let workspace_ref = format!("pose-workspace://{}", Uuid::new_v4());
    store
        .set_pose_context_state(&NewPoseContextState {
            workspace_ref: workspace_ref.clone(),
            kind: PoseContextKind::CharacterLinked,
            source_asset_id: None,
            character_internal_id: Some(character),
            collection_id: None,
            selected_rig_id: Some(rig.rig_id),
            requested_by: "mt-122-manual-proof".to_string(),
        })
        .await
        .expect("append pose context state");
    let head = store
        .current_pose_context_state(&workspace_ref)
        .await
        .expect("re-read pose context head")
        .expect("pose context head present");
    let context_value = serde_json::to_value(&head).expect("serialize re-read context state");
    assert_schema_fields_are_real(context_row, &context_value, &[]);

    // The documented common error is a real runtime refusal, not manual prose.
    let bad_context = store
        .set_pose_context_state(&NewPoseContextState {
            workspace_ref: workspace_ref.clone(),
            kind: PoseContextKind::SingleImage,
            source_asset_id: None,
            character_internal_id: None,
            collection_id: None,
            selected_rig_id: None,
            requested_by: "mt-122-manual-proof".to_string(),
        })
        .await
        .expect_err("single_image context without source_asset_id must be refused");
    assert!(
        bad_context.to_string().contains("source_asset_id"),
        "refusal must name the missing link: {bad_context}"
    );
    assert!(
        failure_marker_documented(context_row, "source_asset_id"),
        "manual row must document the single_image/source_asset_id refusal"
    );

    // Calibration: BLOCKED-by-default is preserved, never faked, and the missing
    // block_reason refusal the manual documents is real.
    let missing_reason = store
        .set_pose_calibration(&NewPoseCalibration {
            rig_id: rig.rig_id,
            state: CalibrationState::Unresolved,
            block_reason: None,
            head_pose_ref: None,
            marker_visibility: CalibrationMarkerVisibility::default(),
            marker_colors: CalibrationMarkerColors::default(),
            hand_rows: Vec::new(),
            history_refs: Vec::new(),
        })
        .await
        .expect_err("unresolved calibration without block_reason must be refused");
    assert!(
        missing_reason.to_string().contains("block_reason"),
        "calibration refusal must name block_reason: {missing_reason}"
    );
    assert!(
        failure_marker_documented(calibration_row, "block_reason"),
        "manual row must document the missing-block_reason refusal"
    );

    store
        .set_pose_calibration(&NewPoseCalibration {
            rig_id: rig.rig_id,
            state: CalibrationState::Unresolved,
            block_reason: Some("Calibration Panel 10.10.4.1.9 not yet implementable".to_string()),
            head_pose_ref: None,
            marker_visibility: CalibrationMarkerVisibility::default(),
            marker_colors: CalibrationMarkerColors::default(),
            hand_rows: Vec::new(),
            history_refs: Vec::new(),
        })
        .await
        .expect("preserve BLOCKED calibration with a reason");
    let reread_calibration = store
        .get_calibration(rig.rig_id)
        .await
        .expect("re-read calibration")
        .expect("calibration present");
    let calibration_value =
        serde_json::to_value(&reread_calibration).expect("serialize re-read calibration");
    assert_schema_fields_are_real(calibration_row, &calibration_value, &[]);
}

/// MT-123 (reduced-confidence item; ported after a full assertion-by-assertion
/// diff of the recovered `atelier_pose_comfy_manual_rows_pg_tests.rs` body
/// against the current live `NewPoseSidecar` / `NewIdentityProfile` /
/// `UpdateIdentityProfile` / `NewIdentityCropArtifact` construction sites in
/// `atelier_pose_tests.rs`, which match field-for-field): the sidecar /
/// hidden-gallery / identity-lineage manual rows document the real live-store
/// surfaces. CRUD, refusal, and EventLedger behavior is already covered live
/// by `atelier_pose_tests.rs` (~1964 onward for identity, neighboring sidecar
/// coverage in the same file); this test adds only the manual-vs-runtime
/// schema_fields cross-check.
#[tokio::test]
async fn mt123_pose_sidecar_and_identity_manual_rows_document_live_surface() {
    let (store, _harness) = connected_store().await;
    let sidecar_row = manual_command("atelier_record_pose_sidecar");
    let projection_row = manual_command("atelier_pose_sidecar_gallery_projection");
    let profile_row = manual_command("atelier_append_identity_profile");
    let crop_row = manual_command("atelier_record_identity_crop_artifact");
    assert_eq!(sidecar_row.name, "AtelierStore::record_pose_sidecar");
    assert_eq!(profile_row.name, "AtelierStore::append_identity_profile");

    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;

    let json_artifact = atelier_surreal_support::write_native_media_artifact(b"mt-123-openpose-json");
    let sidecar = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::OpenPoseJson,
            artifact_ref: json_artifact.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&json_artifact.artifact_ref),
            content_hash: json_artifact.content_hash.clone(),
            byte_len: json_artifact.byte_len,
            mime: "application/json".to_string(),
            width: rig.canvas.width,
            height: rig.canvas.height,
            status: PoseSidecarStatus::Rendered,
            error_message: None,
        })
        .await
        .expect("record OpenPose JSON sidecar");
    let listed = store
        .list_pose_sidecars(rig.rig_id)
        .await
        .expect("re-read sidecars");
    let reread_sidecar = listed
        .iter()
        .find(|s| s.sidecar_id == sidecar.sidecar_id)
        .expect("recorded sidecar present in list");
    let sidecar_value = serde_json::to_value(reread_sidecar).expect("serialize re-read sidecar");
    assert_schema_fields_are_real(sidecar_row, &sidecar_value, &[]);

    // The documented mime/kind refusal is real.
    let mismatched = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::OpenPoseJson,
            artifact_ref: json_artifact.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&json_artifact.artifact_ref),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            byte_len: json_artifact.byte_len,
            mime: "image/png".to_string(),
            width: rig.canvas.width,
            height: rig.canvas.height,
            status: PoseSidecarStatus::Rendered,
            error_message: None,
        })
        .await
        .expect_err("openpose_json sidecar with png mime must be refused");
    assert!(
        failure_marker_documented(sidecar_row, "mime"),
        "manual row must document the mime/kind refusal: {mismatched}"
    );

    let projection = store
        .pose_sidecar_gallery_projection(rig.rig_id)
        .await
        .expect("project sidecars hidden from gallery");
    let projected = projection
        .iter()
        .find(|p| p.sidecar_id == sidecar.sidecar_id)
        .expect("recorded sidecar appears in the projection");
    let projection_value =
        serde_json::to_value(projected).expect("serialize gallery projection row");
    assert_schema_fields_are_real(projection_row, &projection_value, &[]);

    let profile = store
        .append_identity_profile(&NewIdentityProfile {
            character_internal_id: character,
            kind: IdentityProfileKind::Face,
            name: "MT-123 face identity".to_string(),
            description: "Manual-row runtime proof identity".to_string(),
            reference_asset_id: None,
            reference_ref: format!("portrait://{}", Uuid::new_v4()),
            source_ref: Some(format!("source://identity/{}", Uuid::new_v4())),
            crop_ref: None,
            artifact_ref: None,
            provenance: "source: operator upload".to_string(),
        })
        .await
        .expect("append identity profile");
    let head = store
        .latest_identity_profile(character, IdentityProfileKind::Face)
        .await
        .expect("re-read identity head")
        .expect("identity head present");
    let profile_value = serde_json::to_value(&head).expect("serialize re-read identity profile");
    assert_schema_fields_are_real(profile_row, &profile_value, &[]);

    let crop_payload = atelier_surreal_support::write_native_media_artifact(b"mt-123-crop-512");
    let crop = store
        .record_identity_crop_artifact(&NewIdentityCropArtifact {
            profile_id: profile.profile_id,
            source_ref: format!("source://identity-crop/{}", Uuid::new_v4()),
            crop_box: IdentityCropBox {
                x: 100,
                y: 80,
                width: 512,
                height: 512,
            },
            landmarks: vec![IdentityCropLandmark {
                name: "left_eye".to_string(),
                x: 210.5,
                y: 224.25,
                confidence: Some(0.98),
            }],
            artifact_ref: crop_payload.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&crop_payload.artifact_ref),
            content_hash: crop_payload.content_hash.clone(),
            byte_len: crop_payload.byte_len,
            mime: "image/png".to_string(),
            width: 512,
            height: 512,
            created_by: "mt-123-manual-proof".to_string(),
        })
        .await
        .expect("record 512x512 identity crop artifact");
    let crops = store
        .list_identity_crop_artifacts(profile.profile_id)
        .await
        .expect("re-read identity crop artifacts");
    let reread_crop = crops
        .iter()
        .find(|c| c.crop_id == crop.crop_id)
        .expect("crop artifact persisted");
    let crop_value = serde_json::to_value(reread_crop).expect("serialize re-read crop artifact");
    assert_schema_fields_are_real(crop_row, &crop_value, &[]);

    // The documented 512x512 refusal is real.
    let bad_crop = store
        .record_identity_crop_artifact(&NewIdentityCropArtifact {
            profile_id: profile.profile_id,
            source_ref: format!("source://identity-crop/{}", Uuid::new_v4()),
            crop_box: IdentityCropBox {
                x: 0,
                y: 0,
                width: 256,
                height: 256,
            },
            // Valid landmarks so the earlier landmarks-not-empty validation
            // cannot fire first: this probe must reach the 512x512 contract.
            landmarks: vec![IdentityCropLandmark {
                name: "left_eye".to_string(),
                x: 105.5,
                y: 112.25,
                confidence: Some(0.97),
            }],
            artifact_ref: crop_payload.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&crop_payload.artifact_ref),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            byte_len: crop_payload.byte_len,
            mime: "image/png".to_string(),
            width: 256,
            height: 256,
            created_by: "mt-123-manual-proof".to_string(),
        })
        .await
        .expect_err("non-512x512 crop must be refused");
    assert!(
        bad_crop.to_string().contains("512"),
        "crop refusal must name the 512x512 contract: {bad_crop}"
    );
    assert!(
        failure_marker_documented(crop_row, "512"),
        "manual row must document the 512x512 refusal"
    );
}

/// MT-124: the workflow-receipt / history / intake-receipt / registration
/// failure-recovery manual rows document the real live-store surfaces.
/// Schema, refusal, history-replay, and EventLedger behavior is already
/// covered live by
/// `atelier_comfy_tests.rs::atelier_comfy_workflow_receipt_schema_preserves_outputs_status_and_evidence`
/// (~873) and the neighboring output-registration-failure/retry test (~700);
/// this test adds only the manual-vs-runtime schema_fields cross-check.
#[tokio::test]
async fn mt124_comfy_workflow_receipt_manual_rows_document_live_surface() {
    let (store, _harness) = connected_store().await;
    let receipt_row = manual_command("atelier_record_comfy_workflow_receipt");
    let history_row = manual_command("atelier_list_comfy_workflow_history");
    let intake_receipt_row = manual_command("atelier_produce_intake_receipt");
    let fallback_row = manual_command("atelier_mark_saveimage_fallback");
    let failure_row = manual_command("atelier_record_comfy_output_registration_failure");
    let retry_row = manual_command("atelier_retry_comfy_output_registration_failure");
    assert_eq!(receipt_row.name, "AtelierStore::record_comfy_workflow_receipt");
    assert_eq!(
        retry_row.name,
        "AtelierStore::retry_comfy_output_registration_failure"
    );

    let run_id = Uuid::new_v4();
    let spec_ref = format!("workflow-spec://{}", Uuid::new_v4());
    store
        .record_comfy_workflow_receipt(&NewComfyWorkflowReceipt {
            system_id: "comfyui".to_string(),
            workflow_run_id: run_id,
            workflow_spec_ref: spec_ref.clone(),
            workflow_json_ref: format!("artifact://atelier/workflow-json/{}", Uuid::new_v4()),
            prompt_ref: format!("prompt://{}", Uuid::new_v4()),
            status: ComfyWorkflowStatus::Succeeded,
            error_ref: None,
            evidence: serde_json::json!({ "executor": "mt-124-manual-proof" }),
        })
        .await
        .expect("record durable workflow receipt");
    let reread_receipt = store
        .get_comfy_workflow_receipt(run_id)
        .await
        .expect("re-read workflow receipt")
        .expect("workflow receipt present");
    assert_eq!(
        reread_receipt.receipt_json["schema"],
        serde_json::json!(COMFY_WORKFLOW_RECEIPT_SCHEMA)
    );
    assert!(
        receipt_row.description.contains(COMFY_WORKFLOW_RECEIPT_SCHEMA),
        "manual row must name the runtime receipt schema {COMFY_WORKFLOW_RECEIPT_SCHEMA}"
    );
    let receipt_value =
        serde_json::to_value(&reread_receipt).expect("serialize re-read workflow receipt");
    assert_schema_fields_are_real(receipt_row, &receipt_value, &[]);

    // The documented failed-without-error_ref refusal is real.
    let missing_error_ref = store
        .record_comfy_workflow_receipt(&NewComfyWorkflowReceipt {
            system_id: "comfyui".to_string(),
            workflow_run_id: Uuid::new_v4(),
            workflow_spec_ref: format!("workflow-spec://{}", Uuid::new_v4()),
            workflow_json_ref: format!("artifact://atelier/workflow-json/{}", Uuid::new_v4()),
            prompt_ref: format!("prompt://{}", Uuid::new_v4()),
            status: ComfyWorkflowStatus::Failed,
            error_ref: None,
            evidence: serde_json::json!({ "executor": "mt-124-manual-proof" }),
        })
        .await
        .expect_err("failed receipt without error_ref must be refused");
    assert!(
        failure_marker_documented(receipt_row, "error_ref"),
        "manual row must document the failed/error_ref refusal: {missing_error_ref}"
    );

    let history = store
        .list_comfy_workflow_history(&ComfyWorkflowHistoryQuery {
            character_ref: None,
            workflow_spec_ref: Some(spec_ref.clone()),
            status: None,
            from_utc: Some(reread_receipt.created_at_utc - Duration::seconds(1)),
            to_utc: Some(reread_receipt.created_at_utc + Duration::seconds(1)),
        })
        .await
        .expect("replay workflow history");
    let history_row_value = history
        .iter()
        .find(|r| r.workflow_run_id == run_id)
        .expect("history replay must include the persisted receipt");
    let history_value = serde_json::to_value(history_row_value).expect("serialize history row");
    assert_schema_fields_are_real(history_row, &history_value, &[]);

    // SaveImage fallback: the empty-reason refusal the manual documents is real.
    let empty_reason = store.mark_saveimage_fallback(run_id, "  ").await;
    assert!(
        empty_reason.is_err(),
        "empty fallback_reason must be refused as the manual documents"
    );
    assert!(
        failure_marker_documented(fallback_row, "fallback_reason"),
        "manual row must document the empty-fallback_reason refusal"
    );
    store
        .mark_saveimage_fallback(run_id, "no bridge node in graph")
        .await
        .expect("mark SaveImage fallback engaged");
    let intake_receipt = store
        .produce_intake_receipt(run_id)
        .await
        .expect("produce per-run intake receipt");
    let intake_receipt_value =
        serde_json::to_value(&intake_receipt).expect("serialize intake receipt");
    assert_schema_fields_are_real(intake_receipt_row, &intake_receipt_value, &[]);

    let artifact_ref = format!("artifact://atelier/comfy/{}", Uuid::new_v4());
    let failure = store
        .record_comfy_output_registration_failure(&NewComfyOutputRegistrationFailure {
            workflow_run_id: run_id,
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            attempted_registration_id: None,
            source_node_instance_id: "saveimage-late-register".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: artifact_ref.clone(),
            artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: None,
            graph_hash: None,
            seed: Some(124),
            identity_metadata: None,
            failure_stage: "registration".to_string(),
            failure_reason: "capability registration unavailable after image save".to_string(),
            evidence: serde_json::json!({ "case": "mt-124-manual-proof" }),
        })
        .await
        .expect("preserve saved output whose registration failed");
    let failure_value = serde_json::to_value(&failure).expect("serialize failure row");
    assert_schema_fields_are_real(failure_row, &failure_value, &[]);

    let adapter = ComfyBridgeFakeAdapterV1::default();
    let registration = store
        .register_bridge_capability(&adapter.capability_registration(
            run_id,
            ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
            &format!("artifact://atelier/capability-evidence/{}", Uuid::new_v4()),
        ))
        .await
        .expect("register capability before retry");
    let retry = store
        .retry_comfy_output_registration_failure(
            failure.failure_id,
            Some(registration.registration_id),
        )
        .await
        .expect("recover the preserved output");
    let resolved = store
        .get_comfy_output_registration_failure(failure.failure_id)
        .await
        .expect("re-read failure row")
        .expect("failure row still queryable");
    assert_eq!(
        resolved.status,
        ComfyOutputRegistrationFailureStatus::Registered
    );
    assert_eq!(resolved.retry_count, 1);
    let retry_value = serde_json::to_value(&resolved).expect("serialize resolved failure row");
    let recovered_output_value =
        serde_json::to_value(&retry.output).expect("serialize recovered intake output");
    // `registration_id` lives on the recovered `IntakeOutput`, not on the
    // `ComfyOutputRegistrationFailure` row itself -- the manual row documents
    // the combined shape, so the recovered-output document is passed as the
    // nested lookup source (matching the recovered original's structure).
    assert_schema_fields_are_real(retry_row, &retry_value, &[&recovered_output_value]);

    // Retrying a non-retryable failure is refused, as documented.
    let second_retry = store
        .retry_comfy_output_registration_failure(
            failure.failure_id,
            Some(registration.registration_id),
        )
        .await;
    assert!(second_retry.is_err(), "registered failure is no longer retryable");
    assert!(
        failure_marker_documented(retry_row, "not retryable"),
        "manual row must document the not-retryable refusal"
    );
}

/// MT-125 (rescoped around the real symbols the manual rows document --
/// `register_bridge_capability` / `list_capability_rejects`,
/// `record_url_image_import`, `set_calibration` / `get_calibration` -- a
/// prior lane mischaracterised these): the deferred-boundary manual rows
/// document the real live-store surfaces. Capability-gate routing, refusal,
/// idempotent replay, and EventLedger behavior for these exact symbols is
/// already covered live by `atelier_comfy_intake_routing_tests.rs` (MT-120,
/// which also drives `register_bridge_capability`) and
/// `atelier_comfy_tests.rs`'s registration-failure/retry coverage; this test
/// adds only the manual-vs-runtime schema_fields cross-check.
///
/// `atelier_register_bridge_capability`'s manual row documents
/// `accepted_outputs`/`rejected_outputs` as the conceptual accepted/rejected
/// output boundary; the runtime `CapabilityRegistration` row persists the
/// accepted set as `declared_outputs` (rejects are a separate typed list via
/// `list_capability_rejects`), so -- exactly as the recovered original did --
/// this test checks those two rows directly/statically rather than through
/// `assert_schema_fields_are_real`, to avoid asserting a literal field that
/// does not exist on the runtime struct under that name.
#[tokio::test]
async fn mt125_deferred_boundary_manual_rows_document_live_surface() {
    let (store, _harness) = connected_store().await;
    let capability_row = manual_command("atelier_register_bridge_capability");
    let rejects_row = manual_command("atelier_list_capability_rejects");
    let url_import_row = manual_command("atelier_record_url_image_import");
    let blocked_row = manual_command("atelier_set_calibration_blocked");
    assert_eq!(capability_row.name, "AtelierStore::register_bridge_capability");
    assert_eq!(url_import_row.status, CommandStatus::Wired);
    assert_eq!(
        url_import_row.ipc_channel,
        Some("/atelier/image-import/url"),
        "the wired row must document the real Axum route"
    );

    let run_id = Uuid::new_v4();
    let adapter = ComfyBridgeFakeAdapterV1::default();
    let new_registration = adapter.capability_registration(
        run_id,
        ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
        &format!("artifact://atelier/capability-evidence/{}", Uuid::new_v4()),
    );
    store
        .register_bridge_capability(&new_registration)
        .await
        .expect("register bridge capability");
    let reread_registration = store
        .get_capability_registration(run_id)
        .await
        .expect("re-read capability registration")
        .expect("capability registration present");
    let registration_value =
        serde_json::to_value(&reread_registration).expect("serialize capability registration");
    for field in ["registration_id", "workflow_run_id", "capability_grant_ref"] {
        assert!(
            registration_value.get(field).is_some(),
            "manual row {} documents schema field {field} missing on the runtime row",
            capability_row.id
        );
    }
    assert!(
        capability_row.schema_fields.contains(&"accepted_outputs")
            && capability_row.schema_fields.contains(&"rejected_outputs"),
        "manual row must document the accepted/rejected output boundary"
    );
    assert_eq!(
        reread_registration.declared_outputs, new_registration.accepted_outputs,
        "accepted outputs round-trip as the routable declared set"
    );

    let rejects = store
        .list_capability_rejects(run_id)
        .await
        .expect("re-read capability rejects");
    assert_eq!(
        rejects, new_registration.rejected_outputs,
        "rejected outputs round-trip as typed reject rows"
    );
    assert!(
        rejects_row.schema_fields.contains(&"output_slot")
            && rejects_row.schema_fields.contains(&"reason"),
        "manual row must document the typed reject row shape the runtime returns"
    );

    // The documented not-granted-engine.comfyui refusal is real.
    let mut bad_registration = adapter.capability_registration(
        Uuid::new_v4(),
        ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
        &format!("artifact://atelier/capability-evidence/{}", Uuid::new_v4()),
    );
    bad_registration.capability_grant_ref =
        format!("capgrant://wrong-capability/profile/evidence-{}", Uuid::new_v4());
    let bad_grant = store
        .register_bridge_capability(&bad_registration)
        .await
        .expect_err("grant ref outside engine.comfyui must be refused");
    assert!(
        bad_grant.to_string().contains("capability_grant_ref"),
        "refusal must name the grant boundary: {bad_grant}"
    );
    assert!(
        failure_marker_documented(capability_row, "engine.comfyui"),
        "manual row must document the engine.comfyui grant refusal"
    );

    let import_request = UrlImageImportRequest {
        idempotency_key: format!("url-import-mt125-{}", Uuid::new_v4()),
        source_url: format!("https://example.com/images/{}.png", Uuid::new_v4()),
        expected_mime: Some("image/png".to_string()),
        source_label: Some("mt-125 manual proof".to_string()),
        capability_profile_id: "MediaDownloader".to_string(),
        capability_grant_ref: format!(
            "capgrant://media_downloader/MediaDownloader/evidence-{}",
            Uuid::new_v4()
        ),
        requested_by: "mt-125-manual-proof".to_string(),
    };
    let import = store
        .record_url_image_import(&import_request)
        .await
        .expect("record governed URL image import");
    let import_value = serde_json::to_value(&import).expect("serialize image import record");
    // `ImageImportRecord` persists the request-side `source_url` as governed
    // provenance (`normalized_url` + `source_url_hash`), never as a raw
    // stored field literally named `source_url` -- so (matching the
    // recovered original) that one field is checked semantically instead of
    // via the generic literal-field helper.
    for field in [
        "idempotency_key",
        "capability_profile_id",
        "capability_grant_ref",
        "import_id",
    ] {
        assert!(
            import_value.get(field).is_some(),
            "manual row {} documents schema field {field} missing on the runtime record",
            url_import_row.id
        );
    }
    assert!(
        url_import_row.schema_fields.contains(&"source_url"),
        "manual row must document the source_url request field"
    );
    assert!(
        import.source_url_hash.starts_with("sha256:"),
        "the documented source_url is persisted as a stable provenance hash"
    );

    // Replaying the same idempotency key (same request) resolves to the
    // existing import, as the manual's expected_output documents.
    let replay = store
        .record_url_image_import(&import_request)
        .await
        .expect("replay the same idempotency key");
    assert_eq!(replay.import_id, import.import_id);

    assert_eq!(blocked_row.name, "AtelierStore::set_calibration");
    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;
    let missing_reason = store
        .set_calibration(rig.rig_id, CalibrationState::Unresolved, None)
        .await;
    assert!(
        missing_reason.is_err(),
        "BLOCKED deferral without block_reason must be refused"
    );
    assert!(
        failure_marker_documented(blocked_row, "block_reason"),
        "manual row must document the missing-block_reason refusal"
    );
    store
        .set_calibration(
            rig.rig_id,
            CalibrationState::Unresolved,
            Some("carry-forward: WP-0133 PoseKit activation not yet implementable"),
        )
        .await
        .expect("preserve BLOCKED carry-forward deferral");
    let reread_blocked = store
        .get_calibration(rig.rig_id)
        .await
        .expect("re-read BLOCKED deferral")
        .expect("deferral present");
    assert_eq!(reread_blocked.state, CalibrationState::Unresolved);
    assert!(
        reread_blocked
            .block_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("WP-0133")),
        "the carry-forward reason is preserved verbatim"
    );
    let blocked_value = serde_json::to_value(&reread_blocked).expect("serialize BLOCKED deferral");
    assert_schema_fields_are_real(blocked_row, &blocked_value, &[]);
}

/// MT-130: the ComfyUI job-lifecycle (JOB_ENQUEUED/RUNNING/COMPLETED) and
/// output-registration-recovery event families are already individually
/// covered live by `atelier_comfy_job_tests.rs` (mt126/mt127) and
/// `atelier_comfy_tests.rs`'s registration-failure/retry coverage
/// respectively. `model_manual()` documents no CommandReference row for the
/// job-lifecycle surface itself (no `atelier_enqueue_comfy_job` row exists),
/// so the missing-guarantee cross-check can only apply to the
/// registration-failure/retry rows this item shares with MT-124; this test
/// keeps MT-130's own distinguishing value -- proving both families land for
/// the SAME job/run, not just independently in isolated runs -- while adding
/// that cross-check.
#[tokio::test]
async fn mt130_job_lifecycle_and_registration_recovery_event_families_land_in_ledger() {
    let (store, _harness) = connected_store().await;
    let failure_row = manual_command("atelier_record_comfy_output_registration_failure");
    let retry_row = manual_command("atelier_retry_comfy_output_registration_failure");

    // Every contract family is folded into the aggregate parity list.
    for family in [
        comfy_event_family::JOB_ENQUEUED,
        comfy_event_family::JOB_RUNNING,
        comfy_event_family::JOB_COMPLETED,
        comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RECORDED,
        comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RETRIED,
        comfy_event_family::REPLAY_REQUESTED,
        comfy_event_family::REPLAY_COMPLETED,
        comfy_event_family::REPLAY_FAILED,
    ] {
        assert!(
            event_family::ALL.contains(&family),
            "{family} must be registered in the aggregate event family list"
        );
    }

    // --- queue/run/result: QUEUED -> RUNNING -> COMPLETED through the live store ---
    let run_id = Uuid::new_v4();
    let job = store
        .enqueue_comfy_job(&NewComfyJobRequest {
            workflow_run_id: run_id,
            spec_id: None,
            request_json: serde_json::json!({
                "graph": { "nodes": [{ "id": "1", "class_type": "PoseRig" }] },
                "seed": 130,
            }),
        })
        .await
        .expect("enqueue comfy job");
    assert_eq!(job.status, ComfyJobStatus::Queued);
    store
        .mark_comfy_job_running(job.job_id)
        .await
        .expect("advance job to RUNNING");
    store
        .mark_comfy_job_completed(job.job_id)
        .await
        .expect("resolve job to COMPLETED");
    assert_eq!(job.workflow_run_id, run_id, "job lifecycle stays in this run's aggregate scope");
    let reread_job = store
        .get_comfy_job(job.job_id)
        .await
        .expect("re-read job")
        .expect("job persisted");
    assert_eq!(reread_job.status, ComfyJobStatus::Completed);
    assert!(reread_job.started_at.is_some(), "RUNNING stamped started_at");
    assert!(
        reread_job.finished_at.is_some(),
        "COMPLETED stamped finished_at"
    );

    let job_aggregate = job.job_id.to_string();
    for family in [
        comfy_event_family::JOB_ENQUEUED,
        comfy_event_family::JOB_RUNNING,
        comfy_event_family::JOB_COMPLETED,
    ] {
        let count = store
            .count_events_for_aggregate(family, "atelier_comfy_job", &job_aggregate)
            .await
            .expect("count job lifecycle event");
        assert_eq!(count, 1, "exactly one {family} event for this job");
    }

    let artifact_ref = format!("artifact://atelier/comfy/{}", Uuid::new_v4());
    let failure = store
        .record_comfy_output_registration_failure(&NewComfyOutputRegistrationFailure {
            workflow_run_id: run_id,
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            attempted_registration_id: None,
            source_node_instance_id: "saveimage-late-register".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: artifact_ref.clone(),
            artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: None,
            graph_hash: None,
            seed: Some(130),
            identity_metadata: None,
            failure_stage: "registration".to_string(),
            failure_reason: "capability registration unavailable after image save".to_string(),
            evidence: serde_json::json!({ "case": "mt-130-event-family-proof" }),
        })
        .await
        .expect("preserve saved output whose registration failed for THIS job's run");
    assert_eq!(failure.status, ComfyOutputRegistrationFailureStatus::Retryable);
    let failure_value = serde_json::to_value(&failure).expect("serialize failure row");
    assert_schema_fields_are_real(failure_row, &failure_value, &[]);

    let adapter = ComfyBridgeFakeAdapterV1::default();
    let registration = store
        .register_bridge_capability(&adapter.capability_registration(
            run_id,
            ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
            &format!("artifact://atelier/capability-evidence/{}", Uuid::new_v4()),
        ))
        .await
        .expect("register capability before retry");
    let retry = store
        .retry_comfy_output_registration_failure(
            failure.failure_id,
            Some(registration.registration_id),
        )
        .await
        .expect("recover the preserved output");
    assert_eq!(retry.output.artifact_ref, artifact_ref);
    // RE-READ: the failure row flipped to registered with the resolved link.
    let resolved = store
        .get_comfy_output_registration_failure(failure.failure_id)
        .await
        .expect("re-read failure row")
        .expect("failure row persisted");
    assert_eq!(
        resolved.status,
        ComfyOutputRegistrationFailureStatus::Registered
    );
    assert_eq!(
        resolved.resolved_intake_output_id,
        Some(retry.output.intake_output_id)
    );
    assert_eq!(resolved.retry_count, 1);
    let retry_value = serde_json::to_value(&resolved).expect("serialize resolved failure row");
    let recovered_output_value =
        serde_json::to_value(&retry.output).expect("serialize recovered intake output");
    assert_schema_fields_are_real(retry_row, &retry_value, &[&recovered_output_value]);

    let failure_aggregate = failure.failure_id.to_string();
    for family in [
        comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RECORDED,
        comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RETRIED,
    ] {
        let count = store
            .count_events_for_aggregate(
                family,
                "atelier_comfy_output_registration_failure",
                &failure_aggregate,
            )
            .await
            .expect("count registration recovery event");
        assert_eq!(count, 1, "exactly one {family} event for this failure");
    }
}

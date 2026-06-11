//! WP-KERNEL-005 MT-122/MT-123/MT-124/MT-125 manual source rows: runtime proof.
//!
//! `model_manual_tests.rs::manual_covers_pose_comfy_surfaces` proves the
//! pose/ComfyUI manual rows exist with exact-count parity, but that is a
//! static doc-manifest assertion. These tests close the v2 concern by
//! exercising the surfaces each manual row documents against live
//! Handshake-managed PostgreSQL, RE-READING every persisted record, and
//! asserting that the manual's documented method names, schema fields,
//! error semantics, and EventLedger families match what the real store does.
//! No assertion below is satisfiable by manual text alone.
//!
//! Run, e.g.:
//!   cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --features test-utils --test atelier_pose_comfy_manual_rows_pg_tests \
//!     --target-dir ../Handshake_Artifacts/handshake-cargo-target

mod atelier_pg_support;

use chrono::Duration;
use handshake_core::atelier::comfy::{
    comfy_event_family, ComfyBridgeFakeAdapterV1, ComfyOutputRegistrationFailureStatus,
    ComfyWorkflowHistoryQuery, ComfyWorkflowStatus, MediaKind, NewComfyOutputRegistrationFailure,
    NewComfyWorkflowReceipt, RoutingIntent, COMFY_WORKFLOW_RECEIPT_SCHEMA,
};
use handshake_core::atelier::pose::{
    pose_event_family, CalibrationMarkerColors, CalibrationMarkerVisibility, CalibrationState,
    CanvasSize, DetectorStatus, IdentityCropBox, IdentityCropLandmark, IdentityProfileKind,
    NewIdentityCropArtifact, NewIdentityProfile, NewPoseCalibration, NewPoseContextState,
    NewPoseRig, NewPoseSidecar, PoseContextKind, PoseRig, PoseSidecarKind, PoseSidecarStatus,
    UpdateIdentityProfile, BODY_KEYPOINT_COUNT, FACE_KEYPOINT_COUNT, HAND_KEYPOINT_COUNT,
};
use handshake_core::atelier::{event_family, AtelierStore, NewCharacter, UrlImageImportRequest};
use handshake_core::model_manual::{model_manual, CommandReference, CommandStatus};
use uuid::Uuid;

/// Connect + ensure schema, the shared preamble every fixture test runs.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
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

/// Assert every documented schema field names a real field of the runtime
/// value (top level, or one level down inside one of the named nested docs).
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

fn new_rig(character_internal_id: Uuid, keypoints_json: serde_json::Value) -> NewPoseRig {
    NewPoseRig {
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
        keypoints_json,
        sidecar_ref: Some(format!("artifact://atelier/pose/{}", Uuid::new_v4())),
    }
}

/// Ingest a fresh, run-unique rig for a character and return it.
async fn fresh_rig(store: &AtelierStore, character_internal_id: Uuid) -> PoseRig {
    store
        .ingest_pose_rig(&new_rig(character_internal_id, valid_keypoints()))
        .await
        .expect("ingest pose rig")
}

/// MT-122: the pose context / rig CRUD / calibration manual rows must
/// document the live PostgreSQL surface — methods, schema fields, refusal
/// semantics, and EventLedger families — not just exist as static rows.
#[tokio::test]
async fn mt122_pose_context_and_rig_manual_rows_document_live_pg_surface() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt122_pose_context_and_rig_manual_rows_document_live_pg_surface: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;

    let context_row = manual_command("atelier_set_pose_context_state");
    let rig_row = manual_command("atelier_ingest_pose_rig");
    let calibration_row = manual_command("atelier_set_pose_calibration");
    assert_eq!(context_row.name, "AtelierStore::set_pose_context_state");
    assert_eq!(rig_row.name, "AtelierStore::ingest_pose_rig");
    assert_eq!(calibration_row.name, "AtelierStore::set_pose_calibration");

    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;

    // --- pose context: append through the real store, RE-READ the head ---
    let workspace_ref = format!("pose-workspace://{}", Uuid::new_v4());
    let context = store
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
        .expect("re-read pose context head from PostgreSQL")
        .expect("pose context head present");
    assert_eq!(head.context_id, context.context_id);
    assert_eq!(head.kind, PoseContextKind::CharacterLinked);
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
        context_row
            .common_errors
            .iter()
            .any(|error| error.contains("source_asset_id")),
        "manual row must document the single_image/source_asset_id refusal"
    );

    // --- rig CRUD: RE-READ by id and via list (keypoint schema inspect) ---
    let fetched_rig = store
        .get_pose_rig(rig.rig_id)
        .await
        .expect("re-read pose rig from PostgreSQL");
    assert_eq!(fetched_rig.rig_id, rig.rig_id);
    assert_eq!(fetched_rig.content_hash, rig.content_hash);
    let listed = store
        .list_pose_rigs(character, 100)
        .await
        .expect("list pose rigs for character");
    assert!(
        listed.iter().any(|r| r.rig_id == rig.rig_id),
        "ingested rig must appear in the character's rig list"
    );
    let rig_value = serde_json::to_value(&fetched_rig).expect("serialize re-read rig");
    assert_schema_fields_are_real(rig_row, &rig_value, &[]);

    // Keypoint-cardinality refusal the manual documents is real.
    let mut bad_keypoints = valid_keypoints();
    bad_keypoints["people"][0]["pose_keypoints_2d"] =
        serde_json::json!(vec![0.0_f64; (BODY_KEYPOINT_COUNT - 1) * 3]);
    let bad_rig = store
        .ingest_pose_rig(&new_rig(character, bad_keypoints))
        .await
        .expect_err("17-triple body payload must be refused");
    assert!(
        bad_rig.to_string().contains("triples"),
        "keypoint refusal must name the cardinality contract: {bad_rig}"
    );

    // --- calibration: BLOCKED-by-default is preserved, never faked ---
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
        calibration_row
            .common_errors
            .iter()
            .any(|error| error.contains("block_reason")),
        "manual row must document the missing-block_reason refusal"
    );
    let blocked = store
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
        .expect("re-read calibration from PostgreSQL")
        .expect("calibration present");
    assert_eq!(reread_calibration.state, CalibrationState::Unresolved);
    assert_eq!(reread_calibration.block_reason, blocked.block_reason);
    let calibration_value =
        serde_json::to_value(&reread_calibration).expect("serialize re-read calibration");
    assert_schema_fields_are_real(calibration_row, &calibration_value, &[]);

    // --- EventLedger: every documented mutation landed in the ledger ---
    for (family, aggregate_type, aggregate_id, expected) in [
        (
            pose_event_family::POSE_CONTEXT_STATE_SET,
            "atelier_pose_context_state",
            context.context_id.to_string(),
            1,
        ),
        (
            pose_event_family::POSE_RIG_INGESTED,
            "atelier_pose_rig",
            rig.rig_id.to_string(),
            1,
        ),
        (
            pose_event_family::POSE_CALIBRATION_SET,
            "atelier_pose_rig",
            rig.rig_id.to_string(),
            1,
        ),
    ] {
        let count = store
            .count_events_for_aggregate(family, aggregate_type, &aggregate_id)
            .await
            .expect("count pose mutation events");
        assert_eq!(count, expected, "exactly {expected} {family} event(s)");
    }
}

/// MT-123: the sidecar / hidden-gallery / identity-lineage manual rows must
/// document the live PostgreSQL surface end to end.
#[tokio::test]
async fn mt123_pose_sidecar_and_identity_manual_rows_document_live_pg_surface() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt123_pose_sidecar_and_identity_manual_rows_document_live_pg_surface: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;

    let sidecar_row = manual_command("atelier_record_pose_sidecar");
    let projection_row = manual_command("atelier_pose_sidecar_gallery_projection");
    let profile_row = manual_command("atelier_append_identity_profile");
    let crop_row = manual_command("atelier_record_identity_crop_artifact");
    assert_eq!(sidecar_row.name, "AtelierStore::record_pose_sidecar");
    assert_eq!(profile_row.name, "AtelierStore::append_identity_profile");

    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;

    // --- typed sidecar: persist, RE-READ via list, hidden-gallery project ---
    let json_artifact = atelier_pg_support::write_native_media_artifact(b"mt-123-openpose-json");
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
        .expect("re-read sidecars from PostgreSQL");
    let reread_sidecar = listed
        .iter()
        .find(|s| s.sidecar_id == sidecar.sidecar_id)
        .expect("recorded sidecar present in list");
    assert_eq!(reread_sidecar.kind, PoseSidecarKind::OpenPoseJson);
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
        sidecar_row
            .common_errors
            .iter()
            .any(|error| error.contains("mime")),
        "manual row must document the mime/kind refusal: {mismatched}"
    );

    // Hidden-from-gallery projection: hidden rows with a jump target back.
    let projection = store
        .pose_sidecar_gallery_projection(rig.rig_id)
        .await
        .expect("project sidecars hidden from gallery");
    let projected = projection
        .iter()
        .find(|p| p.sidecar_id == sidecar.sidecar_id)
        .expect("recorded sidecar appears in the projection");
    assert!(
        !projected.gallery_visible,
        "sidecars are hidden from normal galleries"
    );
    let projection_value =
        serde_json::to_value(projected).expect("serialize gallery projection row");
    assert_schema_fields_are_real(projection_row, &projection_value, &[]);

    // --- identity lineage: append, update, RE-READ the head ---
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
    assert_eq!(profile.seq, 1);
    assert_eq!(profile.version, 1);
    let updated = store
        .update_identity_profile(&UpdateIdentityProfile {
            profile_id: profile.profile_id,
            name: "MT-123 face identity v2".to_string(),
            description: "Mutable fields adjusted without changing lineage".to_string(),
            source_ref: Some(format!("source://identity/{}", Uuid::new_v4())),
            crop_ref: Some(format!("crop://identity/{}", Uuid::new_v4())),
            artifact_ref: Some(format!("artifact://identity-profile/{}", Uuid::new_v4())),
            requested_by: "mt-123-manual-proof".to_string(),
        })
        .await
        .expect("update identity profile");
    assert_eq!(updated.version, 2);
    assert_eq!(updated.seq, 1, "update preserves append lineage");
    let head = store
        .latest_identity_profile(character, IdentityProfileKind::Face)
        .await
        .expect("re-read identity head from PostgreSQL")
        .expect("identity head present");
    assert_eq!(head.profile_id, profile.profile_id);
    assert_eq!(head.version, 2);
    let profile_value = serde_json::to_value(&head).expect("serialize re-read identity profile");
    assert_schema_fields_are_real(profile_row, &profile_value, &[]);

    // --- 512x512 crop: pinned to the live profile version, RE-READ ---
    let crop_payload = atelier_pg_support::write_native_media_artifact(b"mt-123-crop-512");
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
    assert_eq!(
        crop.profile_version, 2,
        "crop is pinned to the profile version read from PostgreSQL"
    );
    let crops = store
        .list_identity_crop_artifacts(profile.profile_id)
        .await
        .expect("re-read identity crop artifacts from PostgreSQL");
    let reread_crop = crops
        .iter()
        .find(|c| c.crop_id == crop.crop_id)
        .expect("crop artifact persisted");
    let crop_value = serde_json::to_value(reread_crop).expect("serialize re-read crop artifact");
    assert_schema_fields_are_real(crop_row, &crop_value, &[]);

    // The documented exactly-512x512 refusal is real.
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
        crop_row
            .common_errors
            .iter()
            .any(|error| error.contains("512")),
        "manual row must document the 512x512 refusal"
    );

    // --- EventLedger: documented mutations landed in the ledger ---
    let sidecar_events = store
        .count_events_for_aggregate(
            pose_event_family::POSE_SIDECAR_RECORDED,
            "atelier_pose_sidecar",
            &sidecar.sidecar_id.to_string(),
        )
        .await
        .expect("count sidecar events");
    assert_eq!(sidecar_events, 1);
    let profile_events = store
        .count_events_for_aggregate(
            pose_event_family::IDENTITY_PROFILE_APPENDED,
            "atelier_identity_profile",
            &profile.profile_id.to_string(),
        )
        .await
        .expect("count identity profile events");
    assert_eq!(
        profile_events, 2,
        "append + update each emit one lineage event"
    );
    let crop_events = store
        .count_events_for_aggregate(
            pose_event_family::IDENTITY_CROP_ARTIFACT_RECORDED,
            "atelier_identity_crop_artifact",
            &crop.crop_id.to_string(),
        )
        .await
        .expect("count identity crop events");
    assert_eq!(crop_events, 1);
}

/// MT-124: the workflow receipt / history / fallback / registration-failure
/// manual rows must document the live PostgreSQL surface, including the
/// output-first recovery path.
#[tokio::test]
async fn mt124_comfy_workflow_receipt_manual_rows_document_live_pg_surface() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt124_comfy_workflow_receipt_manual_rows_document_live_pg_surface: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;

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

    // --- durable workflow receipt: persist, RE-READ, schema id parity ---
    let run_id = Uuid::new_v4();
    let spec_ref = format!("workflow-spec://{}", Uuid::new_v4());
    let receipt = store
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
        .expect("re-read workflow receipt from PostgreSQL")
        .expect("workflow receipt present");
    assert_eq!(reread_receipt, receipt);
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

    // --- workflow history replay finds the persisted receipt ---
    let history = store
        .list_comfy_workflow_history(&ComfyWorkflowHistoryQuery {
            character_ref: None,
            workflow_spec_ref: Some(spec_ref.clone()),
            status: None,
            from_utc: Some(reread_receipt.created_at_utc - Duration::seconds(1)),
            to_utc: Some(reread_receipt.created_at_utc + Duration::seconds(1)),
        })
        .await
        .expect("replay workflow history from PostgreSQL");
    assert!(
        history
            .iter()
            .any(|r| r.workflow_run_id == run_id),
        "history replay must include the persisted receipt"
    );
    let history_value =
        serde_json::to_value(&history[0]).expect("serialize history receipt row");
    assert_schema_fields_are_real(history_row, &history_value, &[]);

    // --- SaveImage fallback + per-run intake receipt summary ---
    let empty_reason = store.mark_saveimage_fallback(run_id, "  ").await;
    assert!(
        empty_reason.is_err(),
        "empty fallback_reason must be refused as the manual documents"
    );
    assert!(
        fallback_row
            .common_errors
            .iter()
            .any(|error| error.contains("fallback_reason")),
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
    assert!(
        intake_receipt.fallback_engaged,
        "intake receipt reconstructs the persisted fallback marker"
    );
    let intake_receipt_value =
        serde_json::to_value(&intake_receipt).expect("serialize intake receipt");
    assert_schema_fields_are_real(intake_receipt_row, &intake_receipt_value, &[]);

    // --- output-first persistence: failure preserved, then recovered ---
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
        .expect("recover the preserved output (output-first persistence)");
    assert_eq!(retry.output.artifact_ref, artifact_ref);
    let resolved = store
        .get_comfy_output_registration_failure(failure.failure_id)
        .await
        .expect("re-read failure row from PostgreSQL")
        .expect("failure row still queryable");
    assert_eq!(
        resolved.status,
        ComfyOutputRegistrationFailureStatus::Registered
    );
    assert_eq!(resolved.retry_count, 1);
    let retry_value = serde_json::to_value(&resolved).expect("serialize resolved failure row");
    let recovered_output_value =
        serde_json::to_value(&retry.output).expect("serialize recovered intake output");
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
        retry_row
            .common_errors
            .iter()
            .any(|error| error.contains("not retryable")),
        "manual row must document the not-retryable refusal"
    );

    // --- EventLedger: documented mutations landed in the ledger ---
    for (family, aggregate_type, aggregate_id) in [
        (
            comfy_event_family::WORKFLOW_RECEIPT_RECORDED,
            "atelier_comfy_workflow_receipt",
            run_id.to_string(),
        ),
        (
            comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RECORDED,
            "atelier_comfy_output_registration_failure",
            failure.failure_id.to_string(),
        ),
        (
            comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RETRIED,
            "atelier_comfy_output_registration_failure",
            failure.failure_id.to_string(),
        ),
    ] {
        let count = store
            .count_events_for_aggregate(family, aggregate_type, &aggregate_id)
            .await
            .expect("count comfy receipt-path events");
        assert_eq!(count, 1, "exactly one {family} event for this aggregate");
    }
}

fn failure_marker_documented(row: &CommandReference, marker: &str) -> bool {
    row.common_errors.iter().any(|error| error.contains(marker))
}

/// MT-125: the deferred-boundary manual rows (capability adapter, governed
/// image-sourcing lane, BLOCKED carry-forward calibration) must document the
/// live PostgreSQL surface; the one Wired row must match the real route.
#[tokio::test]
async fn mt125_deferred_boundary_manual_rows_document_live_pg_surface() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt125_deferred_boundary_manual_rows_document_live_pg_surface: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;

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

    // --- adapter boundary: capability-gated routing through live PG ---
    let run_id = Uuid::new_v4();
    let adapter = ComfyBridgeFakeAdapterV1::default();
    let new_registration = adapter.capability_registration(
        run_id,
        ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
        &format!("artifact://atelier/capability-evidence/{}", Uuid::new_v4()),
    );
    let registration = store
        .register_bridge_capability(&new_registration)
        .await
        .expect("register bridge capability");
    let reread_registration = store
        .get_capability_registration(run_id)
        .await
        .expect("re-read capability registration from PostgreSQL")
        .expect("capability registration present");
    assert_eq!(
        reread_registration.registration_id,
        registration.registration_id
    );
    // The documented schema fields map onto the runtime contract: ids and the
    // grant ref live on the persisted row; accepted_outputs becomes the
    // routable declared set; rejected_outputs becomes the typed reject rows.
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

    // Gate-rejected outputs are typed reject rows, never routed.
    let rejects = store
        .list_capability_rejects(run_id)
        .await
        .expect("re-read capability rejects from PostgreSQL");
    assert_eq!(
        rejects, new_registration.rejected_outputs,
        "rejected outputs round-trip as typed reject rows"
    );
    assert!(
        !rejects.is_empty(),
        "the fake adapter declares at least one gate-rejected output"
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
        capability_row
            .common_errors
            .iter()
            .any(|error| error.contains("engine.comfyui")),
        "manual row must document the engine.comfyui grant refusal"
    );

    // --- governed URL image-sourcing lane (the Wired surface) ---
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
    assert_eq!(import.source_kind, "url");
    // The documented schema fields map onto the runtime contract: the
    // request-side source_url is persisted as governed provenance (normalized
    // URL + stable hash), never as a raw stored secret-bearing URL.
    let import_value = serde_json::to_value(&import).expect("serialize image import record");
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

    // --- BLOCKED carry-forward deferral is preserved, never faked ---
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
        blocked_row
            .common_errors
            .iter()
            .any(|error| error.contains("block_reason")),
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
        .expect("re-read BLOCKED deferral from PostgreSQL")
        .expect("deferral present");
    assert_eq!(reread_blocked.state, CalibrationState::Unresolved);
    assert!(
        reread_blocked
            .block_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("WP-0133")),
        "the carry-forward reason is preserved verbatim"
    );

    // --- EventLedger: documented mutations landed in the ledger ---
    let capability_events = store
        .count_events_for_aggregate(
            comfy_event_family::CAPABILITY_REGISTERED,
            "atelier_comfy_capability_registration",
            &registration.registration_id.to_string(),
        )
        .await
        .expect("count capability registration events");
    assert_eq!(capability_events, 1);
    let import_events = store
        .count_events_for_aggregate(
            event_family::IMAGE_IMPORT_RECORDED,
            "atelier_image_import_request",
            &import.import_id.to_string(),
        )
        .await
        .expect("count image import events");
    assert_eq!(
        import_events, 1,
        "the initial import emits one event; idempotent replay does not re-record"
    );
    let calibration_events = store
        .count_events_for_aggregate(
            pose_event_family::POSE_CALIBRATION_SET,
            "atelier_pose_rig",
            &rig.rig_id.to_string(),
        )
        .await
        .expect("count BLOCKED deferral events");
    assert_eq!(calibration_events, 1);
}

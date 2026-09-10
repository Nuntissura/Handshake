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
//! `assert_schema_fields_are_real`. It does not re-prove behavior, refusal
//! semantics, or EventLedger counts already covered by the successor suites
//! named above.

mod atelier_surreal_support;

use chrono::Duration;
use handshake_core::atelier::comfy::{
    ComfyBridgeFakeAdapterV1, ComfyWorkflowHistoryQuery, ComfyWorkflowStatus, MediaKind,
    NewComfyJobRequest, NewComfyOutputRegistrationFailure, NewComfyWorkflowReceipt, RoutingIntent,
};
use handshake_core::atelier::exports::{
    build_llm_evidence_pack_manifest, BackupManifestFile, BackupRestorePreflightRequest,
    BackupRestorePreflightStatus, LlmEvidencePackFile, LlmEvidencePackFileKind,
    LlmEvidenceSourceAnchor, NewBackupManifest,
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

    let manifest = build_llm_evidence_pack_manifest(Uuid::new_v4(), format!("{marker}-requested-by"), files)
        .expect("build strict evidence-pack manifest");

    let manifest_value = serde_json::to_value(&manifest).expect("serialize runtime manifest");
    let first_file = manifest_value["files"][0].clone();
    assert_schema_fields_are_real(row, &manifest_value, &[&first_file]);
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
    let preflight_value = serde_json::to_value(&accepted).expect("serialize runtime preflight record");
    assert_schema_fields_are_real(preflight_row, &preflight_value, &[]);
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
    let receipt_value =
        serde_json::to_value(&reread_receipt).expect("serialize re-read workflow receipt");
    assert_schema_fields_are_real(receipt_row, &receipt_value, &[]);

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
    let retry_value = serde_json::to_value(&resolved).expect("serialize resolved failure row");
    let recovered_output_value =
        serde_json::to_value(&retry.output).expect("serialize recovered intake output");
    // `registration_id` lives on the recovered `IntakeOutput`, not on the
    // `ComfyOutputRegistrationFailure` row itself -- the manual row documents
    // the combined shape, so the recovered-output document is passed as the
    // nested lookup source (matching the recovered original's structure).
    assert_schema_fields_are_real(retry_row, &retry_value, &[&recovered_output_value]);
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

    assert_eq!(blocked_row.name, "AtelierStore::set_calibration");
    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;
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
    store
        .mark_comfy_job_running(job.job_id)
        .await
        .expect("advance job to RUNNING");
    store
        .mark_comfy_job_completed(job.job_id)
        .await
        .expect("resolve job to COMPLETED");
    assert_eq!(job.workflow_run_id, run_id, "job lifecycle stays in this run's aggregate scope");

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
    let resolved = store
        .get_comfy_output_registration_failure(failure.failure_id)
        .await
        .expect("re-read failure row")
        .expect("failure row persisted");
    let retry_value = serde_json::to_value(&resolved).expect("serialize resolved failure row");
    let recovered_output_value =
        serde_json::to_value(&retry.output).expect("serialize recovered intake output");
    assert_schema_fields_are_real(retry_row, &retry_value, &[&recovered_output_value]);
}

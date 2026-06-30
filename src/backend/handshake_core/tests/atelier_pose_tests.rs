//! WP-KERNEL-005 atelier PoseKit: real PostgreSQL round-trip proofs for the
//! pose submodule (rig ingest / head pose / calibration / identity profiles).
//! Run with a live DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_pose_tests -- --nocapture
//!
//! No mocks: each test connects the real `AtelierStore` to a live Postgres,
//! ensures the schema, creates a fresh character (pose rows FK to
//! atelier_character), exercises the pose API with REAL data, and asserts the
//! load-bearing invariants: content-hash idempotency, append-only seq
//! monotonicity, BLOCKED/unresolved calibration preservation + block_reason
//! guard, secret redaction in stored provenance, head-pose upsert stability,
//! and per-mutation event emission via `count_events`. Tables persist between
//! runs, so all source refs / hashes / public ids are made unique per run via
//! `Uuid::new_v4()`. Only `handshake_core` + `tokio` + `uuid` + `serde_json`
//! (+ std) are used; sqlx is never imported directly.

mod atelier_pg_support;
mod user_manual_support;

use handshake_core::api::atelier as atelier_api;
use handshake_core::atelier::collections::NewCollection;
use handshake_core::atelier::pose::{
    generate_posekit_openpose_export, generate_posekit_openpose_export_from_keypoints,
    pose_event_family, CalibrationHandKind, CalibrationHandRow, CalibrationMarkerColors,
    CalibrationMarkerVisibility, CalibrationState, CanvasSize, DetectorStatus, IdentityCropBox,
    IdentityCropLandmark, IdentityProfileKind, NewIdentityCropArtifact, NewIdentityProfile,
    NewPoseCalibration, NewPoseContextState, NewPoseRig, NewPoseSidecar, NewPoseWorkspaceRigState,
    NewPoseWorkspaceRouteTarget, PoseContextKind, PoseOpenPoseSidecarStripItem, PoseRig,
    PoseSidecarGalleryProjection, PoseSidecarKind, PoseSidecarStatus, PoseSourceImageStripItem,
    PoseWorkspaceKeyboardAction, PoseWorkspaceKeyboardActionRequest, PoseWorkspaceRigState,
    PosekitExportFraming, PosekitFramingPreset, PosekitMarkerEdit, PosekitMarkerEditAction,
    PosekitMarkerFamily, PosekitOpenPoseExportRequest, UpdateIdentityProfile, BODY_KEYPOINT_COUNT,
    FACE_KEYPOINT_COUNT, HAND_KEYPOINT_COUNT, POSEKIT_OPENPOSE_EXPORT_HEIGHT,
    POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID, POSEKIT_OPENPOSE_EXPORT_WIDTH,
};
use handshake_core::atelier::{AtelierStore, NewCharacter, NewMediaAsset};
use handshake_core::storage::artifacts::{
    artifact_root_rel, validate_artifact_content_hash, write_file_artifact, ArtifactClassification,
    ArtifactLayer, ArtifactManifest, ArtifactPayloadKind,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// Materialize a fresh, run-unique character and return its internal_id. Pose
/// rows FK to atelier_character, so every test creates one first.
async fn fresh_character(store: &AtelierStore) -> Uuid {
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-pose-{}", Uuid::new_v4()),
            display_name: "Pose Subject".to_string(),
        })
        .await
        .expect("create character");
    character.internal_id
}

/// A valid OpenPose keypoint payload (legacy source `rigToOpenposeJson`): body-18 (the
/// only required field) plus zero-filled face-70 and both hands-21.
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

fn visible_source_body_keypoints() -> serde_json::Value {
    let mut keypoints = valid_keypoints();
    keypoints["people"][0]["pose_keypoints_2d"][0] = serde_json::json!(384.0);
    keypoints["people"][0]["pose_keypoints_2d"][1] = serde_json::json!(120.0);
    keypoints["people"][0]["pose_keypoints_2d"][2] = serde_json::json!(0.95);
    keypoints["people"][0]["pose_keypoints_2d"][3] = serde_json::json!(384.0);
    keypoints["people"][0]["pose_keypoints_2d"][4] = serde_json::json!(220.0);
    keypoints["people"][0]["pose_keypoints_2d"][5] = serde_json::json!(0.94);
    keypoints["people"][0]["pose_keypoints_2d"][6] = serde_json::json!(260.0);
    keypoints["people"][0]["pose_keypoints_2d"][7] = serde_json::json!(240.0);
    keypoints["people"][0]["pose_keypoints_2d"][8] = serde_json::json!(0.91);
    keypoints
}

fn posekit_export_request(yaw_deg: f32) -> PosekitOpenPoseExportRequest {
    PosekitOpenPoseExportRequest {
        source_ref: "atelier://media/mira-demo/pose-source.png".to_string(),
        rig_id: None,
        yaw_deg,
        pitch_deg: 0.0,
        zoom: 1.0,
        include_face: true,
        include_body: true,
        include_hands: true,
        marker_edits: Vec::new(),
        framing: PosekitExportFraming::default(),
    }
}

fn posekit_export_request_with_pose(
    yaw_deg: f32,
    pitch_deg: f32,
    zoom: f32,
) -> PosekitOpenPoseExportRequest {
    let mut request = posekit_export_request(yaw_deg);
    request.pitch_deg = pitch_deg;
    request.zoom = zoom;
    request
}

fn artifact_id_from_payload_ref(artifact_ref: &str) -> Uuid {
    let rest = artifact_ref
        .strip_prefix("artifact://.handshake/artifacts/L1/")
        .expect("ArtifactStore payload ref must use the native L1 prefix");
    let artifact_id = rest
        .strip_suffix("/payload")
        .expect("ArtifactStore payload ref must end with /payload");
    Uuid::parse_str(artifact_id).expect("ArtifactStore payload ref carries a UUID artifact id")
}

fn assert_native_posekit_artifact_response(
    response: &serde_json::Value,
    field: &str,
    expected_mime: &str,
    expected_hash: &str,
) -> Uuid {
    let row = response.get(field).expect("artifact response field");
    let artifact_ref = row["artifact_ref"].as_str().expect("artifact_ref string");
    let manifest_ref = row["manifest_ref"].as_str().expect("manifest_ref string");
    assert!(
        artifact_ref.starts_with("artifact://.handshake/artifacts/L1/"),
        "{field}.artifact_ref must be a native ArtifactStore payload handle: {artifact_ref}"
    );
    assert!(
        !artifact_ref.starts_with("preview://"),
        "{field}.artifact_ref must not be a preview handle: {artifact_ref}"
    );
    assert_eq!(
        manifest_ref,
        artifact_ref.replace("/payload", "/artifact.json")
    );
    assert_eq!(row["mime"], serde_json::json!(expected_mime));
    assert_eq!(row["content_hash"], serde_json::json!(expected_hash));
    artifact_id_from_payload_ref(artifact_ref)
}

fn read_l1_artifact_payload_json(
    workspace_root: &std::path::Path,
    artifact_id: Uuid,
) -> serde_json::Value {
    let payload_path = workspace_root
        .join(artifact_root_rel(ArtifactLayer::L1, artifact_id))
        .join("payload");
    let bytes = std::fs::read(&payload_path)
        .unwrap_or_else(|err| panic!("read ArtifactStore JSON payload at {payload_path:?}: {err}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        panic!("decode ArtifactStore JSON payload at {payload_path:?}: {err}")
    })
}

fn assert_openpose_payload_shape(payload: &serde_json::Value) {
    let person = payload["people"][0]
        .as_object()
        .expect("OpenPose payload has people[0] object");
    assert_eq!(
        person["pose_keypoints_2d"]
            .as_array()
            .expect("body keypoints array")
            .len(),
        BODY_KEYPOINT_COUNT * 3
    );
    assert_eq!(
        person["face_keypoints_2d"]
            .as_array()
            .expect("face keypoints array")
            .len(),
        FACE_KEYPOINT_COUNT * 3
    );
    assert_eq!(
        person["hand_left_keypoints_2d"]
            .as_array()
            .expect("left hand keypoints array")
            .len(),
        HAND_KEYPOINT_COUNT * 3
    );
    assert_eq!(
        person["hand_right_keypoints_2d"]
            .as_array()
            .expect("right hand keypoints array")
            .len(),
        HAND_KEYPOINT_COUNT * 3
    );
}

struct NativePoseSidecarArtifact {
    artifact_ref: String,
    content_hash: String,
    byte_len: i64,
    stored_payload: Vec<u8>,
}

fn write_pose_sidecar_artifact(
    stored_payload: &[u8],
    mime: &str,
    filename_hint: &str,
) -> NativePoseSidecarArtifact {
    let workspace_root = atelier_pg_support::test_artifact_workspace_root();
    let artifact_id = Uuid::now_v7();
    let content_hash = sha256_hex(stored_payload);
    let manifest = ArtifactManifest {
        artifact_id,
        layer: ArtifactLayer::L1,
        kind: ArtifactPayloadKind::File,
        mime: mime.to_string(),
        filename_hint: Some(filename_hint.to_string()),
        created_at: chrono::Utc::now(),
        created_by_job_id: None,
        source_entity_refs: Vec::new(),
        source_artifact_refs: Vec::new(),
        content_hash: content_hash.clone(),
        size_bytes: stored_payload.len() as u64,
        classification: ArtifactClassification::Low,
        exportable: true,
        retention_ttl_days: None,
        pinned: Some(true),
        hash_basis: None,
        hash_exclude_paths: Vec::new(),
    };
    write_file_artifact(&workspace_root, &manifest, stored_payload)
        .expect("write Posekit sidecar ArtifactStore payload");
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, artifact_id)
        .expect("validate Posekit sidecar ArtifactStore payload hash");
    NativePoseSidecarArtifact {
        artifact_ref: format!(
            "artifact://{}/payload",
            artifact_root_rel(ArtifactLayer::L1, artifact_id)
        ),
        content_hash,
        byte_len: stored_payload.len() as i64,
        stored_payload: stored_payload.to_vec(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn artifact_manifest_ref(artifact_ref: &str) -> String {
    artifact_ref.replace("/payload", "/artifact.json")
}

/// Ingest a fresh, run-unique rig for a character and return it.
async fn fresh_rig(store: &AtelierStore, character_internal_id: Uuid) -> PoseRig {
    fresh_rig_with_keypoints(store, character_internal_id, valid_keypoints()).await
}

async fn fresh_rig_with_keypoints(
    store: &AtelierStore,
    character_internal_id: Uuid,
    keypoints_json: serde_json::Value,
) -> PoseRig {
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
            keypoints_json,
            sidecar_ref: Some(format!("artifact://atelier/pose/{}", Uuid::new_v4())),
        })
        .await
        .expect("ingest pose rig")
}

async fn expect_pose_source_ref_rejected(
    store: &AtelierStore,
    character_internal_id: Uuid,
    source_ref: String,
    reason: &str,
) {
    let result = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id,
            source_asset_id: None,
            source_ref,
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 64,
                height: 64,
            },
            detector_provider: "mediapipe".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: None,
            source_asset_path_ref: None,
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: None,
        })
        .await;
    assert!(result.is_err(), "{reason}");
}

#[test]
fn posekit_openpose_export_generates_real_png_and_full_payload_shape() {
    let export = generate_posekit_openpose_export(&posekit_export_request(0.0))
        .expect("generate native Posekit OpenPose export");
    assert_eq!(export.schema_id, POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID);
    assert_eq!(export.width, POSEKIT_OPENPOSE_EXPORT_WIDTH);
    assert_eq!(export.height, POSEKIT_OPENPOSE_EXPORT_HEIGHT);
    assert_eq!(export.yaw_deg, 0);
    assert_eq!(export.zoom_percent, 100);
    assert!(export
        .receipt_ref
        .starts_with("preview://atelier/posekit/openpose/"));
    assert_eq!(export.openpose_json["version"], serde_json::json!(1.3));
    assert_eq!(
        export.openpose_json["handshake_schema"],
        serde_json::json!(POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID)
    );
    assert_openpose_payload_shape(&export.openpose_json);

    let decoded =
        image::load_from_memory(&export.openpose_png_bytes).expect("Posekit export PNG decodes");
    assert_eq!(decoded.width(), POSEKIT_OPENPOSE_EXPORT_WIDTH as u32);
    assert_eq!(decoded.height(), POSEKIT_OPENPOSE_EXPORT_HEIGHT as u32);
    let nonblack_pixels = decoded
        .to_rgba8()
        .pixels()
        .filter(|pixel| pixel.0[0] != 0 || pixel.0[1] != 0 || pixel.0[2] != 0)
        .count();
    assert!(
        nonblack_pixels > 1000,
        "OpenPose PNG must contain a visible skeleton, got {nonblack_pixels} nonblack pixels"
    );
    assert_eq!(export.openpose_json_sha256.len(), 64);
    assert_eq!(export.openpose_png_sha256.len(), 64);
    assert_eq!(export.content_hash.len(), 64);
}

#[test]
fn posekit_openpose_export_can_use_stored_rig_keypoints() {
    let mut keypoints = valid_keypoints();
    keypoints["people"][0]["pose_keypoints_2d"][0] = serde_json::json!(123.4);
    keypoints["people"][0]["pose_keypoints_2d"][1] = serde_json::json!(456.7);
    keypoints["people"][0]["pose_keypoints_2d"][2] = serde_json::json!(0.99);
    let export =
        generate_posekit_openpose_export_from_keypoints(&posekit_export_request(0.0), &keypoints)
            .expect("generate Posekit export from stored rig keypoints");
    let body = export.openpose_json["people"][0]["pose_keypoints_2d"]
        .as_array()
        .expect("body keypoints array");
    assert!((body[0].as_f64().expect("body x") - 123.4).abs() < 0.001);
    assert!((body[1].as_f64().expect("body y") - 456.7).abs() < 0.001);
    assert_eq!(
        export.openpose_json["source_keypoints_ref"],
        serde_json::json!("atelier_pose_rig.keypoints_json")
    );
    assert_eq!(
        export.openpose_json["pose_state"]["yaw_deg"],
        serde_json::json!(0)
    );
    assert_eq!(
        export.openpose_json["pose_state"]["source_keypoint_projection"]["mode"],
        serde_json::json!("native-rig-to-openpose")
    );
    assert_openpose_payload_shape(&export.openpose_json);
}

#[test]
fn posekit_openpose_export_renders_png_from_stored_rig_keypoints() {
    let mut keypoints = valid_keypoints();
    keypoints["people"][0]["pose_keypoints_2d"][0] = serde_json::json!(123.0);
    keypoints["people"][0]["pose_keypoints_2d"][1] = serde_json::json!(456.0);
    keypoints["people"][0]["pose_keypoints_2d"][2] = serde_json::json!(0.99);
    keypoints["people"][0]["pose_keypoints_2d"][3] = serde_json::json!(205.0);
    keypoints["people"][0]["pose_keypoints_2d"][4] = serde_json::json!(456.0);
    keypoints["people"][0]["pose_keypoints_2d"][5] = serde_json::json!(0.99);

    let export =
        generate_posekit_openpose_export_from_keypoints(&posekit_export_request(0.0), &keypoints)
            .expect("generate Posekit export from stored rig keypoints");
    let decoded =
        image::load_from_memory(&export.openpose_png_bytes).expect("Posekit export PNG decodes");
    let rgba = decoded.to_rgba8();
    let source_point_rendered = (118..=128).any(|x| {
        (451..=461).any(|y| {
            let pixel = rgba.get_pixel(x, y).0;
            pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0
        })
    });
    assert!(
        source_point_rendered,
        "rig-backed OpenPose PNG must render the stored keypoint at 123,456"
    );

    let procedural = generate_posekit_openpose_export(&posekit_export_request(45.0))
        .expect("generate procedural Posekit export");
    assert_ne!(
        export.openpose_png_sha256, procedural.openpose_png_sha256,
        "rig-backed PNG hash must differ when stored keypoints differ from procedural pose"
    );
}

#[test]
fn posekit_stored_rig_rotation_rerenders_coordinates_and_png() {
    let keypoints = visible_source_body_keypoints();

    let yaw0 =
        generate_posekit_openpose_export_from_keypoints(&posekit_export_request(0.0), &keypoints)
            .expect("generate yaw 0 source-backed export");
    let yaw90 =
        generate_posekit_openpose_export_from_keypoints(&posekit_export_request(90.0), &keypoints)
            .expect("generate yaw 90 source-backed export");
    let yaw180 =
        generate_posekit_openpose_export_from_keypoints(&posekit_export_request(180.0), &keypoints)
            .expect("generate yaw 180 source-backed export");

    let body0 = yaw0.openpose_json["people"][0]["pose_keypoints_2d"]
        .as_array()
        .expect("yaw0 body");
    let body90 = yaw90.openpose_json["people"][0]["pose_keypoints_2d"]
        .as_array()
        .expect("yaw90 body");
    let body180 = yaw180.openpose_json["people"][0]["pose_keypoints_2d"]
        .as_array()
        .expect("yaw180 body");
    assert!((body0[0].as_f64().expect("yaw0 x") - 384.0).abs() < 0.001);
    assert!(
        body90[0].as_f64().expect("yaw90 x") > body0[0].as_f64().expect("yaw0 x"),
        "yaw 90 must project stored rig keypoints, not only record metadata"
    );
    assert!(
        body180[0].as_f64().expect("yaw180 x") > body90[0].as_f64().expect("yaw90 x"),
        "yaw 180 must project farther than yaw 90"
    );
    assert_ne!(yaw0.openpose_png_sha256, yaw90.openpose_png_sha256);
    assert_ne!(yaw90.openpose_png_sha256, yaw180.openpose_png_sha256);
    assert_eq!(
        yaw90.openpose_json["pose_state"]["source_keypoint_projection"]["mode"],
        serde_json::json!("native-rig-to-openpose")
    );
}

#[test]
fn posekit_stored_rig_pitch_and_zoom_rerender_coordinates_and_png() {
    let keypoints = visible_source_body_keypoints();

    let base = generate_posekit_openpose_export_from_keypoints(
        &posekit_export_request_with_pose(0.0, 0.0, 1.0),
        &keypoints,
    )
    .expect("generate base source-backed export");
    let pitched = generate_posekit_openpose_export_from_keypoints(
        &posekit_export_request_with_pose(0.0, 30.0, 1.0),
        &keypoints,
    )
    .expect("generate pitched source-backed export");
    let zoomed = generate_posekit_openpose_export_from_keypoints(
        &posekit_export_request_with_pose(0.0, 0.0, 1.5),
        &keypoints,
    )
    .expect("generate zoomed source-backed export");

    let base_body = base.openpose_json["people"][0]["pose_keypoints_2d"]
        .as_array()
        .expect("base body");
    let pitched_body = pitched.openpose_json["people"][0]["pose_keypoints_2d"]
        .as_array()
        .expect("pitched body");
    let zoomed_body = zoomed.openpose_json["people"][0]["pose_keypoints_2d"]
        .as_array()
        .expect("zoomed body");
    assert!(
        pitched_body[1].as_f64().expect("pitched y") > base_body[1].as_f64().expect("base y"),
        "pitch must project stored rig keypoint y coordinates, not only update metadata"
    );
    assert!(
        zoomed_body[6].as_f64().expect("zoomed x") < base_body[6].as_f64().expect("base x"),
        "zoom must project off-center stored rig keypoint x coordinates, not only update metadata"
    );
    assert_ne!(base.openpose_png_sha256, pitched.openpose_png_sha256);
    assert_ne!(base.openpose_png_sha256, zoomed.openpose_png_sha256);
    assert_eq!(
        pitched.openpose_json["pose_state"]["source_keypoint_projection"]["pitch_deg"],
        serde_json::json!(30)
    );
    assert_eq!(
        zoomed.openpose_json["pose_state"]["source_keypoint_projection"]["zoom_percent"],
        serde_json::json!(150)
    );
}

#[test]
fn posekit_openpose_export_is_deterministic_and_rotation_changes_state() {
    let first = generate_posekit_openpose_export(&posekit_export_request(90.0))
        .expect("generate first Posekit export");
    let repeat = generate_posekit_openpose_export(&posekit_export_request(90.0))
        .expect("generate repeated Posekit export");
    assert_eq!(first.content_hash, repeat.content_hash);
    assert_eq!(first.openpose_json_bytes, repeat.openpose_json_bytes);
    assert_eq!(first.openpose_png_bytes, repeat.openpose_png_bytes);

    let mut hashes = std::collections::BTreeSet::new();
    for yaw in [-180.0_f32, -90.0, 0.0, 90.0, 180.0] {
        let export = generate_posekit_openpose_export(&posekit_export_request(yaw))
            .expect("generate yaw export");
        hashes.insert(export.content_hash);
        assert_eq!(
            export.openpose_json["pose_state"]["yaw_deg"],
            serde_json::json!(yaw as i32)
        );
    }
    assert_eq!(
        hashes.len(),
        5,
        "the full -180..=180 rotation span must rerender distinct OpenPose state"
    );
}

#[test]
fn posekit_openpose_export_rejects_invisible_or_invalid_requests() {
    let mut blank_source = posekit_export_request(0.0);
    blank_source.source_ref = " ".to_string();
    assert!(
        generate_posekit_openpose_export(&blank_source).is_err(),
        "blank source_ref must be rejected"
    );

    let mut padded_source = posekit_export_request(0.0);
    padded_source.source_ref = " atelier://media/padded ".to_string();
    assert!(
        generate_posekit_openpose_export(&padded_source).is_err(),
        "padded source_ref must be rejected"
    );

    let mut all_off = posekit_export_request(0.0);
    all_off.include_face = false;
    all_off.include_body = false;
    all_off.include_hands = false;
    assert!(
        generate_posekit_openpose_export(&all_off).is_err(),
        "all marker layers off would produce an invisible export and must be rejected"
    );

    let out_of_range = posekit_export_request(270.0);
    assert!(
        generate_posekit_openpose_export(&out_of_range).is_err(),
        "yaw outside the -180..=180 full-rotation span must be rejected"
    );

    let mut nan_yaw = posekit_export_request(0.0);
    nan_yaw.yaw_deg = f32::NAN;
    assert!(
        generate_posekit_openpose_export(&nan_yaw).is_err(),
        "NaN yaw must be rejected"
    );

    let mut bad_source_coordinate = valid_keypoints();
    bad_source_coordinate["people"][0]["pose_keypoints_2d"][0] = serde_json::json!(-20.0);
    bad_source_coordinate["people"][0]["pose_keypoints_2d"][1] = serde_json::json!(120.0);
    bad_source_coordinate["people"][0]["pose_keypoints_2d"][2] = serde_json::json!(0.9);
    assert!(
        generate_posekit_openpose_export_from_keypoints(
            &posekit_export_request(180.0),
            &bad_source_coordinate
        )
        .is_err(),
        "visible source rig points outside the export canvas must fail before projection clamps them"
    );

    let mut bad_source_confidence = visible_source_body_keypoints();
    bad_source_confidence["people"][0]["pose_keypoints_2d"][2] = serde_json::json!(1.4);
    assert!(
        generate_posekit_openpose_export_from_keypoints(
            &posekit_export_request(0.0),
            &bad_source_confidence
        )
        .is_err(),
        "source rig confidence outside 0..=1 must fail before projection normalizes it"
    );
}

#[test]
fn posekit_marker_edits_apply_to_export_and_fail_closed() {
    let mut edited = posekit_export_request(0.0);
    edited.marker_edits.push(PosekitMarkerEdit {
        family: PosekitMarkerFamily::Face,
        index: 12,
        action: PosekitMarkerEditAction::Set,
        x: Some(321.0),
        y: Some(222.0),
        confidence: Some(0.87),
    });
    let export = generate_posekit_openpose_export(&edited)
        .expect("facial marker placement edit should produce an export");
    assert_eq!(export.applied_marker_edit_count, 1);
    assert_eq!(
        export.openpose_json["pose_state"]["marker_edits"][0]["family"],
        serde_json::json!("face")
    );
    let face = export.openpose_json["people"][0]["face_keypoints_2d"]
        .as_array()
        .expect("face keypoints");
    let offset = 12 * 3;
    assert_eq!(face[offset], serde_json::json!(321.0));
    assert_eq!(face[offset + 1], serde_json::json!(222.0));
    assert_eq!(face[offset + 2], serde_json::json!(0.87));

    let mut removed = posekit_export_request(0.0);
    removed.marker_edits.push(PosekitMarkerEdit {
        family: PosekitMarkerFamily::Body,
        index: 10,
        action: PosekitMarkerEditAction::Remove,
        x: None,
        y: None,
        confidence: None,
    });
    let removed_export = generate_posekit_openpose_export(&removed)
        .expect("body marker removal should zero one marker without changing cardinality");
    let body = removed_export.openpose_json["people"][0]["pose_keypoints_2d"]
        .as_array()
        .expect("body keypoints");
    assert_eq!(body.len(), BODY_KEYPOINT_COUNT * 3);
    assert_eq!(body[30], serde_json::json!(0.0));
    assert_eq!(body[31], serde_json::json!(0.0));
    assert_eq!(body[32], serde_json::json!(0.0));

    let mut keypoints = valid_keypoints();
    keypoints["people"][0]["pose_keypoints_2d"][0] = serde_json::json!(120.0);
    keypoints["people"][0]["pose_keypoints_2d"][1] = serde_json::json!(420.0);
    keypoints["people"][0]["pose_keypoints_2d"][2] = serde_json::json!(0.9);
    keypoints["people"][0]["pose_keypoints_2d"][3] = serde_json::json!(180.0);
    keypoints["people"][0]["pose_keypoints_2d"][4] = serde_json::json!(420.0);
    keypoints["people"][0]["pose_keypoints_2d"][5] = serde_json::json!(0.9);
    let mut occupied_keypoints = keypoints.clone();
    occupied_keypoints["people"][0]["hand_left_keypoints_2d"][6] = serde_json::json!(260.0);
    occupied_keypoints["people"][0]["hand_left_keypoints_2d"][7] = serde_json::json!(380.0);
    occupied_keypoints["people"][0]["hand_left_keypoints_2d"][8] = serde_json::json!(0.66);
    let mut add_hand = posekit_export_request(0.0);
    add_hand.marker_edits.push(PosekitMarkerEdit {
        family: PosekitMarkerFamily::LeftHand,
        index: 2,
        action: PosekitMarkerEditAction::Add,
        x: Some(280.0),
        y: Some(380.0),
        confidence: Some(0.72),
    });
    let occupied_err =
        generate_posekit_openpose_export_from_keypoints(&add_hand, &occupied_keypoints)
            .expect_err("add marker into occupied stored-rig slot must fail closed");
    assert!(
        occupied_err
            .to_string()
            .contains("Posekit marker add can only fill an empty zero-confidence slot"),
        "unexpected occupied-slot add error: {occupied_err}"
    );

    keypoints["people"][0]["hand_left_keypoints_2d"][6] = serde_json::json!(0.0);
    keypoints["people"][0]["hand_left_keypoints_2d"][7] = serde_json::json!(0.0);
    keypoints["people"][0]["hand_left_keypoints_2d"][8] = serde_json::json!(0.0);
    let add_export = generate_posekit_openpose_export_from_keypoints(&add_hand, &keypoints)
        .expect("add marker should fill a zero slot without changing OpenPose cardinality");
    let left_hand = add_export.openpose_json["people"][0]["hand_left_keypoints_2d"]
        .as_array()
        .expect("left hand keypoints");
    assert_eq!(left_hand[6], serde_json::json!(280.0));
    assert_eq!(left_hand[7], serde_json::json!(380.0));
    assert_eq!(left_hand[8], serde_json::json!(0.72));

    let mut invalid_index = posekit_export_request(0.0);
    invalid_index.marker_edits.push(PosekitMarkerEdit {
        family: PosekitMarkerFamily::Face,
        index: FACE_KEYPOINT_COUNT,
        action: PosekitMarkerEditAction::Set,
        x: Some(20.0),
        y: Some(20.0),
        confidence: Some(0.9),
    });
    assert!(
        generate_posekit_openpose_export(&invalid_index).is_err(),
        "invalid marker index must fail closed before corrupting output"
    );

    let mut bad_confidence = posekit_export_request(0.0);
    bad_confidence.marker_edits.push(PosekitMarkerEdit {
        family: PosekitMarkerFamily::Face,
        index: 1,
        action: PosekitMarkerEditAction::Set,
        x: Some(20.0),
        y: Some(20.0),
        confidence: Some(1.4),
    });
    assert!(
        generate_posekit_openpose_export(&bad_confidence).is_err(),
        "invalid confidence must fail closed before rendering"
    );
}

#[test]
fn posekit_framing_metadata_changes_pixels_and_keeps_points_visible() {
    let base = generate_posekit_openpose_export(&posekit_export_request(0.0))
        .expect("base Posekit export");
    let mut framed = posekit_export_request(0.0);
    framed.framing = PosekitExportFraming {
        preset: PosekitFramingPreset::FullBodyWithFeet,
        lens_mm: 24,
        padding_top_px: 48,
        padding_right_px: 32,
        padding_bottom_px: 96,
        padding_left_px: 32,
    };
    framed.marker_edits.push(PosekitMarkerEdit {
        family: PosekitMarkerFamily::Face,
        index: 12,
        action: PosekitMarkerEditAction::Set,
        x: Some(321.0),
        y: Some(222.0),
        confidence: Some(0.87),
    });
    let export = generate_posekit_openpose_export(&framed)
        .expect("framed Posekit export should produce nonblank output");

    assert_ne!(base.openpose_png_sha256, export.openpose_png_sha256);
    assert_eq!(export.framing, framed.framing);
    assert_eq!(export.applied_marker_edit_count, 1);
    assert_eq!(
        export.openpose_json["pose_state"]["framing"]["preset"],
        serde_json::json!("full_body_with_feet")
    );
    assert_eq!(
        export.openpose_json["pose_state"]["framing"]["padding_bottom_px"],
        serde_json::json!(96)
    );
    assert_eq!(
        export.openpose_json["pose_state"]["marker_edits"][0]["family"],
        serde_json::json!("face")
    );
    let face = export.openpose_json["people"][0]["face_keypoints_2d"]
        .as_array()
        .expect("face keypoints");
    let face_offset = 12 * 3;
    assert_eq!(face[face_offset], serde_json::json!(321.0));
    assert_eq!(face[face_offset + 1], serde_json::json!(222.0));
    assert_eq!(face[face_offset + 2], serde_json::json!(0.87));

    let body = export.openpose_json["people"][0]["pose_keypoints_2d"]
        .as_array()
        .expect("body keypoints");
    let visible_points = body
        .chunks(3)
        .filter_map(|triple| {
            let x = triple[0].as_f64()?;
            let y = triple[1].as_f64()?;
            let confidence = triple[2].as_f64()?;
            (confidence > 0.0).then_some((x, y))
        })
        .collect::<Vec<_>>();
    assert!(!visible_points.is_empty());
    assert!(visible_points.iter().all(|(x, y)| {
        *x >= 32.0
            && *x <= (POSEKIT_OPENPOSE_EXPORT_WIDTH - 32) as f64
            && *y >= 48.0
            && *y <= (POSEKIT_OPENPOSE_EXPORT_HEIGHT - 96) as f64
    }));
}

#[tokio::test]
async fn atelier_pose_rig_ingest_idempotency_and_roundtrip() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP atelier_pose_rig_ingest_idempotency_and_roundtrip: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let source_artifact = atelier_pg_support::write_native_media_artifact(b"mt-087-source-image");
    let source_asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: source_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: source_artifact.byte_len,
            source_provenance: Some("mt-087 pose source asset".to_string()),
            artifact_ref: source_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize pose source asset");

    // --- ingest a rig with REAL OpenPose keypoints ---
    let source_ref = format!("media://{}", source_asset.asset_id);
    let source_asset_version_ref = format!("media-version://{}/v1", source_asset.asset_id);
    let source_asset_path_ref = format!("source://operator-inbox/{}/portrait.png", Uuid::new_v4());
    let content_hash = format!("sha256-{}", Uuid::new_v4());
    let new = NewPoseRig {
        character_internal_id: character,
        source_asset_id: Some(source_asset.asset_id),
        source_ref: source_ref.clone(),
        content_hash: content_hash.clone(),
        canvas: CanvasSize {
            width: 768,
            height: 1024,
        },
        detector_provider: "mediapipe.tasks-vision.pose".to_string(),
        detector_model: "BlazePose GHUM".to_string(),
        detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
        source_asset_version_ref: Some(source_asset_version_ref.clone()),
        source_asset_path_ref: Some(source_asset_path_ref.clone()),
        confidence_available: true,
        detector_status: DetectorStatus::Detected,
        error_reason: None,
        keypoints_json: valid_keypoints(),
        sidecar_ref: Some(format!("artifact://atelier/pose/{}", Uuid::new_v4())),
    };
    let rig = store.ingest_pose_rig(&new).await.expect("ingest rig");

    // --- round-trip: stored fields match what we wrote, fetch returns same row ---
    assert_eq!(rig.character_internal_id, character);
    assert_eq!(rig.source_ref, source_ref);
    assert_eq!(rig.content_hash, content_hash);
    assert_eq!(rig.canvas.width, 768);
    assert_eq!(rig.canvas.height, 1024);
    assert_eq!(rig.detector_model, "BlazePose GHUM");
    assert_eq!(rig.detector_model_version, "mediapipe-tasks-vision-0.10.20");
    assert_eq!(
        rig.source_asset_version_ref.as_deref(),
        Some(source_asset_version_ref.as_str())
    );
    assert_eq!(
        rig.source_asset_path_ref.as_deref(),
        Some(source_asset_path_ref.as_str())
    );
    assert!(rig.confidence_available);
    assert_eq!(rig.detector_status, DetectorStatus::Detected);
    let fetched = store.get_pose_rig(rig.rig_id).await.expect("get rig");
    assert_eq!(fetched, rig, "get_pose_rig round-trips the ingested rig");
    // Keypoints survive the JSONB round-trip intact.
    assert_eq!(
        fetched.keypoints_json, new.keypoints_json,
        "OpenPose keypoint payload round-trips verbatim through JSONB"
    );

    // --- IDEMPOTENCY: re-ingesting the same (character, source_ref, content_hash)
    //     returns the same rig_id, no duplicate row ---
    let rig_again = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: None,
            source_ref: source_ref.clone(),
            content_hash: content_hash.clone(),
            canvas: CanvasSize {
                width: 768,
                height: 1024,
            },
            detector_provider: "fallback".to_string(),
            detector_model: "fallback-pose-model".to_string(),
            detector_model_version: "fallback-v2".to_string(),
            source_asset_version_ref: Some(format!("media-version://{}/v2", source_asset.asset_id)),
            source_asset_path_ref: Some(format!(
                "source://operator-inbox/{}/portrait-redetect.png",
                Uuid::new_v4()
            )),
            confidence_available: false,
            detector_status: DetectorStatus::Fallback,
            error_reason: Some(
                "deterministic body-18 fallback: face and hand landmarks unavailable".to_string(),
            ),
            keypoints_json: valid_keypoints(),
            sidecar_ref: None,
        })
        .await
        .expect("re-ingest identical rig");
    assert_eq!(
        rig.rig_id, rig_again.rig_id,
        "re-ingesting the same (character, source_ref, content_hash) is idempotent"
    );

    // --- INVARIANT: an empty content_hash is rejected (idempotency key guard) ---
    let bad = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: None,
            source_ref: format!("portrait://{}", Uuid::new_v4()),
            content_hash: "   ".to_string(),
            canvas: CanvasSize {
                width: 64,
                height: 64,
            },
            detector_provider: "mediapipe".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: None,
            source_asset_path_ref: None,
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: None,
        })
        .await;
    assert!(bad.is_err(), "empty content_hash must be rejected");

    // --- INVARIANT: stale local-runtime refs are rejected before storage (MT-004) ---
    let bad_source_ref = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: None,
            source_ref: "http://localhost:9000/pose.json".to_string(),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 64,
                height: 64,
            },
            detector_provider: "mediapipe".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: None,
            source_asset_path_ref: None,
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: None,
        })
        .await;
    assert!(
        bad_source_ref.is_err(),
        "localhost pose source_ref must be rejected"
    );
    for (forbidden_ref, reason) in [
        (
            "ckc://posekit/source-image".to_string(),
            "CKC namespace pose source_ref must be rejected",
        ),
        (
            "electron://ipc/posekit/source-image".to_string(),
            "Electron IPC pose source_ref must be rejected",
        ),
        (
            "llm://pose-detector/openpose".to_string(),
            "direct LLM pose source_ref must be rejected",
        ),
        (
            ".GOV/pose-output/openpose.json".to_string(),
            ".GOV output pose source_ref must be rejected",
        ),
        (
            "C:\\operator\\pose-source.png".to_string(),
            "machine-local pose source_ref must be rejected",
        ),
    ] {
        expect_pose_source_ref_rejected(&store, character, forbidden_ref, reason).await;
    }

    let bad_sidecar_ref = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: None,
            source_ref: format!("portrait://{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 64,
                height: 64,
            },
            detector_provider: "mediapipe".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: None,
            source_asset_path_ref: None,
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: Some(".GOV/pose-sidecar.json".to_string()),
        })
        .await;
    assert!(
        bad_sidecar_ref.is_err(),
        ".GOV pose sidecar_ref must be rejected"
    );

    // --- INVARIANT: malformed keypoint cardinality is rejected (structural gate) ---
    let bad_kp = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: None,
            source_ref: format!("portrait://{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 64,
                height: 64,
            },
            detector_provider: "mediapipe".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: None,
            source_asset_path_ref: None,
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: serde_json::json!({
                "people": [{ "pose_keypoints_2d": vec![0.0_f64; 9] }]
            }),
            sidecar_ref: None,
        })
        .await;
    assert!(
        bad_kp.is_err(),
        "wrong body-18 cardinality must be rejected before storage"
    );

    let bad_provenance_ref = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: Some(source_asset.asset_id),
            source_ref: format!("media://{}", source_asset.asset_id),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 64,
                height: 64,
            },
            detector_provider: "mediapipe".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: Some(format!("media-version://{}/v2", source_asset.asset_id)),
            source_asset_path_ref: Some("C:\\operator\\portrait.png".to_string()),
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: None,
        })
        .await;
    assert!(
        bad_provenance_ref.is_err(),
        "machine-local source_asset_path_ref must be rejected"
    );

    let fallback_reason =
        "deterministic body-18 fallback: face and hand landmarks unavailable".to_string();
    let fallback_rig = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: Some(source_asset.asset_id),
            source_ref: format!("media://{}#fallback", source_asset.asset_id),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 768,
                height: 1024,
            },
            detector_provider: "fallback".to_string(),
            detector_model: "deterministic-body-18".to_string(),
            detector_model_version: "1".to_string(),
            source_asset_version_ref: Some(format!("media-version://{}/v3", source_asset.asset_id)),
            source_asset_path_ref: Some(format!(
                "source://operator-inbox/{}/portrait-fallback.png",
                Uuid::new_v4()
            )),
            confidence_available: false,
            detector_status: DetectorStatus::Fallback,
            error_reason: Some(fallback_reason.clone()),
            keypoints_json: valid_keypoints(),
            sidecar_ref: None,
        })
        .await
        .expect("ingest fallback rig with reason");
    assert_eq!(fallback_rig.detector_status, DetectorStatus::Fallback);
    assert_eq!(
        fallback_rig.error_reason.as_deref(),
        Some(fallback_reason.as_str())
    );
    let fetched_fallback = store
        .get_pose_rig(fallback_rig.rig_id)
        .await
        .expect("fetch fallback rig");
    assert_eq!(
        fetched_fallback.error_reason.as_deref(),
        Some(fallback_reason.as_str()),
        "fallback error_reason must round-trip through Postgres"
    );

    let failed_reason = "detector job failed: no visible body landmarks".to_string();
    let failed_rig = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: Some(source_asset.asset_id),
            source_ref: format!("media://{}#failed", source_asset.asset_id),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 768,
                height: 1024,
            },
            detector_provider: "mediapipe.tasks-vision.pose".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: Some(format!("media-version://{}/v4", source_asset.asset_id)),
            source_asset_path_ref: Some(format!(
                "source://operator-inbox/{}/portrait-failed.png",
                Uuid::new_v4()
            )),
            confidence_available: false,
            detector_status: DetectorStatus::Failed,
            error_reason: Some(failed_reason.clone()),
            keypoints_json: valid_keypoints(),
            sidecar_ref: None,
        })
        .await
        .expect("ingest failed rig with reason");
    assert_eq!(failed_rig.detector_status, DetectorStatus::Failed);
    assert_eq!(
        failed_rig.error_reason.as_deref(),
        Some(failed_reason.as_str())
    );

    let fallback_without_reason = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: Some(source_asset.asset_id),
            source_ref: format!("media://{}#fallback-missing-reason", source_asset.asset_id),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 768,
                height: 1024,
            },
            detector_provider: "fallback".to_string(),
            detector_model: "deterministic-body-18".to_string(),
            detector_model_version: "1".to_string(),
            source_asset_version_ref: Some(format!("media-version://{}/v5", source_asset.asset_id)),
            source_asset_path_ref: Some(format!(
                "source://operator-inbox/{}/portrait-fallback-missing-reason.png",
                Uuid::new_v4()
            )),
            confidence_available: false,
            detector_status: DetectorStatus::Fallback,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: None,
        })
        .await;
    assert!(
        fallback_without_reason.is_err(),
        "fallback detector status must persist a non-empty error_reason"
    );

    // --- EVENT EMISSION: exactly one new rig-ingested event (the duplicate
    //     ingest still upserts + emits, the two rejected ingests must not) ---
    let events_after = store
        .count_events_for_aggregate(
            pose_event_family::POSE_RIG_INGESTED,
            "atelier_pose_rig",
            &rig.rig_id.to_string(),
        )
        .await
        .expect("count run-scoped rig-ingested events");
    assert_eq!(
        events_after, 2,
        "two successful ingests (original + idempotent re-ingest) each emit one event; rejected ingests emit none"
    );
    let rig_event_payload: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_pose_rig'
             AND aggregate_id = $2
           ORDER BY created_at_utc ASC
           LIMIT 1"#,
    )
    .bind(pose_event_family::POSE_RIG_INGESTED)
    .bind(rig.rig_id.to_string())
    .fetch_one(store.pool())
    .await
    .expect("read pose rig ingest event payload");
    assert_eq!(
        rig_event_payload["detector_model"],
        serde_json::json!("BlazePose GHUM")
    );
    assert_eq!(
        rig_event_payload["detector_model_version"],
        serde_json::json!("mediapipe-tasks-vision-0.10.20")
    );
    assert_eq!(
        rig_event_payload["source_asset_version_ref"],
        serde_json::json!(source_asset_version_ref)
    );
    assert_eq!(
        rig_event_payload["source_asset_path_ref"],
        serde_json::json!(source_asset_path_ref)
    );
    assert_eq!(
        rig_event_payload["confidence_available"],
        serde_json::json!(true)
    );
    assert_eq!(rig_event_payload["error_reason"], serde_json::json!(null));

    let fallback_event_payload: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_pose_rig'
             AND aggregate_id = $2
           ORDER BY created_at_utc ASC
           LIMIT 1"#,
    )
    .bind(pose_event_family::POSE_RIG_INGESTED)
    .bind(fallback_rig.rig_id.to_string())
    .fetch_one(store.pool())
    .await
    .expect("read fallback pose rig ingest event payload");
    assert_eq!(
        fallback_event_payload["error_reason"],
        serde_json::json!(fallback_reason)
    );

    // The rig is listable for the character.
    let listed = store
        .list_pose_rigs(character, 50)
        .await
        .expect("list rigs");
    assert!(
        listed.iter().any(|r| r.rig_id == rig.rig_id),
        "ingested rig is listed for its character"
    );
}

#[tokio::test]
async fn atelier_posekit_generated_openpose_export_records_real_sidecars() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_posekit_generated_openpose_export_records_real_sidecars: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;
    let export = generate_posekit_openpose_export(&posekit_export_request(90.0))
        .expect("generate native Posekit export");

    let json_artifact = write_pose_sidecar_artifact(
        &export.openpose_json_bytes,
        "application/json",
        "posekit-openpose.json",
    );
    let png_artifact = write_pose_sidecar_artifact(
        &export.openpose_png_bytes,
        "image/png",
        "posekit-openpose.png",
    );
    image::load_from_memory(&png_artifact.stored_payload)
        .expect("generated OpenPose PNG sidecar payload decodes");

    let json_sidecar = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::OpenPoseJson,
            artifact_ref: json_artifact.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&json_artifact.artifact_ref),
            content_hash: json_artifact.content_hash.clone(),
            byte_len: json_artifact.byte_len,
            mime: "application/json".to_string(),
            width: export.width,
            height: export.height,
            status: PoseSidecarStatus::Rendered,
            error_message: None,
        })
        .await
        .expect("record generated OpenPose JSON sidecar");
    let png_sidecar = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::OpenPosePng,
            artifact_ref: png_artifact.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&png_artifact.artifact_ref),
            content_hash: png_artifact.content_hash.clone(),
            byte_len: png_artifact.byte_len,
            mime: "image/png".to_string(),
            width: export.width,
            height: export.height,
            status: PoseSidecarStatus::Rendered,
            error_message: None,
        })
        .await
        .expect("record generated OpenPose PNG sidecar");

    let sidecars = store
        .list_pose_sidecars(rig.rig_id)
        .await
        .expect("list generated Posekit sidecars");
    assert_eq!(
        sidecars
            .iter()
            .map(|sidecar| sidecar.kind)
            .collect::<Vec<_>>(),
        vec![PoseSidecarKind::OpenPoseJson, PoseSidecarKind::OpenPosePng],
        "generated export sidecars should list in deterministic OpenPose JSON/PNG order"
    );
    assert_eq!(json_sidecar.width, POSEKIT_OPENPOSE_EXPORT_WIDTH);
    assert_eq!(json_sidecar.height, POSEKIT_OPENPOSE_EXPORT_HEIGHT);
    assert_eq!(png_sidecar.width, POSEKIT_OPENPOSE_EXPORT_WIDTH);
    assert_eq!(png_sidecar.height, POSEKIT_OPENPOSE_EXPORT_HEIGHT);
    assert_eq!(png_sidecar.content_hash, png_artifact.content_hash);
    assert!(export.receipt_ref.contains(export.content_hash.as_str()));
    assert!(export.receipt_ref.ends_with("/receipt"));
    assert_openpose_payload_shape(&export.openpose_json);
}

#[tokio::test]
async fn atelier_posekit_openpose_export_route_returns_real_artifact_refs() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_posekit_openpose_export_route_returns_real_artifact_refs: PostgreSQL unavailable"
        );
        return;
    };
    let workspace_root = atelier_pg_support::test_artifact_workspace_root();
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let rig = fresh_rig_with_keypoints(&store, character, visible_source_body_keypoints()).await;

    let state = user_manual_support::app_state_for(&url).await;
    let (base, server) = user_manual_support::start_server(atelier_api::routes(state)).await;
    let mut request = posekit_export_request_with_pose(90.0, 30.0, 1.5);
    request.source_ref = rig.source_ref.clone();
    request.rig_id = Some(rig.rig_id);
    request.include_hands = false;
    request.framing = PosekitExportFraming {
        preset: PosekitFramingPreset::FullBodyWithFeet,
        lens_mm: 24,
        padding_top_px: 48,
        padding_right_px: 32,
        padding_bottom_px: 96,
        padding_left_px: 32,
    };
    request.marker_edits.push(PosekitMarkerEdit {
        family: PosekitMarkerFamily::Face,
        index: 12,
        action: PosekitMarkerEditAction::Set,
        x: Some(321.0),
        y: Some(222.0),
        confidence: Some(0.87),
    });

    let response = reqwest::Client::new()
        .post(format!("{base}/atelier/posekit/openpose-export"))
        .header("x-hsk-actor-id", "posekit-route-test")
        .json(&request)
        .send()
        .await
        .expect("send Posekit OpenPose export route request");
    let status = response.status();
    let text = response.text().await.expect("read Posekit route response");
    server.abort();
    assert!(
        status.is_success(),
        "Posekit route must return success, got {status}: {text}"
    );
    let body: serde_json::Value = serde_json::from_str(&text).expect("Posekit route response JSON");

    assert_eq!(
        body["schema_id"],
        serde_json::json!(POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID)
    );
    assert_eq!(body["source_ref"], serde_json::json!(rig.source_ref));
    assert_eq!(body["rig_id"], serde_json::json!(rig.rig_id));
    assert_eq!(body["yaw_deg"], serde_json::json!(90));
    assert_eq!(body["pitch_deg"], serde_json::json!(30));
    assert_eq!(body["zoom_percent"], serde_json::json!(150));
    assert_eq!(
        body["openpose_json"]["pose_state"]["source_keypoint_projection"]["mode"],
        serde_json::json!("native-rig-to-openpose")
    );
    assert_eq!(
        body["openpose_json"]["pose_state"]["source_keypoint_projection"]["pitch_deg"],
        serde_json::json!(30)
    );
    assert_eq!(
        body["openpose_json"]["pose_state"]["source_keypoint_projection"]["zoom_percent"],
        serde_json::json!(150)
    );
    assert_eq!(body["applied_marker_edit_count"], serde_json::json!(1));
    assert_eq!(
        body["framing"]["preset"],
        serde_json::json!("full_body_with_feet")
    );
    assert_eq!(body["framing"]["lens_mm"], serde_json::json!(24));
    assert_eq!(
        body["openpose_json"]["pose_state"]["marker_edits"][0]["family"],
        serde_json::json!("face")
    );
    let route_marker_confidence = body["openpose_json"]["pose_state"]["marker_edits"][0]
        ["confidence"]
        .as_f64()
        .expect("route marker confidence");
    assert!(
        (route_marker_confidence - 0.87).abs() < 0.000_001,
        "route marker confidence must preserve the staged value, got {route_marker_confidence}"
    );

    let png_artifact_id = assert_native_posekit_artifact_response(
        &body,
        "openpose_png_artifact",
        "image/png",
        body["openpose_png_sha256"]
            .as_str()
            .expect("openpose_png_sha256 string"),
    );
    let json_artifact_id = assert_native_posekit_artifact_response(
        &body,
        "openpose_json_artifact",
        "application/json",
        body["openpose_json_sha256"]
            .as_str()
            .expect("openpose_json_sha256 string"),
    );
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, png_artifact_id)
        .expect("route-produced OpenPose PNG artifact validates");
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, json_artifact_id)
        .expect("route-produced OpenPose JSON artifact validates");
    let stored_openpose_json = read_l1_artifact_payload_json(&workspace_root, json_artifact_id);
    assert_eq!(
        stored_openpose_json["pose_state"]["marker_edits"][0]["index"],
        serde_json::json!(12)
    );
    assert_eq!(
        stored_openpose_json["pose_state"]["framing"]["padding_bottom_px"],
        serde_json::json!(96)
    );

    let receipt_ref = body["receipt_ref"]
        .as_str()
        .expect("Posekit route receipt_ref string");
    assert!(
        receipt_ref.starts_with("artifact://.handshake/artifacts/L1/"),
        "route receipt_ref must be a native ArtifactStore payload handle: {receipt_ref}"
    );
    assert!(
        !receipt_ref.starts_with("preview://"),
        "route receipt_ref must not be a preview handle: {receipt_ref}"
    );
    validate_artifact_content_hash(
        &workspace_root,
        ArtifactLayer::L1,
        artifact_id_from_payload_ref(receipt_ref),
    )
    .expect("route-produced Posekit receipt artifact validates");
    let stored_receipt =
        read_l1_artifact_payload_json(&workspace_root, artifact_id_from_payload_ref(receipt_ref));
    assert_eq!(
        stored_receipt["marker_edits"][0]["family"],
        serde_json::json!("face")
    );
    assert_eq!(
        stored_receipt["framing"]["preset"],
        serde_json::json!("full_body_with_feet")
    );
    assert_eq!(
        stored_receipt["applied_marker_edit_count"],
        serde_json::json!(1)
    );

    let sidecars = body["sidecars"]
        .as_array()
        .expect("Posekit route sidecars array");
    assert_eq!(sidecars.len(), 2);
    assert!(sidecars.iter().any(|sidecar| {
        sidecar["artifact_ref"] == body["openpose_json_artifact"]["artifact_ref"]
            && sidecar["kind"] == serde_json::json!("openpose_json")
    }));
    assert!(sidecars.iter().any(|sidecar| {
        sidecar["artifact_ref"] == body["openpose_png_artifact"]["artifact_ref"]
            && sidecar["kind"] == serde_json::json!("openpose_png")
    }));
}

#[tokio::test]
async fn atelier_pose_openpose_png_sidecars_are_registered_as_portable_artifacts() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_pose_openpose_png_sidecars_are_registered_as_portable_artifacts: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;

    let export = generate_posekit_openpose_export(&posekit_export_request(0.0))
        .expect("generate test OpenPose export");
    let json_artifact = write_pose_sidecar_artifact(
        &export.openpose_json_bytes,
        "application/json",
        "posekit-openpose.json",
    );
    let preview_png = write_pose_sidecar_artifact(
        &export.openpose_png_bytes,
        "image/png",
        "posekit-openpose-preview.png",
    );
    let conditioning_png = write_pose_sidecar_artifact(
        &export.openpose_png_bytes,
        "image/png",
        "posekit-conditioning.png",
    );

    let json_sidecar = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::OpenPoseJson,
            artifact_ref: json_artifact.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&json_artifact.artifact_ref),
            content_hash: json_artifact.content_hash.clone(),
            byte_len: json_artifact.byte_len,
            mime: "application/json".to_string(),
            width: export.width,
            height: export.height,
            status: PoseSidecarStatus::Rendered,
            error_message: None,
        })
        .await
        .expect("record OpenPose JSON sidecar");
    let preview_sidecar = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::OpenPosePng,
            artifact_ref: preview_png.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&preview_png.artifact_ref),
            content_hash: preview_png.content_hash.clone(),
            byte_len: preview_png.byte_len,
            mime: "image/png".to_string(),
            width: export.width,
            height: export.height,
            status: PoseSidecarStatus::Rendered,
            error_message: None,
        })
        .await
        .expect("record OpenPose PNG preview sidecar");
    let conditioning_sidecar = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::ConditioningPng,
            artifact_ref: conditioning_png.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&conditioning_png.artifact_ref),
            content_hash: conditioning_png.content_hash.clone(),
            byte_len: conditioning_png.byte_len,
            mime: "image/png".to_string(),
            width: export.width,
            height: export.height,
            status: PoseSidecarStatus::Rendered,
            error_message: None,
        })
        .await
        .expect("record conditioning PNG sidecar");

    let sidecars = store
        .list_pose_sidecars(rig.rig_id)
        .await
        .expect("list pose sidecars");
    assert_eq!(sidecars.len(), 3);
    assert_eq!(
        sidecars
            .iter()
            .map(|sidecar| sidecar.kind)
            .collect::<Vec<_>>(),
        vec![
            PoseSidecarKind::OpenPoseJson,
            PoseSidecarKind::OpenPosePng,
            PoseSidecarKind::ConditioningPng
        ],
        "sidecars list in deterministic OpenPose/PNG/conditioning order"
    );
    assert!(
        sidecars.iter().all(|sidecar| {
            sidecar
                .artifact_ref
                .starts_with("artifact://.handshake/artifacts/")
                && !sidecar.artifact_ref.contains(".GOV")
                && !sidecar.artifact_ref.contains('\\')
        }),
        "pose sidecars must be portable ArtifactStore handles outside .GOV"
    );
    assert_eq!(json_sidecar.role, "openpose_json");
    assert_eq!(
        json_sidecar.manifest_ref,
        artifact_manifest_ref(&json_artifact.artifact_ref)
    );
    assert_eq!(json_sidecar.mime, "application/json");
    assert_eq!(preview_sidecar.mime, "image/png");
    assert_eq!(conditioning_sidecar.mime, "image/png");
    assert_eq!(preview_sidecar.source_ref, rig.source_ref);
    assert_eq!(preview_sidecar.source_asset_id, rig.source_asset_id);
    assert_eq!(preview_sidecar.width, export.width);
    assert_eq!(preview_sidecar.height, export.height);
    assert_eq!(preview_sidecar.status, PoseSidecarStatus::Rendered);
    assert_eq!(preview_sidecar.error_message, None);

    let wrong_mime = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::OpenPosePng,
            artifact_ref: preview_png.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&preview_png.artifact_ref),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            byte_len: preview_png.byte_len,
            mime: "image/jpeg".to_string(),
            width: export.width,
            height: export.height,
            status: PoseSidecarStatus::Rendered,
            error_message: None,
        })
        .await
        .expect_err("OpenPose PNG sidecars require image/png");
    assert!(wrong_mime.to_string().contains("mime"));

    let gov_ref = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::ConditioningPng,
            artifact_ref: "artifact://.GOV/pose/openpose.png".to_string(),
            manifest_ref: artifact_manifest_ref(&conditioning_png.artifact_ref),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            byte_len: conditioning_png.byte_len,
            mime: "image/png".to_string(),
            width: rig.canvas.width,
            height: rig.canvas.height,
            status: PoseSidecarStatus::Rendered,
            error_message: None,
        })
        .await
        .expect_err(".GOV pose sidecar refs are rejected");
    assert!(gov_ref.to_string().contains("artifact_ref"));

    let rendered_with_error = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::ConditioningPng,
            artifact_ref: conditioning_png.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&conditioning_png.artifact_ref),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            byte_len: conditioning_png.byte_len,
            mime: "image/png".to_string(),
            width: rig.canvas.width,
            height: rig.canvas.height,
            status: PoseSidecarStatus::Rendered,
            error_message: Some("should not persist on rendered sidecar".to_string()),
        })
        .await
        .expect_err("rendered sidecars must not carry error messages");
    assert!(rendered_with_error.to_string().contains("error_message"));

    let preview_events = store
        .count_events_for_aggregate(
            pose_event_family::POSE_SIDECAR_RECORDED,
            "atelier_pose_sidecar",
            &preview_sidecar.sidecar_id.to_string(),
        )
        .await
        .expect("count pose sidecar event");
    assert_eq!(preview_events, 1);
    let json_sidecar_event: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_pose_sidecar'
             AND aggregate_id = $2
           ORDER BY created_at_utc ASC
           LIMIT 1"#,
    )
    .bind(pose_event_family::POSE_SIDECAR_RECORDED)
    .bind(json_sidecar.sidecar_id.to_string())
    .fetch_one(store.pool())
    .await
    .expect("read OpenPose JSON sidecar event payload");
    assert_eq!(
        json_sidecar_event["role"],
        serde_json::json!("openpose_json")
    );
    assert_eq!(
        json_sidecar_event["manifest_ref"],
        serde_json::json!(artifact_manifest_ref(&json_artifact.artifact_ref))
    );
}

#[tokio::test]
async fn atelier_pose_sidecars_lookup_by_source_and_hidden_gallery_projection() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_pose_sidecars_lookup_by_source_and_hidden_gallery_projection: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;

    let export = generate_posekit_openpose_export(&posekit_export_request(0.0))
        .expect("generate lookup OpenPose export");
    for (kind, mime, payload) in [
        (
            PoseSidecarKind::OpenPoseJson,
            "application/json",
            export.openpose_json_bytes.as_slice(),
        ),
        (
            PoseSidecarKind::OpenPosePng,
            "image/png",
            export.openpose_png_bytes.as_slice(),
        ),
        (
            PoseSidecarKind::ConditioningPng,
            "image/png",
            export.openpose_png_bytes.as_slice(),
        ),
    ] {
        let artifact = write_pose_sidecar_artifact(
            payload,
            mime,
            match kind {
                PoseSidecarKind::OpenPoseJson => "posekit-lookup-openpose.json",
                PoseSidecarKind::OpenPosePng => "posekit-lookup-openpose.png",
                PoseSidecarKind::ConditioningPng => "posekit-lookup-conditioning.png",
            },
        );
        store
            .record_pose_sidecar(&NewPoseSidecar {
                rig_id: rig.rig_id,
                kind,
                artifact_ref: artifact.artifact_ref.clone(),
                manifest_ref: artifact_manifest_ref(&artifact.artifact_ref),
                content_hash: artifact.content_hash,
                byte_len: artifact.byte_len,
                mime: mime.to_string(),
                width: export.width,
                height: export.height,
                status: PoseSidecarStatus::Rendered,
                error_message: None,
            })
            .await
            .expect("record lookup sidecar");
    }

    let source_sidecars = store
        .list_pose_sidecars_for_source(&rig.source_ref, Some(rig.rig_id))
        .await
        .expect("lookup pose sidecars by source + rig");
    assert_eq!(source_sidecars.len(), 3);
    assert!(
        source_sidecars
            .iter()
            .all(|sidecar| sidecar.source_ref == rig.source_ref && sidecar.rig_id == rig.rig_id),
        "lookup stays scoped to the requested source image and rig"
    );

    let wrong_source = store
        .list_pose_sidecars_for_source(&format!("portrait://{}", Uuid::new_v4()), Some(rig.rig_id))
        .await
        .expect("wrong source lookup returns empty set");
    assert!(wrong_source.is_empty());

    let bad_source = store
        .list_pose_sidecars_for_source(".GOV/pose-source", Some(rig.rig_id))
        .await
        .expect_err(".GOV source lookups are rejected");
    assert!(bad_source.to_string().contains("source_ref"));

    let projection: Vec<PoseSidecarGalleryProjection> = store
        .pose_sidecar_gallery_projection(rig.rig_id)
        .await
        .expect("build pose sidecar gallery projection");
    assert_eq!(projection.len(), 3);
    assert!(
        projection
            .iter()
            .all(|item| !item.gallery_visible && item.hidden_reason == "pose_sidecar"),
        "pose sidecar projection explicitly hides OpenPose/conditioning artifacts from normal galleries"
    );
}

#[tokio::test]
async fn atelier_pose_strip_state_projection_exposes_source_and_openpose_sidecars_for_diagnostics()
{
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_pose_strip_state_projection_exposes_source_and_openpose_sidecars_for_diagnostics: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;

    let media_artifact = atelier_pg_support::write_native_media_artifact(b"strip-source-image");
    let media = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: media_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: media_artifact.byte_len,
            source_provenance: Some("mt-095 strip source".to_string()),
            artifact_ref: media_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize strip source image");
    let rig = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: Some(media.asset_id),
            source_ref: format!("media://{}", media.asset_id),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 1024,
                height: 1536,
            },
            detector_provider: "mediapipe.tasks-vision.pose".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: Some(format!("media-version://{}/v1", media.asset_id)),
            source_asset_path_ref: Some(format!(
                "source://operator-inbox/{}/strip-source.png",
                Uuid::new_v4()
            )),
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: Some(format!("artifact://atelier/pose/{}", Uuid::new_v4())),
        })
        .await
        .expect("ingest strip source-linked rig");
    let fallback_rig = fresh_rig(&store, character).await;

    let export = generate_posekit_openpose_export(&posekit_export_request(0.0))
        .expect("generate strip OpenPose export");
    for (kind, mime, payload) in [
        (
            PoseSidecarKind::OpenPoseJson,
            "application/json",
            export.openpose_json_bytes.as_slice(),
        ),
        (
            PoseSidecarKind::OpenPosePng,
            "image/png",
            export.openpose_png_bytes.as_slice(),
        ),
        (
            PoseSidecarKind::ConditioningPng,
            "image/png",
            export.openpose_png_bytes.as_slice(),
        ),
    ] {
        let artifact = write_pose_sidecar_artifact(
            payload,
            mime,
            match kind {
                PoseSidecarKind::OpenPoseJson => "posekit-strip-openpose.json",
                PoseSidecarKind::OpenPosePng => "posekit-strip-openpose.png",
                PoseSidecarKind::ConditioningPng => "posekit-strip-conditioning.png",
            },
        );
        store
            .record_pose_sidecar(&NewPoseSidecar {
                rig_id: rig.rig_id,
                kind,
                artifact_ref: artifact.artifact_ref.clone(),
                manifest_ref: artifact_manifest_ref(&artifact.artifact_ref),
                content_hash: artifact.content_hash,
                byte_len: artifact.byte_len,
                mime: mime.to_string(),
                width: export.width,
                height: export.height,
                status: PoseSidecarStatus::Rendered,
                error_message: None,
            })
            .await
            .expect("record strip sidecar");
    }
    let failed_json_artifact = write_pose_sidecar_artifact(
        &export.openpose_json_bytes,
        "application/json",
        "posekit-strip-openpose-failed.json",
    );
    let failed_json_message = "openpose renderer unavailable";
    store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: fallback_rig.rig_id,
            kind: PoseSidecarKind::OpenPoseJson,
            artifact_ref: failed_json_artifact.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&failed_json_artifact.artifact_ref),
            content_hash: failed_json_artifact.content_hash,
            byte_len: failed_json_artifact.byte_len,
            mime: "application/json".to_string(),
            width: export.width,
            height: export.height,
            status: PoseSidecarStatus::Failed,
            error_message: Some(failed_json_message.to_string()),
        })
        .await
        .expect("record failed OpenPose JSON sidecar");

    let source_strip: Vec<PoseSourceImageStripItem> = store
        .pose_source_image_strip_state(character, 25)
        .await
        .expect("read source-image strip state");
    let source_item = source_strip
        .iter()
        .find(|item| item.rig_id == rig.rig_id)
        .expect("source image strip includes the rig source");
    assert_eq!(source_item.source_asset_id, Some(media.asset_id));
    assert_eq!(source_item.source_ref, rig.source_ref);
    assert_eq!(
        source_item.artifact_ref.as_deref(),
        Some(media.artifact_ref.as_str())
    );
    assert_eq!(
        source_item.content_hash.as_deref(),
        Some(media.content_hash.as_str())
    );
    assert_eq!(source_item.mime.as_deref(), Some("image/png"));
    assert_eq!(source_item.byte_len, Some(media.byte_len));
    assert!(source_item.diagnostics_visible);
    assert!(source_item.gallery_visible);
    assert_eq!(
        source_item.jump_target,
        format!("atelier://pose-rig/{}/source", rig.rig_id)
    );
    let fallback_source_item = source_strip
        .iter()
        .find(|item| item.rig_id == fallback_rig.rig_id)
        .expect("source image strip includes source-ref-only fallback rig");
    assert_eq!(fallback_source_item.source_asset_id, None);
    assert_eq!(fallback_source_item.source_ref, fallback_rig.source_ref);
    assert_eq!(fallback_source_item.artifact_ref, None);
    assert_eq!(fallback_source_item.content_hash, None);
    assert_eq!(fallback_source_item.mime, None);
    assert_eq!(fallback_source_item.byte_len, None);
    assert!(fallback_source_item.diagnostics_visible);
    assert!(
        !fallback_source_item.gallery_visible,
        "source-ref-only rigs are diagnostics-visible without becoming gallery media"
    );

    let openpose_strip: Vec<PoseOpenPoseSidecarStripItem> = store
        .pose_openpose_sidecar_strip_state(rig.rig_id)
        .await
        .expect("read OpenPose sidecar strip state");
    assert_eq!(openpose_strip.len(), 2);
    assert_eq!(
        openpose_strip
            .iter()
            .map(|item| item.kind)
            .collect::<Vec<_>>(),
        vec![PoseSidecarKind::OpenPoseJson, PoseSidecarKind::OpenPosePng],
        "OpenPose strip excludes conditioning PNGs"
    );
    assert!(openpose_strip.iter().all(|item| {
        item.rig_id == rig.rig_id
            && item.source_ref == rig.source_ref
            && item
                .artifact_ref
                .starts_with("artifact://.handshake/artifacts/")
            && item.diagnostics_visible
            && !item.gallery_visible
            && item.hidden_reason == "pose_sidecar"
            && item.jump_target == format!("atelier://pose-rig/{}/sidecars", rig.rig_id)
    }));
    assert!(openpose_strip
        .iter()
        .all(|item| item.error_message.is_none()));

    let failed_openpose_strip: Vec<PoseOpenPoseSidecarStripItem> = store
        .pose_openpose_sidecar_strip_state(fallback_rig.rig_id)
        .await
        .expect("read failed OpenPose sidecar strip state");
    assert_eq!(failed_openpose_strip.len(), 1);
    assert_eq!(failed_openpose_strip[0].status, PoseSidecarStatus::Failed);
    assert_eq!(
        failed_openpose_strip[0].error_message.as_deref(),
        Some(failed_json_message),
        "Diagnostics strip state preserves failed sidecar reason"
    );
}

#[tokio::test]
async fn atelier_pose_context_state_switches_without_deleting_rigs_media_or_links() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_pose_context_state_switches_without_deleting_rigs_media_or_links: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let workspace_ref = format!("pose-workspace://{}", Uuid::new_v4());

    let media_artifact = atelier_pg_support::write_native_media_artifact(b"context-source-image");
    let media = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: media_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: media_artifact.byte_len,
            source_provenance: Some("mt-094 pose context source".to_string()),
            artifact_ref: media_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize pose context source media");
    let rig = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: Some(media.asset_id),
            source_ref: format!("media://{}", media.asset_id),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 1024,
                height: 1536,
            },
            detector_provider: "mediapipe.tasks-vision.pose".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: Some(format!("media-version://{}/v1", media.asset_id)),
            source_asset_path_ref: Some(format!(
                "source://operator-inbox/{}/context-source.png",
                Uuid::new_v4()
            )),
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: Some(format!("artifact://atelier/pose/{}", Uuid::new_v4())),
        })
        .await
        .expect("ingest source-linked pose rig");
    let unbound_rig = fresh_rig(&store, character).await;
    let collection = store
        .create_collection(&NewCollection {
            name: format!("pose-context-{}", Uuid::new_v4()),
            notes: "pose context collection".to_string(),
            tags: vec!["pose".to_string()],
            character_internal_id: Some(character),
            sheet_version_id: None,
        })
        .await
        .expect("create pose context collection");
    let other_character = fresh_character(&store).await;
    let other_rig = fresh_rig(&store, other_character).await;

    let blank = store
        .set_pose_context_state(&NewPoseContextState {
            workspace_ref: workspace_ref.clone(),
            kind: PoseContextKind::Blank,
            source_asset_id: None,
            character_internal_id: None,
            collection_id: None,
            selected_rig_id: None,
            requested_by: "mt-094-context".to_string(),
        })
        .await
        .expect("set blank pose context");
    assert_eq!(blank.kind, PoseContextKind::Blank);

    let single_image = store
        .set_pose_context_state(&NewPoseContextState {
            workspace_ref: workspace_ref.clone(),
            kind: PoseContextKind::SingleImage,
            source_asset_id: Some(media.asset_id),
            character_internal_id: None,
            collection_id: None,
            selected_rig_id: Some(rig.rig_id),
            requested_by: "mt-094-context".to_string(),
        })
        .await
        .expect("set single-image pose context");
    assert_eq!(single_image.source_asset_id, Some(media.asset_id));
    assert_eq!(single_image.selected_rig_id, Some(rig.rig_id));

    let character_linked = store
        .set_pose_context_state(&NewPoseContextState {
            workspace_ref: workspace_ref.clone(),
            kind: PoseContextKind::CharacterLinked,
            source_asset_id: None,
            character_internal_id: Some(character),
            collection_id: None,
            selected_rig_id: Some(rig.rig_id),
            requested_by: "mt-094-context".to_string(),
        })
        .await
        .expect("set character-linked pose context");
    assert_eq!(character_linked.character_internal_id, Some(character));

    let collection_linked = store
        .set_pose_context_state(&NewPoseContextState {
            workspace_ref: workspace_ref.clone(),
            kind: PoseContextKind::CollectionLinked,
            source_asset_id: None,
            character_internal_id: Some(character),
            collection_id: Some(collection.collection_id),
            selected_rig_id: Some(rig.rig_id),
            requested_by: "mt-094-context".to_string(),
        })
        .await
        .expect("set collection-linked pose context");
    assert_eq!(
        collection_linked.collection_id,
        Some(collection.collection_id)
    );

    let current = store
        .current_pose_context_state(&workspace_ref)
        .await
        .expect("read current pose context")
        .expect("current context exists");
    assert_eq!(current.context_id, collection_linked.context_id);

    let history = store
        .list_pose_context_history(&workspace_ref)
        .await
        .expect("read pose context history");
    assert_eq!(
        history.iter().map(|state| state.kind).collect::<Vec<_>>(),
        vec![
            PoseContextKind::Blank,
            PoseContextKind::SingleImage,
            PoseContextKind::CharacterLinked,
            PoseContextKind::CollectionLinked
        ],
        "pose context switches are append-only and preserve all prior modes"
    );

    assert!(
        store
            .get_pose_rig(rig.rig_id)
            .await
            .expect("rig still exists after context switches")
            .rig_id
            == rig.rig_id,
        "context switches must not delete the selected rig"
    );
    assert!(
        store
            .get_media_asset_by_hash(&media.content_hash)
            .await
            .expect("media still exists after context switches")
            .is_some(),
        "context switches must not delete source media"
    );
    assert!(
        store
            .get_collection(collection.collection_id)
            .await
            .expect("collection link still exists after context switches")
            .collection_id
            == collection.collection_id,
        "context switches must not delete linked collections"
    );

    let invalid_blank = store
        .set_pose_context_state(&NewPoseContextState {
            workspace_ref: format!("pose-workspace://{}", Uuid::new_v4()),
            kind: PoseContextKind::Blank,
            source_asset_id: Some(media.asset_id),
            character_internal_id: None,
            collection_id: None,
            selected_rig_id: None,
            requested_by: "mt-094-context".to_string(),
        })
        .await
        .expect_err("blank contexts must not carry image links");
    assert!(invalid_blank.to_string().contains("blank"));

    let invalid_unbound_single_image = store
        .set_pose_context_state(&NewPoseContextState {
            workspace_ref: format!("pose-workspace://{}", Uuid::new_v4()),
            kind: PoseContextKind::SingleImage,
            source_asset_id: Some(media.asset_id),
            character_internal_id: None,
            collection_id: None,
            selected_rig_id: Some(unbound_rig.rig_id),
            requested_by: "mt-094-context".to_string(),
        })
        .await
        .expect_err("single-image contexts must not attach source-unlinked rigs");
    assert!(invalid_unbound_single_image
        .to_string()
        .contains("source_asset_id"));

    let invalid_collection_rig = store
        .set_pose_context_state(&NewPoseContextState {
            workspace_ref: format!("pose-workspace://{}", Uuid::new_v4()),
            kind: PoseContextKind::CollectionLinked,
            source_asset_id: None,
            character_internal_id: None,
            collection_id: Some(collection.collection_id),
            selected_rig_id: Some(other_rig.rig_id),
            requested_by: "mt-094-context".to_string(),
        })
        .await
        .expect_err("collection-linked contexts must not attach a rig from another character");
    assert!(invalid_collection_rig.to_string().contains("collection_id"));

    for context_id in [
        blank.context_id,
        single_image.context_id,
        character_linked.context_id,
        collection_linked.context_id,
    ] {
        let event_count = store
            .count_events_for_aggregate(
                pose_event_family::POSE_CONTEXT_STATE_SET,
                "atelier_pose_context_state",
                &context_id.to_string(),
            )
            .await
            .expect("count pose context event");
        assert_eq!(event_count, 1);
    }
}

#[tokio::test]
async fn atelier_pose_multi_rig_workspace_state_tracks_tabs_active_order_dirty_panel() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_pose_multi_rig_workspace_state_tracks_tabs_active_order_dirty_panel: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let rig_a = fresh_rig(&store, character).await;
    let rig_b = fresh_rig(&store, character).await;
    let rig_c = fresh_rig(&store, character).await;
    let workspace_ref = format!("pose-workspace://{}", Uuid::new_v4());
    let session_ref = format!("pose-session://{}", Uuid::new_v4());
    let other_session_ref = format!("pose-session://{}", Uuid::new_v4());

    store
        .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            rig_id: rig_a.rig_id,
            open: true,
            sort_order: 1,
            active: true,
            dirty_calibration: false,
            panel_state: serde_json::json!({"panel": "source", "zoom": 1.0}),
            requested_by: "mt-096-workspace".to_string(),
        })
        .await
        .expect("open rig A as active workspace tab");
    store
        .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            rig_id: rig_b.rig_id,
            open: true,
            sort_order: 0,
            active: false,
            dirty_calibration: true,
            panel_state: serde_json::json!({
                "panel": "calibration",
                "expanded": ["head_pose", "limits"]
            }),
            requested_by: "mt-096-workspace".to_string(),
        })
        .await
        .expect("open dirty rig B workspace tab");
    let rig_c_state = store
        .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            rig_id: rig_c.rig_id,
            open: true,
            sort_order: 2,
            active: true,
            dirty_calibration: false,
            panel_state: serde_json::json!({"panel": "sidecars"}),
            requested_by: "mt-096-initial-c".to_string(),
        })
        .await
        .expect("open rig C as active workspace tab");

    let states: Vec<PoseWorkspaceRigState> = store
        .list_pose_workspace_rig_state(&workspace_ref, &session_ref)
        .await
        .expect("list multi-rig workspace state");
    assert_eq!(
        states.iter().map(|state| state.rig_id).collect::<Vec<_>>(),
        vec![rig_b.rig_id, rig_a.rig_id, rig_c.rig_id],
        "workspace tabs are ordered by explicit sort_order"
    );
    assert_eq!(
        states.iter().filter(|state| state.active).count(),
        1,
        "workspace state keeps one active rig"
    );
    assert!(states
        .iter()
        .any(|state| state.rig_id == rig_c.rig_id && state.active));
    let dirty_b = states
        .iter()
        .find(|state| state.rig_id == rig_b.rig_id)
        .expect("rig B state present");
    assert!(dirty_b.dirty_calibration);
    assert_eq!(
        dirty_b.panel_state,
        serde_json::json!({"panel": "calibration", "expanded": ["head_pose", "limits"]})
    );

    let rig_b_updated = store
        .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            rig_id: rig_b.rig_id,
            open: true,
            sort_order: 3,
            active: true,
            dirty_calibration: false,
            panel_state: serde_json::json!({"panel": "identity", "tab": "profile"}),
            requested_by: "mt-096-switch-active".to_string(),
        })
        .await
        .expect("activate rig B and update panel state");
    assert!(rig_b_updated.active);
    assert!(!rig_b_updated.dirty_calibration);

    let states_after_update = store
        .list_pose_workspace_rig_state(&workspace_ref, &session_ref)
        .await
        .expect("list updated multi-rig workspace state");
    assert_eq!(
        states_after_update
            .iter()
            .map(|state| state.rig_id)
            .collect::<Vec<_>>(),
        vec![rig_a.rig_id, rig_c.rig_id, rig_b.rig_id],
        "updated sort_order is reflected in tab order"
    );
    assert_eq!(
        states_after_update
            .iter()
            .filter(|state| state.active)
            .map(|state| state.rig_id)
            .collect::<Vec<_>>(),
        vec![rig_b.rig_id],
        "activating rig B deactivates prior active rig C"
    );
    assert_eq!(
        states_after_update
            .iter()
            .find(|state| state.rig_id == rig_b.rig_id)
            .expect("rig B updated state present")
            .panel_state,
        serde_json::json!({"panel": "identity", "tab": "profile"})
    );

    let rig_c_deactivation_events = store
        .count_events_for_aggregate(
            pose_event_family::POSE_WORKSPACE_RIG_STATE_SET,
            "atelier_pose_workspace_rig_state",
            &format!("{}:{}:{}", workspace_ref, session_ref, rig_c.rig_id),
        )
        .await
        .expect("count rig C workspace state events");
    assert_eq!(
        rig_c_deactivation_events, 2,
        "activating rig B emits a deactivation event for prior active rig C"
    );
    let rig_b_activation_payload: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_pose_workspace_rig_state'
             AND aggregate_id = $2
           ORDER BY created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(pose_event_family::POSE_WORKSPACE_RIG_STATE_SET)
    .bind(format!(
        "{}:{}:{}",
        workspace_ref, session_ref, rig_b.rig_id
    ))
    .fetch_one(store.pool())
    .await
    .expect("read rig B activation event payload");
    let rig_c_deactivation_payload: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_pose_workspace_rig_state'
             AND aggregate_id = $2
             AND payload->>'reason' = 'deactivated_by_active_rig_switch'
           ORDER BY created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(pose_event_family::POSE_WORKSPACE_RIG_STATE_SET)
    .bind(format!(
        "{}:{}:{}",
        workspace_ref, session_ref, rig_c.rig_id
    ))
    .fetch_one(store.pool())
    .await
    .expect("read rig C deactivation event payload");
    assert!(rig_c_deactivation_payload.get("requested_by").is_none());
    assert_eq!(
        rig_c_deactivation_payload["requested_by_ref"],
        rig_b_activation_payload["requested_by_ref"],
        "deactivation event is attributed to the actor that switched active rigs"
    );

    let duplicate_order = store
        .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            rig_id: rig_c.rig_id,
            open: true,
            sort_order: 1,
            active: false,
            dirty_calibration: false,
            panel_state: serde_json::json!({"panel": "sidecars"}),
            requested_by: "mt-096-workspace".to_string(),
        })
        .await
        .expect_err("open tabs must not share sort_order");
    assert!(duplicate_order.to_string().contains("sort_order"));

    store
        .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            rig_id: rig_a.rig_id,
            open: false,
            sort_order: 0,
            active: false,
            dirty_calibration: false,
            panel_state: serde_json::json!({"panel": "closed"}),
            requested_by: "mt-096-workspace".to_string(),
        })
        .await
        .expect("close rig A workspace tab");
    let open_after_close = store
        .list_pose_workspace_rig_state(&workspace_ref, &session_ref)
        .await
        .expect("list open tabs after close");
    assert_eq!(
        open_after_close
            .iter()
            .map(|state| state.rig_id)
            .collect::<Vec<_>>(),
        vec![rig_c.rig_id, rig_b.rig_id],
        "closed rigs no longer appear in the open-rig tab list, while inactive open tabs remain"
    );

    store
        .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
            workspace_ref: workspace_ref.clone(),
            session_ref: other_session_ref.clone(),
            rig_id: rig_a.rig_id,
            open: true,
            sort_order: 0,
            active: true,
            dirty_calibration: true,
            panel_state: serde_json::json!({"panel": "other-session"}),
            requested_by: "mt-096-workspace".to_string(),
        })
        .await
        .expect("same workspace can hold independent state in another session");
    let other_session_states = store
        .list_pose_workspace_rig_state(&workspace_ref, &other_session_ref)
        .await
        .expect("list other session workspace state");
    assert_eq!(other_session_states.len(), 1);
    assert_eq!(other_session_states[0].rig_id, rig_a.rig_id);
    assert!(other_session_states[0].dirty_calibration);

    let invalid_workspace = store
        .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
            workspace_ref: ".GOV/pose-workspace".to_string(),
            session_ref: session_ref.clone(),
            rig_id: rig_a.rig_id,
            open: true,
            sort_order: 4,
            active: false,
            dirty_calibration: false,
            panel_state: serde_json::json!({"panel": "source"}),
            requested_by: "mt-096-workspace".to_string(),
        })
        .await
        .expect_err(".GOV workspace refs are rejected");
    assert!(invalid_workspace.to_string().contains("workspace_ref"));

    let invalid_panel = store
        .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            rig_id: rig_a.rig_id,
            open: true,
            sort_order: 4,
            active: false,
            dirty_calibration: false,
            panel_state: serde_json::json!("source"),
            requested_by: "mt-096-workspace".to_string(),
        })
        .await
        .expect_err("panel_state must be a structured object");
    assert!(invalid_panel.to_string().contains("panel_state"));

    let event_count = store
        .count_events_for_aggregate(
            pose_event_family::POSE_WORKSPACE_RIG_STATE_SET,
            "atelier_pose_workspace_rig_state",
            &format!(
                "{}:{}:{}",
                rig_c_state.workspace_ref, rig_c_state.session_ref, rig_c_state.rig_id
            ),
        )
        .await
        .expect("count workspace rig state event");
    assert_eq!(
        event_count, 2,
        "rig C has its initial active-state event plus the emitted deactivation event"
    );
}

#[tokio::test]
async fn atelier_pose_workspace_route_and_keyboard_navigation_are_durable() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_pose_workspace_route_and_keyboard_navigation_are_durable: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let rig_a = fresh_rig(&store, character).await;
    let rig_b = fresh_rig(&store, character).await;
    let rig_c = fresh_rig(&store, character).await;
    let workspace_ref = format!("pose-workspace://{}", Uuid::new_v4());
    let session_ref = format!("pose-session://{}", Uuid::new_v4());

    for (rig_id, sort_order, active, panel) in [
        (
            rig_a.rig_id,
            1,
            true,
            serde_json::json!({"panel": "source"}),
        ),
        (
            rig_b.rig_id,
            0,
            false,
            serde_json::json!({"panel": "identity"}),
        ),
        (
            rig_c.rig_id,
            2,
            false,
            serde_json::json!({"panel": "sidecars"}),
        ),
    ] {
        store
            .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
                workspace_ref: workspace_ref.clone(),
                session_ref: session_ref.clone(),
                rig_id,
                open: true,
                sort_order,
                active,
                dirty_calibration: false,
                panel_state: panel,
                requested_by: "mt-097-seed".to_string(),
            })
            .await
            .expect("seed pose workspace rig state");
    }

    let direct_route = store
        .route_pose_workspace_to_rig(&NewPoseWorkspaceRouteTarget {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            rig_id: rig_a.rig_id,
            panel_id: "calibration".to_string(),
            requested_by: "mt-097-route-direct".to_string(),
        })
        .await
        .expect("resolve direct pose workspace route");
    assert_eq!(direct_route.rig_id, rig_a.rig_id);
    assert_eq!(direct_route.panel_id, "calibration");
    assert_eq!(direct_route.active_sort_order, 1);
    assert_eq!(direct_route.open_rig_count, 3);
    assert!(
        direct_route
            .route_ref
            .starts_with("pose-workspace-route://"),
        "route_ref must be a portable product route, got {}",
        direct_route.route_ref
    );

    let next_route = store
        .apply_pose_workspace_keyboard_action(&PoseWorkspaceKeyboardActionRequest {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            action: PoseWorkspaceKeyboardAction::ActivateNextRig,
            panel_id: "calibration".to_string(),
            requested_by: "mt-097-keyboard-next".to_string(),
        })
        .await
        .expect("keyboard next activates the next open rig by sort order");
    assert_eq!(next_route.rig_id, rig_c.rig_id);
    assert_eq!(
        next_route.keyboard_action,
        Some(PoseWorkspaceKeyboardAction::ActivateNextRig)
    );

    let previous_route = store
        .apply_pose_workspace_keyboard_action(&PoseWorkspaceKeyboardActionRequest {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            action: PoseWorkspaceKeyboardAction::ActivatePreviousRig,
            panel_id: "calibration".to_string(),
            requested_by: "mt-097-keyboard-previous".to_string(),
        })
        .await
        .expect("keyboard previous activates the previous open rig by sort order");
    assert_eq!(previous_route.rig_id, rig_a.rig_id);
    assert_eq!(
        previous_route.keyboard_action,
        Some(PoseWorkspaceKeyboardAction::ActivatePreviousRig)
    );

    let identity_route = store
        .route_pose_workspace_to_rig(&NewPoseWorkspaceRouteTarget {
            workspace_ref: workspace_ref.clone(),
            session_ref: session_ref.clone(),
            rig_id: rig_b.rig_id,
            panel_id: "identity".to_string(),
            requested_by: "mt-097-route-identity".to_string(),
        })
        .await
        .expect("direct route activates requested rig");
    assert_eq!(identity_route.rig_id, rig_b.rig_id);
    assert_eq!(identity_route.panel_id, "identity");

    let active_after_routes = store
        .list_pose_workspace_rig_state(&workspace_ref, &session_ref)
        .await
        .expect("list state after routes");
    assert_eq!(
        active_after_routes
            .iter()
            .filter(|state| state.active)
            .map(|state| state.rig_id)
            .collect::<Vec<_>>(),
        vec![rig_b.rig_id],
        "route and keyboard APIs update the same durable active-rig state"
    );

    let invalid_panel = store
        .route_pose_workspace_to_rig(&NewPoseWorkspaceRouteTarget {
            workspace_ref,
            session_ref,
            rig_id: rig_b.rig_id,
            panel_id: "identity/details".to_string(),
            requested_by: "mt-097-invalid-panel".to_string(),
        })
        .await
        .expect_err("route panel ids must be stable tokens");
    assert!(invalid_panel.to_string().contains("panel_id"));
}

#[tokio::test]
async fn atelier_pose_head_pose_upsert_and_limits() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP atelier_pose_head_pose_upsert_and_limits: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;

    // --- record a head pose; the quaternion is normalized on the way in ---
    let head = store
        .record_head_pose(rig.rig_id, 30.0, -20.0, 10.0, [0.0, 0.0, 0.0, 2.0])
        .await
        .expect("record head pose");
    assert_eq!(head.rig_id, rig.rig_id);
    assert_eq!(head.yaw_deg, 30.0);
    assert_eq!(head.pitch_deg, -20.0);
    assert_eq!(head.roll_deg, 10.0);
    // [0,0,0,2] normalizes to a unit quaternion [0,0,0,1].
    let len: f64 = head.quaternion.iter().map(|c| c * c).sum::<f64>().sqrt();
    assert!(
        (len - 1.0).abs() < 1e-9,
        "stored quaternion is normalized to unit length, got {len}"
    );
    assert!(
        (head.quaternion[3] - 1.0).abs() < 1e-9,
        "[0,0,0,2] normalizes to w=1"
    );

    // --- IDEMPOTENCY / one-per-rig: re-recording UPDATES in place (no second row) ---
    let head2 = store
        .record_head_pose(rig.rig_id, -45.0, 5.0, -15.0, [0.0, 1.0, 0.0, 0.0])
        .await
        .expect("re-record head pose");
    assert_eq!(head2.rig_id, rig.rig_id, "head pose stays keyed on the rig");
    let got = store
        .get_head_pose(rig.rig_id)
        .await
        .expect("get head pose")
        .expect("head pose present");
    assert_eq!(got.yaw_deg, -45.0, "upsert replaced the yaw in place");
    assert_eq!(
        got.quaternion, head2.quaternion,
        "upsert replaced the quaternion"
    );

    // --- MT-089: YXZ Euler input computes the quaternion instead of requiring callers to supply it ---
    let euler_head = store
        .record_head_pose_from_yxz_euler(rig.rig_id, 90.0, 0.0, 0.0)
        .await
        .expect("record head pose from YXZ Euler");
    let yaw_half = std::f64::consts::FRAC_1_SQRT_2;
    assert_eq!(euler_head.yaw_deg, 90.0);
    assert_eq!(euler_head.pitch_deg, 0.0);
    assert_eq!(euler_head.roll_deg, 0.0);
    assert!(
        euler_head.quaternion[0].abs() < 1e-9
            && (euler_head.quaternion[1] - yaw_half).abs() < 1e-9
            && euler_head.quaternion[2].abs() < 1e-9
            && (euler_head.quaternion[3] - yaw_half).abs() < 1e-9,
        "YXZ Euler yaw-only conversion should produce [0, sin(45deg), 0, cos(45deg)], got {:?}",
        euler_head.quaternion
    );

    // --- MT-089: legacy yaw-only import path preserves yaw and derives the matching quaternion ---
    let legacy_yaw = store
        .import_legacy_yaw_head_pose(rig.rig_id, -45.0)
        .await
        .expect("import legacy yaw head pose");
    let yaw_22_5 = (22.5_f64).to_radians().sin();
    let cos_22_5 = (22.5_f64).to_radians().cos();
    assert_eq!(legacy_yaw.yaw_deg, -45.0);
    assert_eq!(legacy_yaw.pitch_deg, 0.0);
    assert_eq!(legacy_yaw.roll_deg, 0.0);
    assert!(
        legacy_yaw.quaternion[0].abs() < 1e-9
            && (legacy_yaw.quaternion[1] + yaw_22_5).abs() < 1e-9
            && legacy_yaw.quaternion[2].abs() < 1e-9
            && (legacy_yaw.quaternion[3] - cos_22_5).abs() < 1e-9,
        "legacy yaw import should compute a yaw-only quaternion, got {:?}",
        legacy_yaw.quaternion
    );

    // --- INVARIANT: legacy source degree limits enforced (yaw +-90 / pitch +-75 / roll +-45) ---
    assert!(
        store
            .record_head_pose(rig.rig_id, 120.0, 0.0, 0.0, [0.0, 0.0, 0.0, 1.0])
            .await
            .is_err(),
        "yaw beyond +-90 is rejected"
    );
    assert!(
        store
            .record_head_pose(rig.rig_id, 0.0, 0.0, 0.0, [0.0, 0.0, 0.0, 0.0])
            .await
            .is_err(),
        "degenerate (zero-length) quaternion is rejected"
    );

    // --- EVENT EMISSION: exactly four successful records emitted events ---
    let event_count = store
        .count_events_for_aggregate(
            pose_event_family::POSE_HEAD_POSE_RECORDED,
            "atelier_pose_rig",
            &rig.rig_id.to_string(),
        )
        .await
        .expect("count run-scoped head-pose events");
    assert_eq!(
        event_count, 4,
        "four successful head-pose records each emit one event; rejected ones emit none"
    );
}

#[tokio::test]
async fn atelier_pose_calibration_blocked_preservation() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP atelier_pose_calibration_blocked_preservation: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;

    // --- INVARIANT: an Unresolved calibration WITHOUT a block_reason is rejected
    //     (the BLOCKED status must always be auditable, never silently empty) ---
    assert!(
        store
            .set_calibration(rig.rig_id, CalibrationState::Unresolved, None)
            .await
            .is_err(),
        "unresolved calibration must record a block_reason"
    );
    assert!(
        store
            .set_calibration(rig.rig_id, CalibrationState::Unresolved, Some("  "))
            .await
            .is_err(),
        "blank block_reason does not satisfy the BLOCKED audit requirement"
    );

    // --- set BLOCKED/unresolved calibration WITH a block reason (the default,
    //     spec Calibration Panel 10.10.4.1.9 is not implementable so values are
    //     never fabricated) ---
    let reason = "spec Calibration Panel 10.10.4.1.9 not yet implementable";
    let cal = store
        .set_calibration(rig.rig_id, CalibrationState::Unresolved, Some(reason))
        .await
        .expect("set unresolved calibration");
    assert_eq!(cal.rig_id, rig.rig_id);
    assert_eq!(
        cal.state,
        CalibrationState::Unresolved,
        "calibration is preserved as BLOCKED/unresolved, not fabricated"
    );
    assert_eq!(
        cal.block_reason.as_deref(),
        Some(reason),
        "the block reason is preserved verbatim for audit"
    );

    // --- IDEMPOTENCY / one-per-rig: re-setting upserts in place on rig_id ---
    let reason2 = "still blocked pending Workflow-Engine calibration job";
    let cal2 = store
        .set_calibration(rig.rig_id, CalibrationState::Unresolved, Some(reason2))
        .await
        .expect("re-set calibration");
    assert_eq!(
        cal2.rig_id, rig.rig_id,
        "calibration stays keyed on the rig"
    );
    let got = store
        .get_calibration(rig.rig_id)
        .await
        .expect("get calibration")
        .expect("calibration present");
    assert_eq!(
        got.block_reason.as_deref(),
        Some(reason2),
        "upsert replaced the block_reason in place (single row per rig)"
    );
    assert_eq!(got.state, CalibrationState::Unresolved);

    // --- MT-090: typed calibration data round-trips instead of collapsing to state+reason ---
    let head_pose_ref = format!("atelier://pose-rig/{}/head-pose", rig.rig_id);
    let history_ref = format!("calibration-history://{}", Uuid::new_v4());
    let marker_visibility = CalibrationMarkerVisibility {
        body: true,
        face: true,
        left_hand: true,
        right_hand: false,
    };
    let marker_colors = CalibrationMarkerColors {
        body: "#ff3366".to_string(),
        face: "#33ccff".to_string(),
        left_hand: "#77dd77".to_string(),
        right_hand: "#999999".to_string(),
    };
    let hand_rows = vec![
        CalibrationHandRow {
            hand: CalibrationHandKind::Left,
            visible: true,
            marker_count: HAND_KEYPOINT_COUNT as i32,
            confidence_available: true,
        },
        CalibrationHandRow {
            hand: CalibrationHandKind::Right,
            visible: false,
            marker_count: HAND_KEYPOINT_COUNT as i32,
            confidence_available: false,
        },
    ];
    let typed = store
        .set_pose_calibration(&NewPoseCalibration {
            rig_id: rig.rig_id,
            state: CalibrationState::Resolved,
            block_reason: None,
            head_pose_ref: Some(head_pose_ref.clone()),
            marker_visibility: marker_visibility.clone(),
            marker_colors: marker_colors.clone(),
            hand_rows: hand_rows.clone(),
            history_refs: vec![history_ref.clone()],
        })
        .await
        .expect("set typed pose calibration");
    assert_eq!(typed.state, CalibrationState::Resolved);
    assert_eq!(typed.block_reason, None);
    assert_eq!(typed.head_pose_ref.as_deref(), Some(head_pose_ref.as_str()));
    assert_eq!(typed.marker_visibility, marker_visibility);
    assert_eq!(typed.marker_colors, marker_colors);
    assert_eq!(typed.hand_rows, hand_rows);
    assert_eq!(typed.history_refs, vec![history_ref.clone()]);

    let fetched_typed = store
        .get_calibration(rig.rig_id)
        .await
        .expect("get typed calibration")
        .expect("typed calibration present");
    assert_eq!(
        fetched_typed, typed,
        "typed calibration fields round-trip through Postgres"
    );

    let bad_history_ref = store
        .set_pose_calibration(&NewPoseCalibration {
            rig_id: rig.rig_id,
            state: CalibrationState::Resolved,
            block_reason: None,
            head_pose_ref: Some(head_pose_ref.clone()),
            marker_visibility: CalibrationMarkerVisibility::default(),
            marker_colors: CalibrationMarkerColors::default(),
            hand_rows: Vec::new(),
            history_refs: vec!["C:\\operator\\pose-calibration.json".to_string()],
        })
        .await;
    assert!(
        bad_history_ref.is_err(),
        "machine-local calibration history refs must be rejected"
    );

    // --- EVENT EMISSION: three successful sets emitted events; rejected sets did not ---
    let event_count = store
        .count_events_for_aggregate(
            pose_event_family::POSE_CALIBRATION_SET,
            "atelier_pose_rig",
            &rig.rig_id.to_string(),
        )
        .await
        .expect("count run-scoped calibration events");
    assert_eq!(
        event_count, 3,
        "three successful calibration sets each emit one event; rejected sets emit none"
    );
}

#[tokio::test]
async fn atelier_identity_profile_append_only_seq_and_redaction() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_identity_profile_append_only_seq_and_redaction: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;

    // --- append the first identity profile; seq starts at 1 for a new character ---
    let p1 = store
        .append_identity_profile(&NewIdentityProfile {
            character_internal_id: character,
            kind: IdentityProfileKind::Face,
            name: "Front portrait identity".to_string(),
            description: "Neutral front portrait used for face identity matching".to_string(),
            reference_asset_id: None,
            reference_ref: format!("portrait://{}", Uuid::new_v4()),
            source_ref: Some(format!("source://identity/{}", Uuid::new_v4())),
            crop_ref: Some(format!("crop://identity/{}", Uuid::new_v4())),
            artifact_ref: Some(format!("artifact://identity-profile/{}", Uuid::new_v4())),
            provenance: "source: operator upload".to_string(),
        })
        .await
        .expect("append first identity profile");
    assert_eq!(p1.seq, 1, "first profile for a fresh character is seq 1");
    assert_eq!(p1.version, 1, "new identity profiles start at version 1");
    assert_eq!(p1.kind, IdentityProfileKind::Face);
    assert_eq!(p1.name, "Front portrait identity");
    assert_eq!(
        p1.description,
        "Neutral front portrait used for face identity matching"
    );
    assert!(p1.source_ref.as_deref().unwrap().starts_with("source://"));
    assert!(p1.crop_ref.as_deref().unwrap().starts_with("crop://"));
    assert!(p1
        .artifact_ref
        .as_deref()
        .unwrap()
        .starts_with("artifact://"));
    assert!(
        p1.updated_at_utc >= p1.created_at_utc,
        "identity profile exposes update timestamp"
    );

    // --- SECRET REDACTION: secret-looking provenance lines are masked before
    //     storage (no raw cookies/tokens ever persisted) ---
    let raw_secret = "civitai_token=supersecretvalue123";
    let p2 = store
        .append_identity_profile(&NewIdentityProfile {
            character_internal_id: character,
            kind: IdentityProfileKind::Reference,
            name: "Full body identity reference".to_string(),
            description: "Full-body pose and wardrobe reference".to_string(),
            reference_asset_id: None,
            reference_ref: format!("reference://{}", Uuid::new_v4()),
            source_ref: Some(format!("source://identity/{}", Uuid::new_v4())),
            crop_ref: None,
            artifact_ref: Some(format!("artifact://identity-profile/{}", Uuid::new_v4())),
            provenance: format!("source: civitai\n{raw_secret}\nnote: full-body ref"),
        })
        .await
        .expect("append second identity profile");
    // --- INVARIANT: append-only seq monotonicity (strictly increments) ---
    assert_eq!(
        p2.seq, 2,
        "second profile increments seq to 2 (append-only)"
    );
    assert!(
        !p2.provenance.contains("supersecretvalue123"),
        "the raw secret value must NOT be stored: {}",
        p2.provenance
    );
    assert!(
        p2.provenance.contains("[REDACTED]"),
        "the secret line is masked with the redaction placeholder"
    );
    assert!(
        p2.provenance.contains("source: civitai"),
        "non-secret provenance lines are preserved"
    );

    // --- query round-trip: list is ascending seq order, redaction is persisted ---
    let listed = store
        .list_identity_profiles(character, None)
        .await
        .expect("list identity profiles");
    assert_eq!(listed.len(), 2, "both appended profiles are listed");
    assert_eq!(listed[0].seq, 1, "list is ascending by seq");
    assert_eq!(listed[1].seq, 2);
    assert!(
        !listed[1].provenance.contains("supersecretvalue123"),
        "redaction is durable in the stored/queried provenance"
    );

    let fetched_p1 = store
        .get_identity_profile(p1.profile_id)
        .await
        .expect("get identity profile")
        .expect("identity profile is present");
    assert_eq!(fetched_p1.profile_id, p1.profile_id);
    assert_eq!(fetched_p1.name, p1.name);

    let updated_p1 = store
        .update_identity_profile(&UpdateIdentityProfile {
            profile_id: p1.profile_id,
            name: "Front portrait identity v2".to_string(),
            description: "Updated crop and artifact refs for the face identity record".to_string(),
            source_ref: Some(format!("source://identity/{}", Uuid::new_v4())),
            crop_ref: Some(format!("crop://identity/{}", Uuid::new_v4())),
            artifact_ref: Some(format!("artifact://identity-profile/{}", Uuid::new_v4())),
            requested_by: "mt-098-update".to_string(),
        })
        .await
        .expect("update identity profile record");
    assert_eq!(updated_p1.version, 2);
    assert_eq!(updated_p1.seq, 1, "update preserves append sequence");
    assert_eq!(updated_p1.name, "Front portrait identity v2");
    assert!(
        updated_p1.updated_at_utc >= updated_p1.created_at_utc,
        "update keeps timestamps queryable"
    );

    // --- kind filter + latest helper round-trip ---
    let faces = store
        .list_identity_profiles(character, Some(IdentityProfileKind::Face))
        .await
        .expect("list face profiles");
    assert_eq!(
        faces.len(),
        1,
        "kind filter narrows to the single Face profile"
    );
    assert_eq!(faces[0].profile_id, p1.profile_id);
    let latest_ref = store
        .latest_identity_profile(character, IdentityProfileKind::Reference)
        .await
        .expect("latest reference profile")
        .expect("a reference profile exists");
    assert_eq!(
        latest_ref.profile_id, p2.profile_id,
        "latest Reference profile is the highest-seq one"
    );

    // --- INVARIANT: empty reference_ref is rejected ---
    assert!(
        store
            .append_identity_profile(&NewIdentityProfile {
                character_internal_id: character,
                kind: IdentityProfileKind::Face,
                name: "Bad empty ref".to_string(),
                description: "invalid".to_string(),
                reference_asset_id: None,
                reference_ref: "   ".to_string(),
                source_ref: None,
                crop_ref: None,
                artifact_ref: None,
                provenance: "x".to_string(),
            })
            .await
            .is_err(),
        "empty reference_ref is rejected"
    );
    assert!(
        store
            .append_identity_profile(&NewIdentityProfile {
                character_internal_id: character,
                kind: IdentityProfileKind::Face,
                name: "Bad local ref".to_string(),
                description: "invalid".to_string(),
                reference_asset_id: None,
                reference_ref: "file:///tmp/ref.png".to_string(),
                source_ref: None,
                crop_ref: None,
                artifact_ref: None,
                provenance: "x".to_string(),
            })
            .await
            .is_err(),
        "machine-local identity reference_ref is rejected"
    );
    assert!(
        store
            .update_identity_profile(&UpdateIdentityProfile {
                profile_id: p1.profile_id,
                name: "Bad crop".to_string(),
                description: "invalid".to_string(),
                source_ref: None,
                crop_ref: Some("C:\\operator\\face-crop.png".to_string()),
                artifact_ref: None,
                requested_by: "mt-098-invalid-update".to_string(),
            })
            .await
            .is_err(),
        "machine-local crop refs are rejected"
    );

    assert!(
        store
            .delete_identity_profile(p2.profile_id, "mt-098-delete")
            .await
            .expect("delete identity profile"),
        "delete reports that an active profile was soft-deleted"
    );
    assert!(
        store
            .get_identity_profile(p2.profile_id)
            .await
            .expect("get deleted profile")
            .is_none(),
        "deleted identity profiles are hidden from CRUD reads"
    );
    let listed_after_delete = store
        .list_identity_profiles(character, None)
        .await
        .expect("list identity profiles after delete");
    assert_eq!(
        listed_after_delete
            .iter()
            .map(|profile| profile.profile_id)
            .collect::<Vec<_>>(),
        vec![p1.profile_id],
        "deleted identity profiles are hidden from list projections"
    );

    // --- EVENT EMISSION: append, update, and delete mutations emitted events ---
    let p1_event_count = store
        .count_events_for_aggregate(
            pose_event_family::IDENTITY_PROFILE_APPENDED,
            "atelier_identity_profile",
            &p1.profile_id.to_string(),
        )
        .await
        .expect("count first identity-profile event");
    let p2_event_count = store
        .count_events_for_aggregate(
            pose_event_family::IDENTITY_PROFILE_APPENDED,
            "atelier_identity_profile",
            &p2.profile_id.to_string(),
        )
        .await
        .expect("count second identity-profile event");
    assert_eq!(
        p1_event_count + p2_event_count,
        4,
        "identity-profile append/update/delete mutations each emit one event; rejected mutations emit none"
    );
}

#[tokio::test]
async fn atelier_identity_crop_artifact_links_profile_version_and_manifest() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_identity_crop_artifact_links_profile_version_and_manifest: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;

    let profile = store
        .append_identity_profile(&NewIdentityProfile {
            character_internal_id: character,
            kind: IdentityProfileKind::Face,
            name: "MT-099 face identity".to_string(),
            description: "Identity profile that owns a face crop artifact".to_string(),
            reference_asset_id: None,
            reference_ref: format!("reference://identity/{}", Uuid::new_v4()),
            source_ref: Some(format!("source://identity/{}", Uuid::new_v4())),
            crop_ref: None,
            artifact_ref: None,
            provenance: "operator-selected face reference".to_string(),
        })
        .await
        .expect("append identity profile for crop");
    let profile = store
        .update_identity_profile(&UpdateIdentityProfile {
            profile_id: profile.profile_id,
            name: "MT-099 face identity v2".to_string(),
            description: "Versioned identity profile that owns a crop artifact".to_string(),
            source_ref: Some(format!("source://identity/{}", Uuid::new_v4())),
            crop_ref: None,
            artifact_ref: None,
            requested_by: "mt-099-profile-update".to_string(),
        })
        .await
        .expect("bump profile version before crop registration");
    assert_eq!(profile.version, 2);

    let crop_payload = atelier_pg_support::write_native_media_artifact(b"mt-099-face-crop-512");
    let crop = store
        .record_identity_crop_artifact(&NewIdentityCropArtifact {
            profile_id: profile.profile_id,
            source_ref: format!("source://identity-crop/{}", Uuid::new_v4()),
            crop_box: IdentityCropBox {
                x: 144,
                y: 96,
                width: 512,
                height: 512,
            },
            landmarks: vec![
                IdentityCropLandmark {
                    name: "left_eye".to_string(),
                    x: 210.5,
                    y: 224.25,
                    confidence: Some(0.98),
                },
                IdentityCropLandmark {
                    name: "right_eye".to_string(),
                    x: 304.75,
                    y: 223.5,
                    confidence: Some(0.97),
                },
                IdentityCropLandmark {
                    name: "mouth_center".to_string(),
                    x: 257.0,
                    y: 350.0,
                    confidence: None,
                },
            ],
            artifact_ref: crop_payload.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&crop_payload.artifact_ref),
            content_hash: crop_payload.content_hash.clone(),
            byte_len: crop_payload.byte_len,
            mime: "image/png".to_string(),
            width: 512,
            height: 512,
            created_by: "mt-099-crop".to_string(),
        })
        .await
        .expect("record identity crop artifact");

    assert_eq!(crop.profile_id, profile.profile_id);
    assert_eq!(
        crop.profile_version, profile.version,
        "crop record links to the identity profile version active at registration"
    );
    assert_eq!(crop.crop_box.width, 512);
    assert_eq!(crop.crop_box.height, 512);
    assert_eq!(crop.width, 512);
    assert_eq!(crop.height, 512);
    assert_eq!(crop.artifact_ref, crop_payload.artifact_ref);
    assert_eq!(crop.manifest_ref, artifact_manifest_ref(&crop.artifact_ref));
    assert_eq!(crop.content_hash, crop_payload.content_hash);
    assert_eq!(
        crop.manifest["schema"],
        "hsk.atelier.identity_crop_artifact_manifest@1"
    );
    assert_eq!(crop.manifest["crop_id"], crop.crop_id.to_string());
    assert_eq!(crop.manifest["profile_id"], profile.profile_id.to_string());
    assert_eq!(
        crop.manifest["profile_version"],
        serde_json::json!(profile.version)
    );
    assert_eq!(crop.manifest["source_ref"], crop.source_ref);
    assert_eq!(crop.manifest["crop_box"]["width"], 512);
    assert_eq!(crop.manifest["crop_box"]["height"], 512);
    assert_eq!(crop.manifest["landmarks"][0]["name"], "left_eye");
    assert_eq!(
        crop.manifest["artifact_store"]["handle"],
        crop_payload.artifact_ref
    );
    assert_eq!(
        crop.manifest["artifact_store"]["manifest"],
        artifact_manifest_ref(&crop_payload.artifact_ref)
    );
    assert_eq!(
        crop.manifest["artifact_store"]["content_hash"],
        crop_payload.content_hash
    );

    let fetched = store
        .get_identity_crop_artifact(crop.crop_id)
        .await
        .expect("get identity crop artifact")
        .expect("crop artifact present");
    assert_eq!(fetched, crop);
    let listed = store
        .list_identity_crop_artifacts(profile.profile_id)
        .await
        .expect("list identity crop artifacts");
    assert_eq!(listed, vec![crop.clone()]);

    let duplicate = store
        .record_identity_crop_artifact(&NewIdentityCropArtifact {
            profile_id: profile.profile_id,
            source_ref: crop.source_ref.clone(),
            crop_box: crop.crop_box.clone(),
            landmarks: crop.landmarks.clone(),
            artifact_ref: crop.artifact_ref.clone(),
            manifest_ref: crop.manifest_ref.clone(),
            content_hash: crop.content_hash.clone(),
            byte_len: crop.byte_len,
            mime: crop.mime.clone(),
            width: 512,
            height: 512,
            created_by: "mt-099-crop-duplicate".to_string(),
        })
        .await
        .expect("duplicate crop registration is idempotent");
    assert_eq!(
        duplicate.crop_id, crop.crop_id,
        "same profile version + content hash returns existing crop record"
    );

    assert!(
        store
            .record_identity_crop_artifact(&NewIdentityCropArtifact {
                profile_id: profile.profile_id,
                source_ref: "C:\\operator\\face.png".to_string(),
                crop_box: crop.crop_box.clone(),
                landmarks: crop.landmarks.clone(),
                artifact_ref: crop.artifact_ref.clone(),
                manifest_ref: crop.manifest_ref.clone(),
                content_hash: crop.content_hash.clone(),
                byte_len: crop.byte_len,
                mime: crop.mime.clone(),
                width: 512,
                height: 512,
                created_by: "mt-099-bad-source".to_string(),
            })
            .await
            .is_err(),
        "machine-local source refs are rejected"
    );
    assert!(
        store
            .record_identity_crop_artifact(&NewIdentityCropArtifact {
                profile_id: profile.profile_id,
                source_ref: crop.source_ref.clone(),
                crop_box: IdentityCropBox {
                    x: 0,
                    y: 0,
                    width: 511,
                    height: 512,
                },
                landmarks: crop.landmarks.clone(),
                artifact_ref: crop.artifact_ref.clone(),
                manifest_ref: crop.manifest_ref.clone(),
                content_hash: crop.content_hash.clone(),
                byte_len: crop.byte_len,
                mime: crop.mime.clone(),
                width: 511,
                height: 512,
                created_by: "mt-099-bad-size".to_string(),
            })
            .await
            .is_err(),
        "identity crop artifacts must be exactly 512x512"
    );
    assert!(
        store
            .record_identity_crop_artifact(&NewIdentityCropArtifact {
                profile_id: profile.profile_id,
                source_ref: crop.source_ref.clone(),
                crop_box: crop.crop_box.clone(),
                landmarks: crop.landmarks.clone(),
                artifact_ref: ".GOV/identity-crop.png".to_string(),
                manifest_ref: crop.manifest_ref.clone(),
                content_hash: crop.content_hash.clone(),
                byte_len: crop.byte_len,
                mime: crop.mime.clone(),
                width: 512,
                height: 512,
                created_by: "mt-099-bad-artifact".to_string(),
            })
            .await
            .is_err(),
        ".GOV artifact refs are rejected"
    );

    let event_count = store
        .count_events_for_aggregate(
            pose_event_family::IDENTITY_CROP_ARTIFACT_RECORDED,
            "atelier_identity_crop_artifact",
            &crop.crop_id.to_string(),
        )
        .await
        .expect("count identity crop event");
    assert_eq!(
        event_count, 1,
        "only the first successful crop registration emits an event"
    );
}

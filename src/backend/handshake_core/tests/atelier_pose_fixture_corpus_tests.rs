//! WP-KERNEL-005 MT-113 atelier POSE fixture corpus: portable JSON fixtures
//! round-tripped through the real `AtelierStore` pose APIs against live
//! PostgreSQL (no SQLite, ever, no CKC namespace).
//!
//! Each fixture under `tests/fixtures/atelier_pose/` is a portable, non-SQLite
//! JSON document covering one pose area: rig ingest + typed OpenPose/conditioning
//! sidecars, append-only context/workspace state, BLOCKED calibration + head
//! pose, and append-only versioned identity profiles. The loader reads each
//! fixture via `include_str!`, deserializes it with serde, makes every public
//! id / source ref run-unique with a per-run suffix, persists it through the
//! real pose store APIs (`ingest_pose_rig`, `record_pose_sidecar`,
//! `set_pose_context_state`, `upsert_pose_workspace_rig_state`,
//! `record_head_pose`, `set_pose_calibration`, `append_identity_profile`),
//! reloads, and asserts round-trip fidelity. Every assertion is run-scoped: the
//! test owns its own character / rig / workspace / profile ids and never uses a
//! global table count. Gated on `atelier_pg_support::database_url()`.
//!
//! Run:
//!   cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --features test-utils --test atelier_pose_fixture_corpus_tests \
//!     mt113_pose_fixture_corpus_round_trips \
//!     --target-dir ../Handshake_Artifacts/handshake-cargo-target

mod atelier_pg_support;

use handshake_core::atelier::collections::NewCollection;
use handshake_core::atelier::pose::{
    CalibrationHandKind, CalibrationHandRow, CalibrationMarkerColors, CalibrationMarkerVisibility,
    CalibrationState, CanvasSize, DetectorStatus, IdentityProfileKind, NewIdentityProfile,
    NewPoseCalibration, NewPoseContextState, NewPoseRig, NewPoseSidecar,
    NewPoseWorkspaceRigState, PoseContextKind, PoseRig, PoseSidecarKind, PoseSidecarStatus,
    BODY_KEYPOINT_COUNT, FACE_KEYPOINT_COUNT, HAND_KEYPOINT_COUNT,
};
use handshake_core::atelier::{AtelierStore, NewCharacter, NewMediaAsset};
use serde::Deserialize;
use uuid::Uuid;

/// Connect + ensure schema, the shared preamble every fixture test runs.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// A short, run-unique suffix used to scope every public id / source ref so the
/// fixtures never collide across runs (tables persist between runs).
fn run_suffix() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Append the run suffix to a slug, keeping it a stable, portable token.
fn run_unique(slug: &str, suffix: &str) -> String {
    format!("{slug}-{suffix}")
}

/// A valid OpenPose keypoint payload (legacy `rigToOpenposeJson` shape): body-18
/// (required) plus zero-filled face-70 and both hands-21.
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

/// `artifact://.../payload` -> `artifact://.../artifact.json` (the manifest_ref
/// the store requires alongside a sidecar payload handle).
fn artifact_manifest_ref(artifact_ref: &str) -> String {
    artifact_ref.replace("/payload", "/artifact.json")
}

/// Materialize a fresh, run-unique character and return its internal_id. Pose
/// rows FK to atelier_character, so every area creates one first.
async fn fresh_character(store: &AtelierStore, suffix: &str) -> Uuid {
    let character = store
        .create_character(&NewCharacter {
            public_id: run_unique("char-pose-mt113", suffix),
            display_name: "MT-113 Pose Subject".to_string(),
        })
        .await
        .expect("create character");
    character.internal_id
}

// =====================================================================
// Fixture shapes
// =====================================================================

#[derive(Debug, Deserialize)]
struct PoseRigSidecarsFixture {
    rigs: Vec<FixtureRig>,
}

#[derive(Debug, Deserialize)]
struct FixtureRig {
    source_ref_slug: String,
    content_hash_slug: String,
    canvas: FixtureCanvas,
    detector_provider: String,
    detector_model: String,
    detector_model_version: String,
    confidence_available: bool,
    detector_status: String,
    error_reason: Option<String>,
    sidecars: Vec<FixtureSidecar>,
}

#[derive(Debug, Deserialize)]
struct FixtureCanvas {
    width: i32,
    height: i32,
}

#[derive(Debug, Deserialize)]
struct FixtureSidecar {
    kind: String,
    mime: String,
    payload_seed: String,
    status: String,
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PoseContextWorkspaceFixture {
    context_modes: Vec<String>,
    workspace_tabs: Vec<FixtureWorkspaceTab>,
}

#[derive(Debug, Deserialize)]
struct FixtureWorkspaceTab {
    rig_index: usize,
    open: bool,
    sort_order: i32,
    active: bool,
    dirty_calibration: bool,
    panel_state: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct PoseCalibrationFixture {
    head_pose: FixtureHeadPose,
    calibration: FixtureCalibration,
}

#[derive(Debug, Deserialize)]
struct FixtureHeadPose {
    yaw_deg: f64,
    pitch_deg: f64,
    roll_deg: f64,
}

#[derive(Debug, Deserialize)]
struct FixtureCalibration {
    state: String,
    block_reason: Option<String>,
    head_pose_ref: Option<String>,
    marker_visibility: FixtureMarkerVisibility,
    marker_colors: FixtureMarkerColors,
    hand_rows: Vec<FixtureHandRow>,
    history_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureMarkerVisibility {
    body: bool,
    face: bool,
    left_hand: bool,
    right_hand: bool,
}

#[derive(Debug, Deserialize)]
struct FixtureMarkerColors {
    body: String,
    face: String,
    left_hand: String,
    right_hand: String,
}

#[derive(Debug, Deserialize)]
struct FixtureHandRow {
    hand: String,
    visible: bool,
    confidence_available: bool,
}

#[derive(Debug, Deserialize)]
struct PoseIdentityProfilesFixture {
    profiles: Vec<FixtureIdentityProfile>,
}

#[derive(Debug, Deserialize)]
struct FixtureIdentityProfile {
    kind: String,
    name: String,
    description: String,
    reference_ref_slug: String,
    source_ref_slug: String,
    provenance_clean: String,
    provenance_secret_line: Option<String>,
}

fn load_rig_sidecars_fixture() -> PoseRigSidecarsFixture {
    let raw = include_str!("fixtures/atelier_pose/pose_rig_sidecars.json");
    serde_json::from_str(raw).expect("parse pose_rig_sidecars.json fixture")
}

fn load_context_workspace_fixture() -> PoseContextWorkspaceFixture {
    let raw = include_str!("fixtures/atelier_pose/pose_context_workspace.json");
    serde_json::from_str(raw).expect("parse pose_context_workspace.json fixture")
}

fn load_calibration_fixture() -> PoseCalibrationFixture {
    let raw = include_str!("fixtures/atelier_pose/pose_calibration.json");
    serde_json::from_str(raw).expect("parse pose_calibration.json fixture")
}

fn load_identity_profiles_fixture() -> PoseIdentityProfilesFixture {
    let raw = include_str!("fixtures/atelier_pose/pose_identity_profiles.json");
    serde_json::from_str(raw).expect("parse pose_identity_profiles.json fixture")
}

/// Ingest one fixture rig (source-ref-only authored rig) through the real store.
async fn ingest_fixture_rig(
    store: &AtelierStore,
    character: Uuid,
    fixture_rig: &FixtureRig,
    suffix: &str,
) -> PoseRig {
    store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: None,
            source_ref: run_unique(&fixture_rig.source_ref_slug, suffix),
            content_hash: run_unique(&fixture_rig.content_hash_slug, suffix),
            canvas: CanvasSize {
                width: fixture_rig.canvas.width,
                height: fixture_rig.canvas.height,
            },
            detector_provider: fixture_rig.detector_provider.clone(),
            detector_model: fixture_rig.detector_model.clone(),
            detector_model_version: fixture_rig.detector_model_version.clone(),
            source_asset_version_ref: None,
            source_asset_path_ref: None,
            confidence_available: fixture_rig.confidence_available,
            detector_status: DetectorStatus::from_token(&fixture_rig.detector_status)
                .expect("fixture detector_status is a valid token"),
            error_reason: fixture_rig.error_reason.clone(),
            keypoints_json: valid_keypoints(),
            sidecar_ref: Some(format!("artifact://atelier/pose/{}", Uuid::new_v4())),
        })
        .await
        .expect("ingest fixture pose rig")
}

#[tokio::test]
async fn mt113_pose_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt113_pose_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let character = fresh_character(&store, &suffix).await;

    // -----------------------------------------------------------------
    // Area 1: rig ingest + typed OpenPose/conditioning sidecars.
    // -----------------------------------------------------------------
    let rig_fixture = load_rig_sidecars_fixture();
    assert!(!rig_fixture.rigs.is_empty(), "fixture must define rigs");

    let mut ingested_rigs: Vec<PoseRig> = Vec::new();
    for fixture_rig in &rig_fixture.rigs {
        let rig = ingest_fixture_rig(&store, character, fixture_rig, &suffix).await;

        // Round-trip: ingested fields match what the fixture declared.
        assert_eq!(
            rig.source_ref,
            run_unique(&fixture_rig.source_ref_slug, &suffix)
        );
        assert_eq!(
            rig.content_hash,
            run_unique(&fixture_rig.content_hash_slug, &suffix)
        );
        assert_eq!(rig.canvas.width, fixture_rig.canvas.width);
        assert_eq!(rig.canvas.height, fixture_rig.canvas.height);
        assert_eq!(rig.detector_model, fixture_rig.detector_model);
        assert_eq!(
            rig.detector_status,
            DetectorStatus::from_token(&fixture_rig.detector_status).unwrap()
        );
        assert_eq!(rig.error_reason, fixture_rig.error_reason);

        // Re-fetch returns the exact same row.
        let fetched = store.get_pose_rig(rig.rig_id).await.expect("get rig");
        assert_eq!(fetched, rig, "get_pose_rig round-trips the fixture rig");

        // Idempotent re-ingest: same (character, source_ref, content_hash).
        let again = ingest_fixture_rig(&store, character, fixture_rig, &suffix).await;
        assert_eq!(
            rig.rig_id, again.rig_id,
            "re-ingesting the same fixture rig is idempotent"
        );

        // Typed sidecars: each fixture sidecar payload is materialized through
        // the real ArtifactStore so artifact_ref/manifest_ref/content_hash are
        // produced exactly like production.
        for fixture_sidecar in &fixture_rig.sidecars {
            let kind = PoseSidecarKind::from_token(&fixture_sidecar.kind)
                .expect("fixture sidecar kind is a valid token");
            let seed = run_unique(&fixture_sidecar.payload_seed, &suffix);
            let artifact = atelier_pg_support::write_native_media_artifact(seed.as_bytes());
            let status = PoseSidecarStatus::from_token(&fixture_sidecar.status)
                .expect("fixture sidecar status is a valid token");
            let recorded = store
                .record_pose_sidecar(&NewPoseSidecar {
                    rig_id: rig.rig_id,
                    kind,
                    artifact_ref: artifact.artifact_ref.clone(),
                    manifest_ref: artifact_manifest_ref(&artifact.artifact_ref),
                    content_hash: artifact.content_hash.clone(),
                    byte_len: artifact.byte_len,
                    mime: fixture_sidecar.mime.clone(),
                    width: rig.canvas.width,
                    height: rig.canvas.height,
                    status,
                    error_message: fixture_sidecar.error_message.clone(),
                })
                .await
                .expect("record fixture pose sidecar");
            assert_eq!(recorded.kind, kind);
            assert_eq!(recorded.mime, fixture_sidecar.mime);
            assert_eq!(recorded.status, status);
            assert_eq!(recorded.error_message, fixture_sidecar.error_message);
            // Portable ArtifactStore handle outside .GOV, no backslashes.
            assert!(
                recorded
                    .artifact_ref
                    .starts_with("artifact://.handshake/artifacts/")
                    && !recorded.artifact_ref.contains(".GOV")
                    && !recorded.artifact_ref.contains('\\'),
                "pose sidecar must be a portable ArtifactStore handle outside .GOV"
            );
        }

        let listed = store
            .list_pose_sidecars(rig.rig_id)
            .await
            .expect("list pose sidecars");
        assert_eq!(
            listed.len(),
            fixture_rig.sidecars.len(),
            "all fixture sidecars persisted under this rig"
        );

        ingested_rigs.push(rig);
    }

    // -----------------------------------------------------------------
    // Area 2: append-only context state switches + multi-rig workspace state.
    // -----------------------------------------------------------------
    let ctx_fixture = load_context_workspace_fixture();

    // Source-linked media + rig are needed for single_image / collection modes.
    let media_artifact =
        atelier_pg_support::write_native_media_artifact(run_unique("mt113-ctx-source", &suffix).as_bytes());
    let media = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: media_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: media_artifact.byte_len,
            source_provenance: Some("mt-113 pose context source".to_string()),
            artifact_ref: media_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize pose context source media");
    let source_linked_rig = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character,
            source_asset_id: Some(media.asset_id),
            source_ref: format!("media://{}", media.asset_id),
            content_hash: run_unique("sha256-mt113-ctx", &suffix),
            canvas: CanvasSize {
                width: 1024,
                height: 1536,
            },
            detector_provider: "mediapipe.tasks-vision.pose".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: Some(format!("media-version://{}/v1", media.asset_id)),
            source_asset_path_ref: Some(format!(
                "source://operator-inbox/{}/context.png",
                Uuid::new_v4()
            )),
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: Some(format!("artifact://atelier/pose/{}", Uuid::new_v4())),
        })
        .await
        .expect("ingest source-linked context rig");
    let collection = store
        .create_collection(&NewCollection {
            name: run_unique("pose-context-mt113", &suffix),
            notes: "mt-113 pose context collection".to_string(),
            tags: vec!["pose".to_string()],
            character_internal_id: Some(character),
            sheet_version_id: None,
        })
        .await
        .expect("create pose context collection");

    let workspace_ref = format!("pose-workspace://{}", Uuid::new_v4());
    let mut applied_modes: Vec<PoseContextKind> = Vec::new();
    for mode_token in &ctx_fixture.context_modes {
        let kind = PoseContextKind::from_token(mode_token)
            .expect("fixture context mode is a valid token");
        let new_state = match kind {
            PoseContextKind::Blank => NewPoseContextState {
                workspace_ref: workspace_ref.clone(),
                kind,
                source_asset_id: None,
                character_internal_id: None,
                collection_id: None,
                selected_rig_id: None,
                requested_by: "mt-113-context".to_string(),
            },
            PoseContextKind::SingleImage => NewPoseContextState {
                workspace_ref: workspace_ref.clone(),
                kind,
                source_asset_id: Some(media.asset_id),
                character_internal_id: None,
                collection_id: None,
                selected_rig_id: Some(source_linked_rig.rig_id),
                requested_by: "mt-113-context".to_string(),
            },
            PoseContextKind::CharacterLinked => NewPoseContextState {
                workspace_ref: workspace_ref.clone(),
                kind,
                source_asset_id: None,
                character_internal_id: Some(character),
                collection_id: None,
                selected_rig_id: Some(source_linked_rig.rig_id),
                requested_by: "mt-113-context".to_string(),
            },
            PoseContextKind::CollectionLinked => NewPoseContextState {
                workspace_ref: workspace_ref.clone(),
                kind,
                source_asset_id: None,
                character_internal_id: Some(character),
                collection_id: Some(collection.collection_id),
                selected_rig_id: Some(source_linked_rig.rig_id),
                requested_by: "mt-113-context".to_string(),
            },
        };
        let state = store
            .set_pose_context_state(&new_state)
            .await
            .expect("set fixture pose context state");
        assert_eq!(state.kind, kind);
        applied_modes.push(kind);
    }

    // Append-only: history preserves every prior mode in order.
    let history = store
        .list_pose_context_history(&workspace_ref)
        .await
        .expect("read pose context history");
    assert_eq!(
        history.iter().map(|s| s.kind).collect::<Vec<_>>(),
        applied_modes,
        "context switches are append-only and preserve all prior modes"
    );

    // Multi-rig workspace state: tabs need real rigs; index into ingested_rigs.
    assert!(
        ingested_rigs.len() >= 2,
        "workspace fixture needs at least two ingested rigs"
    );
    let workspace_rigs = [
        ingested_rigs[0].rig_id,
        ingested_rigs[1].rig_id,
        source_linked_rig.rig_id,
    ];
    let session_ref = format!("pose-session://{}", Uuid::new_v4());
    for tab in &ctx_fixture.workspace_tabs {
        let rig_id = workspace_rigs[tab.rig_index];
        store
            .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
                workspace_ref: workspace_ref.clone(),
                session_ref: session_ref.clone(),
                rig_id,
                open: tab.open,
                sort_order: tab.sort_order,
                active: tab.active,
                dirty_calibration: tab.dirty_calibration,
                panel_state: tab.panel_state.clone(),
                requested_by: "mt-113-workspace".to_string(),
            })
            .await
            .expect("upsert fixture workspace rig state");
    }
    let states = store
        .list_pose_workspace_rig_state(&workspace_ref, &session_ref)
        .await
        .expect("list workspace rig state");
    assert_eq!(
        states.len(),
        ctx_fixture.workspace_tabs.len(),
        "all fixture workspace tabs persisted"
    );
    // Tabs are ordered by explicit sort_order, and at most one is active.
    let sort_orders: Vec<i32> = states.iter().map(|s| s.sort_order).collect();
    let mut sorted_orders = sort_orders.clone();
    sorted_orders.sort_unstable();
    assert_eq!(
        sort_orders, sorted_orders,
        "workspace tabs are returned in sort_order"
    );
    assert!(
        states.iter().filter(|s| s.active).count() <= 1,
        "workspace state keeps at most one active rig"
    );
    let expected_active_tabs = ctx_fixture.workspace_tabs.iter().filter(|t| t.active).count();
    assert_eq!(
        states.iter().filter(|s| s.active).count(),
        expected_active_tabs.min(1),
        "the fixture's single active tab round-trips as the one active rig"
    );

    // -----------------------------------------------------------------
    // Area 3: head pose + BLOCKED calibration (typed, never faked).
    // -----------------------------------------------------------------
    let calib_fixture = load_calibration_fixture();
    let calib_rig = &ingested_rigs[0];

    let head_pose = store
        .record_head_pose(
            calib_rig.rig_id,
            calib_fixture.head_pose.yaw_deg,
            calib_fixture.head_pose.pitch_deg,
            calib_fixture.head_pose.roll_deg,
            // Authored quaternion; store normalizes it. Use the identity-ish
            // vector so it is non-degenerate and finite.
            [0.0, 0.0, 0.0, 1.0],
        )
        .await
        .expect("record fixture head pose");
    assert_eq!(head_pose.yaw_deg, calib_fixture.head_pose.yaw_deg);
    assert_eq!(head_pose.pitch_deg, calib_fixture.head_pose.pitch_deg);
    assert_eq!(head_pose.roll_deg, calib_fixture.head_pose.roll_deg);
    let fetched_head = store
        .get_head_pose(calib_rig.rig_id)
        .await
        .expect("get head pose")
        .expect("head pose present");
    assert_eq!(fetched_head, head_pose, "head pose round-trips");

    let hand_rows: Vec<CalibrationHandRow> = calib_fixture
        .calibration
        .hand_rows
        .iter()
        .map(|hr| CalibrationHandRow {
            hand: match hr.hand.as_str() {
                "left" => CalibrationHandKind::Left,
                "right" => CalibrationHandKind::Right,
                other => panic!("fixture hand kind {other} is not valid"),
            },
            visible: hr.visible,
            // marker_count must be the hand-21 keypoint count or the store rejects it.
            marker_count: HAND_KEYPOINT_COUNT as i32,
            confidence_available: hr.confidence_available,
        })
        .collect();

    let calibration = store
        .set_pose_calibration(&NewPoseCalibration {
            rig_id: calib_rig.rig_id,
            state: CalibrationState::from_token(&calib_fixture.calibration.state)
                .expect("fixture calibration state is a valid token"),
            block_reason: calib_fixture.calibration.block_reason.clone(),
            head_pose_ref: calib_fixture.calibration.head_pose_ref.clone(),
            marker_visibility: CalibrationMarkerVisibility {
                body: calib_fixture.calibration.marker_visibility.body,
                face: calib_fixture.calibration.marker_visibility.face,
                left_hand: calib_fixture.calibration.marker_visibility.left_hand,
                right_hand: calib_fixture.calibration.marker_visibility.right_hand,
            },
            marker_colors: CalibrationMarkerColors {
                body: calib_fixture.calibration.marker_colors.body.clone(),
                face: calib_fixture.calibration.marker_colors.face.clone(),
                left_hand: calib_fixture.calibration.marker_colors.left_hand.clone(),
                right_hand: calib_fixture.calibration.marker_colors.right_hand.clone(),
            },
            hand_rows: hand_rows.clone(),
            history_refs: calib_fixture.calibration.history_refs.clone(),
        })
        .await
        .expect("set fixture pose calibration");
    assert_eq!(calibration.state, CalibrationState::Unresolved);
    assert_eq!(
        calibration.block_reason,
        calib_fixture.calibration.block_reason,
        "BLOCKED calibration preserves its block_reason, never faked"
    );
    let fetched_calib = store
        .get_calibration(calib_rig.rig_id)
        .await
        .expect("get calibration")
        .expect("calibration present");
    assert_eq!(
        fetched_calib, calibration,
        "typed calibration (markers/hands/history) round-trips verbatim"
    );
    assert_eq!(fetched_calib.hand_rows, hand_rows);
    assert_eq!(
        fetched_calib.head_pose_ref,
        calib_fixture.calibration.head_pose_ref
    );

    // -----------------------------------------------------------------
    // Area 4: append-only versioned identity profiles + secret redaction.
    // -----------------------------------------------------------------
    let identity_fixture = load_identity_profiles_fixture();
    assert!(
        !identity_fixture.profiles.is_empty(),
        "identity fixture must define profiles"
    );

    let mut expected_face_seqs: Vec<i64> = Vec::new();
    for (idx, fixture_profile) in identity_fixture.profiles.iter().enumerate() {
        let kind = IdentityProfileKind::from_token(&fixture_profile.kind)
            .expect("fixture identity kind is a valid token");
        // Compose provenance: a clean line plus an optional secret-looking line.
        let provenance = match &fixture_profile.provenance_secret_line {
            Some(secret) => format!("{}\n{}", fixture_profile.provenance_clean, secret),
            None => fixture_profile.provenance_clean.clone(),
        };
        let profile = store
            .append_identity_profile(&NewIdentityProfile {
                character_internal_id: character,
                kind,
                name: fixture_profile.name.clone(),
                description: fixture_profile.description.clone(),
                reference_asset_id: None,
                reference_ref: run_unique(&fixture_profile.reference_ref_slug, &suffix),
                source_ref: Some(run_unique(&fixture_profile.source_ref_slug, &suffix)),
                crop_ref: None,
                artifact_ref: None,
                provenance,
            })
            .await
            .expect("append fixture identity profile");
        // Append-only seq is 1-based and increments per character.
        assert_eq!(profile.seq, (idx as i64) + 1, "identity seq is 1-based per character");
        assert_eq!(profile.kind, kind);
        assert_eq!(profile.name, fixture_profile.name);
        assert_eq!(
            profile.reference_ref,
            run_unique(&fixture_profile.reference_ref_slug, &suffix)
        );

        // Secret material in provenance is redacted before storage; clean lines survive.
        assert!(
            profile.provenance.contains(&fixture_profile.provenance_clean),
            "clean provenance line round-trips"
        );
        if fixture_profile.provenance_secret_line.is_some() {
            assert!(
                profile.provenance.contains("[REDACTED]"),
                "secret-looking provenance line is redacted before storage"
            );
            assert!(
                !profile.provenance.to_ascii_lowercase().contains("bearer mt113"),
                "raw bearer token must never be stored"
            );
        }

        // Re-fetch returns the exact same active profile.
        let fetched = store
            .get_identity_profile(profile.profile_id)
            .await
            .expect("get identity profile")
            .expect("identity profile present");
        assert_eq!(fetched, profile, "identity profile round-trips");

        if kind == IdentityProfileKind::Face {
            expected_face_seqs.push(profile.seq);
        }
    }

    // list_identity_profiles is run-scoped to this character; count matches fixture.
    let all_profiles = store
        .list_identity_profiles(character, None)
        .await
        .expect("list identity profiles");
    assert_eq!(
        all_profiles.len(),
        identity_fixture.profiles.len(),
        "all fixture identity profiles persisted for this character"
    );
    // Ordered by seq ascending.
    let seqs: Vec<i64> = all_profiles.iter().map(|p| p.seq).collect();
    let mut sorted_seqs = seqs.clone();
    sorted_seqs.sort_unstable();
    assert_eq!(seqs, sorted_seqs, "identity profiles list by seq ascending");

    // latest_identity_profile(face) returns the highest-seq face profile.
    if let Some(&max_face_seq) = expected_face_seqs.iter().max() {
        let latest_face = store
            .latest_identity_profile(character, IdentityProfileKind::Face)
            .await
            .expect("latest face identity profile")
            .expect("has a face profile");
        assert_eq!(
            latest_face.seq, max_face_seq,
            "latest face identity profile is the highest-seq face profile"
        );
    }
}

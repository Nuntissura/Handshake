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

use handshake_core::atelier::pose::{
    pose_event_family, CalibrationState, DetectorStatus, IdentityProfileKind, NewIdentityProfile,
    NewPoseRig, PoseRig, CanvasSize, BODY_KEYPOINT_COUNT, FACE_KEYPOINT_COUNT, HAND_KEYPOINT_COUNT,
};
use handshake_core::atelier::{AtelierStore, NewCharacter};
use uuid::Uuid;

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

/// A valid OpenPose keypoint payload (CKC `rigToOpenposeJson`): body-18 (the
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
            detector_status: DetectorStatus::Detected,
            keypoints_json: valid_keypoints(),
            sidecar_ref: Some(format!("artifact://atelier/pose/{}", Uuid::new_v4())),
        })
        .await
        .expect("ingest pose rig")
}

#[tokio::test]
async fn atelier_pose_rig_ingest_idempotency_and_roundtrip() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_pose_rig_ingest_idempotency_and_roundtrip: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;

    let events_before = store
        .count_events(pose_event_family::POSE_RIG_INGESTED)
        .await
        .expect("count rig-ingested events before");

    // --- ingest a rig with REAL OpenPose keypoints ---
    let source_ref = format!("portrait://{}", Uuid::new_v4());
    let content_hash = format!("sha256-{}", Uuid::new_v4());
    let new = NewPoseRig {
        character_internal_id: character,
        source_asset_id: None,
        source_ref: source_ref.clone(),
        content_hash: content_hash.clone(),
        canvas: CanvasSize {
            width: 768,
            height: 1024,
        },
        detector_provider: "mediapipe.tasks-vision.pose".to_string(),
        detector_status: DetectorStatus::Detected,
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
            detector_status: DetectorStatus::Fallback,
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
            detector_status: DetectorStatus::Detected,
            keypoints_json: valid_keypoints(),
            sidecar_ref: None,
        })
        .await;
    assert!(bad.is_err(), "empty content_hash must be rejected");

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
            detector_status: DetectorStatus::Detected,
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

    // --- EVENT EMISSION: exactly one new rig-ingested event (the duplicate
    //     ingest still upserts + emits, the two rejected ingests must not) ---
    let events_after = store
        .count_events(pose_event_family::POSE_RIG_INGESTED)
        .await
        .expect("count rig-ingested events after");
    assert_eq!(
        events_after - events_before,
        2,
        "two successful ingests (original + idempotent re-ingest) each emit one event; rejected ingests emit none"
    );

    // The rig is listable for the character.
    let listed = store.list_pose_rigs(character, 50).await.expect("list rigs");
    assert!(
        listed.iter().any(|r| r.rig_id == rig.rig_id),
        "ingested rig is listed for its character"
    );
}

#[tokio::test]
async fn atelier_pose_head_pose_upsert_and_limits() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_pose_head_pose_upsert_and_limits: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;

    let events_before = store
        .count_events(pose_event_family::POSE_HEAD_POSE_RECORDED)
        .await
        .expect("count head-pose events before");

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
    assert_eq!(got.quaternion, head2.quaternion, "upsert replaced the quaternion");

    // --- INVARIANT: CKC degree limits enforced (yaw +-90 / pitch +-75 / roll +-45) ---
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

    // --- EVENT EMISSION: exactly two successful records emitted events ---
    let events_after = store
        .count_events(pose_event_family::POSE_HEAD_POSE_RECORDED)
        .await
        .expect("count head-pose events after");
    assert_eq!(
        events_after - events_before,
        2,
        "two successful head-pose records each emit one event; rejected ones emit none"
    );
}

#[tokio::test]
async fn atelier_pose_calibration_blocked_preservation() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_pose_calibration_blocked_preservation: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;
    let rig = fresh_rig(&store, character).await;

    let events_before = store
        .count_events(pose_event_family::POSE_CALIBRATION_SET)
        .await
        .expect("count calibration events before");

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
    assert_eq!(cal2.rig_id, rig.rig_id, "calibration stays keyed on the rig");
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

    // --- EVENT EMISSION: two successful sets emitted events; two rejected did not ---
    let events_after = store
        .count_events(pose_event_family::POSE_CALIBRATION_SET)
        .await
        .expect("count calibration events after");
    assert_eq!(
        events_after - events_before,
        2,
        "two successful calibration sets each emit one event; rejected sets emit none"
    );
}

#[tokio::test]
async fn atelier_identity_profile_append_only_seq_and_redaction() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_identity_profile_append_only_seq_and_redaction: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character = fresh_character(&store).await;

    let events_before = store
        .count_events(pose_event_family::IDENTITY_PROFILE_APPENDED)
        .await
        .expect("count identity-profile events before");

    // --- append the first identity profile; seq starts at 1 for a new character ---
    let p1 = store
        .append_identity_profile(&NewIdentityProfile {
            character_internal_id: character,
            kind: IdentityProfileKind::Face,
            reference_asset_id: None,
            reference_ref: format!("portrait://{}", Uuid::new_v4()),
            provenance: "source: operator upload".to_string(),
        })
        .await
        .expect("append first identity profile");
    assert_eq!(p1.seq, 1, "first profile for a fresh character is seq 1");
    assert_eq!(p1.kind, IdentityProfileKind::Face);

    // --- SECRET REDACTION: secret-looking provenance lines are masked before
    //     storage (no raw cookies/tokens ever persisted) ---
    let raw_secret = "civitai_token=supersecretvalue123";
    let p2 = store
        .append_identity_profile(&NewIdentityProfile {
            character_internal_id: character,
            kind: IdentityProfileKind::Reference,
            reference_asset_id: None,
            reference_ref: format!("reference://{}", Uuid::new_v4()),
            provenance: format!("source: civitai\n{raw_secret}\nnote: full-body ref"),
        })
        .await
        .expect("append second identity profile");
    // --- INVARIANT: append-only seq monotonicity (strictly increments) ---
    assert_eq!(p2.seq, 2, "second profile increments seq to 2 (append-only)");
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

    // --- kind filter + latest helper round-trip ---
    let faces = store
        .list_identity_profiles(character, Some(IdentityProfileKind::Face))
        .await
        .expect("list face profiles");
    assert_eq!(faces.len(), 1, "kind filter narrows to the single Face profile");
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
                reference_asset_id: None,
                reference_ref: "   ".to_string(),
                provenance: "x".to_string(),
            })
            .await
            .is_err(),
        "empty reference_ref is rejected"
    );

    // --- EVENT EMISSION: two successful appends emitted events ---
    let events_after = store
        .count_events(pose_event_family::IDENTITY_PROFILE_APPENDED)
        .await
        .expect("count identity-profile events after");
    assert_eq!(
        events_after - events_before,
        2,
        "two successful identity-profile appends each emit one event; rejected append emits none"
    );
}

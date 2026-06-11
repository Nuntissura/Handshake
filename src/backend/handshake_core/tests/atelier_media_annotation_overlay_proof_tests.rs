//! WP-KERNEL-005 MT-198 proof — per-image annotation overlays: EventLedger
//! emission, decoupling from pose keypoints, and export survival.
//!
//! The CRUD/seq/geometry half is covered by
//! `atelier_core_data_tests.rs::atelier_annotation_sequence_update_count_and_remove`;
//! v2 flagged that no test asserts the acceptance criteria beyond CRUD. This
//! file proves the remaining contract halves on Handshake-managed PostgreSQL:
//!
//!   * every annotation mutation (add / note-update / remove) emits its
//!     `atelier.annotation.*` EventLedger event, asserted via
//!     `count_events_for_aggregate` against the real `atelier_event` table;
//!   * overlays are DECOUPLED from pose keypoints: ingesting a real pose rig
//!     (OpenPose keypoints) over the same media asset leaves the overlay
//!     bit-identical, and the live schema carries no foreign key from
//!     `atelier_media_annotation` to any pose table — only to
//!     `atelier_media_asset`;
//!   * overlays SURVIVE export: after a real sheet-export request + result +
//!     media manifest entry referencing the annotated asset, the re-read
//!     overlay (ids, seq order, geometry) is unchanged.
//!
//! Gated on `atelier_pg_support::database_url()`; prints SKIP when no
//! PostgreSQL is available. Never SQLite.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::annotation::{
    annotation_event_family, AnnotationKind, NewMediaAnnotation,
};
use handshake_core::atelier::exports::{ExportFormat, ManifestItemKind, NewExportRequest};
use handshake_core::atelier::pose::{
    CanvasSize, DetectorStatus, NewPoseRig, BODY_KEYPOINT_COUNT, FACE_KEYPOINT_COUNT,
    HAND_KEYPOINT_COUNT,
};
use handshake_core::atelier::{
    AtelierStore, NewCharacter, NewMediaAsset, NewSheetVersion,
};
use uuid::Uuid;

async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
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

#[tokio::test]
async fn mt198_annotation_overlays_emit_events_decouple_from_pose_and_survive_export() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt198_annotation_overlay_proof: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let marker = format!("mt-198-overlay-{}", Uuid::new_v4());

    // --- Seed: character + sheet version + the media asset to annotate. ---
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-{marker}"),
            display_name: "Annotation Overlay Subject".to_string(),
        })
        .await
        .expect("create character");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "mt-198 annotation overlay proof sheet".to_string(),
            author: "mt-198-author".to_string(),
            tool: Some("mt-198-test".to_string()),
        })
        .await
        .expect("append sheet version");
    let artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-media").as_bytes());
    let asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some(format!("test-source:{marker}")),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize media asset");
    let aggregate_id = asset.asset_id.to_string();
    let count_annotation_events = |family: &'static str| {
        let store = store.clone();
        let aggregate_id = aggregate_id.clone();
        async move {
            store
                .count_events_for_aggregate(family, "atelier_media_annotation", &aggregate_id)
                .await
                .unwrap_or_else(|err| panic!("count {family} events: {err:?}"))
        }
    };

    // --- Mutations emit their EventLedger events (fresh asset => exact). ---
    let pin = store
        .add_media_annotation(&NewMediaAnnotation {
            asset_id: asset.asset_id,
            kind: AnnotationKind::Point,
            label: Some("focus".to_string()),
            note: "left eye".to_string(),
            geometry: serde_json::json!({ "x": 0.25, "y": 0.40 }),
            author: "mt-198-operator".to_string(),
        })
        .await
        .expect("add point annotation");
    let region = store
        .add_media_annotation(&NewMediaAnnotation {
            asset_id: asset.asset_id,
            kind: AnnotationKind::Box,
            label: Some("wardrobe".to_string()),
            note: "jacket".to_string(),
            geometry: serde_json::json!({ "x": 0.1, "y": 0.1, "w": 0.3, "h": 0.4 }),
            author: "mt-198-operator".to_string(),
        })
        .await
        .expect("add box annotation");
    assert_eq!(
        count_annotation_events(annotation_event_family::ANNOTATION_ADDED).await,
        2,
        "each add must emit exactly one ANNOTATION_ADDED event"
    );

    store
        .update_media_annotation_note(pin.annotation_id, "right eye", Some("focus-2"))
        .await
        .expect("update annotation note");
    assert_eq!(
        count_annotation_events(annotation_event_family::ANNOTATION_NOTE_UPDATED).await,
        1,
        "a note update must emit exactly one ANNOTATION_NOTE_UPDATED event"
    );

    let scratch = store
        .add_media_annotation(&NewMediaAnnotation {
            asset_id: asset.asset_id,
            kind: AnnotationKind::Point,
            label: None,
            note: "scratch pin to remove".to_string(),
            geometry: serde_json::json!({ "x": 0.9, "y": 0.9 }),
            author: "mt-198-operator".to_string(),
        })
        .await
        .expect("add scratch annotation");
    store
        .remove_media_annotation(scratch.annotation_id)
        .await
        .expect("remove scratch annotation");
    assert_eq!(
        count_annotation_events(annotation_event_family::ANNOTATION_REMOVED).await,
        1,
        "a removal must emit exactly one ANNOTATION_REMOVED event"
    );

    let overlay_before = store
        .list_media_annotations(asset.asset_id)
        .await
        .expect("list overlay before pose/export");
    assert_eq!(overlay_before.len(), 2, "point + box remain on the overlay");

    // --- Decoupled from pose keypoints: a real rig over the same asset. ---
    let rig = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id: character.internal_id,
            source_asset_id: Some(asset.asset_id),
            source_ref: format!("portrait://{marker}"),
            content_hash: asset.content_hash.clone(),
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
        .expect("ingest pose rig over the annotated asset");
    assert_eq!(rig.source_asset_id, Some(asset.asset_id));

    let overlay_after_pose = store
        .list_media_annotations(asset.asset_id)
        .await
        .expect("list overlay after pose rig ingest");
    assert_eq!(
        overlay_after_pose, overlay_before,
        "pose keypoints over the same asset must leave the overlay bit-identical"
    );

    // The live schema enforces the decoupling: the annotation table's only
    // foreign key points at the media asset, never at a pose/rig table.
    let fk_targets: Vec<(String,)> = sqlx::query_as(
        r#"SELECT DISTINCT ccu.table_name
           FROM information_schema.table_constraints tc
           JOIN information_schema.constraint_column_usage ccu
             ON ccu.constraint_name = tc.constraint_name
            AND ccu.constraint_schema = tc.constraint_schema
           WHERE tc.table_name = 'atelier_media_annotation'
             AND tc.constraint_type = 'FOREIGN KEY'"#,
    )
    .fetch_all(store.pool())
    .await
    .expect("introspect atelier_media_annotation foreign keys");
    assert_eq!(
        fk_targets,
        vec![("atelier_media_asset".to_string(),)],
        "annotations must be keyed to media identity only (no pose/rig FK)"
    );

    // --- Survives export: real export request + result + media manifest. --
    let export = store
        .request_sheet_export(&NewExportRequest {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            format: ExportFormat::Json,
            label: Some(format!("{marker}-export")),
            requested_by: "mt-198-exporter".to_string(),
        })
        .await
        .expect("request sheet export");
    store
        .record_export_result(
            export.export_id,
            &format!("artifact://atelier/export/{}", Uuid::new_v4()),
            &artifact.content_hash,
            artifact.byte_len,
        )
        .await
        .expect("record export result");
    store
        .add_manifest_entry(
            export.export_id,
            ManifestItemKind::Media,
            &asset.artifact_ref,
            "images/annotated.png",
        )
        .await
        .expect("add annotated media to the export manifest");
    let manifest = store
        .export_manifest(export.export_id)
        .await
        .expect("re-read export manifest");
    assert!(
        manifest
            .iter()
            .any(|entry| entry.kind == ManifestItemKind::Media
                && entry.artifact_ref == asset.artifact_ref),
        "the export manifest must bundle the annotated asset"
    );

    let overlay_after_export = store
        .list_media_annotations(asset.asset_id)
        .await
        .expect("list overlay after export");
    assert_eq!(
        overlay_after_export, overlay_before,
        "exporting the asset must leave the overlay (ids, seq order, geometry) unchanged"
    );
    assert_eq!(
        overlay_after_export[0].geometry,
        serde_json::json!({ "x": 0.25, "y": 0.40 }),
        "normalized 0..1 geometry survives untouched"
    );
}

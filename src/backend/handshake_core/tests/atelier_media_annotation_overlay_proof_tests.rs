//! WP-KERNEL-005 MT-198 proof — per-image annotation overlays: EventLedger
//! emission, decoupling from pose keypoints, and export survival.
//!
//! The CRUD/seq/geometry half is covered by
//! `atelier_core_data_tests.rs::atelier_annotation_sequence_update_count_and_remove`;
//! v2 flagged that no test asserts the acceptance criteria beyond CRUD. This
//! file proves the remaining contract halves on Handshake-managed embedded
//! SurrealDB:
//!
//!   * every annotation mutation (add / note-update / remove) emits its
//!     `atelier.annotation.*` EventLedger event, asserted via
//!     `count_events_for_aggregate` against the real embedded event projection;
//!   * overlays are DECOUPLED from pose keypoints: ingesting a real pose rig
//!     (OpenPose keypoints) over the same media asset leaves the overlay
//!     bit-identical, and the embedded schema carries no reference from
//!     `atelier_media_annotation` to any pose table — only to
//!     `atelier_media_asset`;
//!   * overlays SURVIVE export: after a real sheet-export request + result +
//!     media manifest entry referencing the annotated asset, the re-read
//!     overlay (ids, seq order, geometry) is unchanged.
//!
//! Uses the embedded SurrealDB harness for an isolated schema and data root.

mod atelier_surreal_support;

use handshake_core::atelier::annotation::{
    annotation_event_family, AnnotationKind, NewMediaAnnotation,
};
use handshake_core::atelier::exports::{ExportFormat, ManifestItemKind, NewExportRequest};
use handshake_core::atelier::pose::{
    CanvasSize, DetectorStatus, NewPoseRig, BODY_KEYPOINT_COUNT, FACE_KEYPOINT_COUNT,
    HAND_KEYPOINT_COUNT,
};
use handshake_core::atelier::{AtelierStore, NewCharacter, NewMediaAsset, NewSheetVersion};
use handshake_core::storage::surreal::SurrealTestInspector;
use uuid::Uuid;

async fn connected_store() -> (AtelierStore, atelier_surreal_support::AtelierSurrealHarness) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    (harness.atelier.clone(), harness)
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
    let (store, harness) = connected_store().await;
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
        atelier_surreal_support::write_native_media_artifact(format!("{marker}-media").as_bytes());
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

    // The embedded schema inspector enforces the decoupling: the annotation
    // table references media identity only, never a pose/rig table.
    let inspector: SurrealTestInspector = harness.storage.test_inspector();
    let media_asset_table = inspector
        .table_selector("atelier_media_asset")
        .await
        .expect("inspect media asset table");
    let annotation_refs = inspector
        .references_to(&media_asset_table)
        .await
        .expect("inspect embedded media references");
    let annotation_refs = annotation_refs
        .iter()
        .filter(|reference| reference.source_table() == "atelier_media_annotation")
        .collect::<Vec<_>>();
    assert_eq!(annotation_refs.len(), 1);
    assert_eq!(annotation_refs[0].source_field(), "asset_id");
    assert_eq!(annotation_refs[0].target_table(), "atelier_media_asset");

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

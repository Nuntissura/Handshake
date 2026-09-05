//! Focused Atelier moodboard proofs on the embedded SurrealDB store.
//!
//! Port of the reference (PostgreSQL) file onto `AtelierSurrealHarness`; it always runs against
//! the embedded store and never skips. Kept separate from `atelier_core_data_tests.rs` so
//! moodboard persistence proofs do not depend on unrelated mixed-binary WIP.

mod atelier_surreal_support;

use std::path::PathBuf;
use std::sync::OnceLock;

use atelier_surreal_support::{write_native_media_artifact_in_workspace, AtelierSurrealHarness};
use handshake_core::atelier::documents::{CharacterDocumentType, NewCharacterDocument};
use handshake_core::atelier::moodboards::{
    moodboard_event_family, NewMoodboardSnapshot, MOODBOARD_SCHEMA_ID,
};
use handshake_core::atelier::{AtelierError, AtelierStore, NewCharacter, NewMediaAsset};
use uuid::Uuid;

/// One workspace root for the whole test binary: `HANDSHAKE_WORKSPACE_ROOT` is process-global and
/// `materialize_media_asset` verifies the ArtifactStore binding on disk under it.
fn shared_workspace_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let root = tempfile::tempdir()
            .expect("create isolated moodboard workspace root")
            .into_path();
        std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", &root);
        root
    })
}

async fn fresh_asset(store: &AtelierStore) -> Uuid {
    let payload = format!("moodboard-test-media {}", Uuid::now_v7()).into_bytes();
    let artifact = write_native_media_artifact_in_workspace(shared_workspace_root(), &payload);
    let asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash,
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("moodboard-test".to_string()),
            artifact_ref: artifact.artifact_ref,
        })
        .await
        .expect("materialize media asset");
    asset.asset_id
}

fn moodboard_json(board_id: Uuid, layer_id: Uuid, image_id: Uuid, asset_id: Uuid, x: f64, y: f64) -> String {
    serde_json::to_string(&serde_json::json!({
        "schema_id": MOODBOARD_SCHEMA_ID,
        "schema_version": 1,
        "moodboard_id": board_id,
        "name": "MT-045 position round-trip",
        "description": "position round-trip",
        "canvas": { "width": 1600.0, "height": 1000.0, "background_color": "#101418" },
        "layers": [{
            "layer_id": layer_id, "name": "layer", "order": 1,
            "visible": true, "locked": false, "opacity": 1.0, "parent_layer_id": null
        }],
        "images": [{
            "element_id": image_id, "layer_id": layer_id, "asset_id": asset_id,
            "source": "local", "url": null,
            "position": { "x": x, "y": y }, "size": { "width": 640.0, "height": 480.0 },
            "rotation": 0.0, "opacity": 1.0, "flags": {}
        }],
        "text": [], "shapes": [], "connectors": [], "folders": [], "guides": [],
        "flags": { "locked": false, "archived": false, "operator_reviewed": false },
        "style": {
            "dominant_colors": ["#101418"], "mood_keywords": ["ckc"],
            "style_description": "proof", "suggested_presets": []
        },
        "history": [{
            "history_id": Uuid::now_v7(), "at": "2026-06-29T00:00:00Z",
            "actor": "mt-045", "operation": "created", "summary": "seed"
        }]
    }))
    .expect("serialize moodboard json")
}

/// WP-CKC MT-012 / MT-045 backend proof: a moodboard snapshot recorded with element position P1
/// is the latest; recording again with changed positions P2 produces a distinct snapshot and a
/// second MOODBOARD_SNAPSHOT_RECORDED event.
#[tokio::test]
async fn atelier_moodboard_changed_positions_record_distinct_snapshots() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-mt045-{}", Uuid::now_v7()),
            display_name: "MT-045 Position Subject".to_string(),
        })
        .await
        .expect("create character for MT-045 proof");
    let moodboard_doc = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Moodboard,
            title: "MT-045 Position Board".to_string(),
            body_raw_text: "moodboard shell text stays separate".to_string(),
            tags: vec!["moodboard".to_string()],
            author: "mt-045-author".to_string(),
        })
        .await
        .expect("create moodboard document");

    let asset_id = fresh_asset(store).await;
    let board_id = Uuid::now_v7();
    let layer_id = Uuid::now_v7();
    let image_id = Uuid::now_v7();

    let p1 = store
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: moodboard_doc.document_id,
            raw_json_text: moodboard_json(board_id, layer_id, image_id, asset_id, 120.0, 80.0),
            expected_document_version_id: None,
            author: "mt-045-author".to_string(),
        })
        .await
        .expect("record P1 moodboard snapshot");
    assert_eq!(p1.snapshot_id.get_version_num(), 7, "snapshot_id must be UUID v7");
    assert_eq!(p1.moodboard.images[0].position.x, 120.0);
    assert_eq!(p1.moodboard.images[0].position.y, 80.0);
    assert_eq!(p1.document_version_id, moodboard_doc.version_id);
    let latest_after_p1 = store
        .latest_moodboard_snapshot(moodboard_doc.document_id)
        .await
        .expect("load latest after P1")
        .expect("latest exists after P1");
    assert_eq!(latest_after_p1.snapshot_id, p1.snapshot_id);
    assert_eq!(latest_after_p1.moodboard.images[0].position.x, 120.0);

    let p2 = store
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: moodboard_doc.document_id,
            raw_json_text: moodboard_json(board_id, layer_id, image_id, asset_id, 300.0, 240.0),
            expected_document_version_id: None,
            author: "mt-045-author".to_string(),
        })
        .await
        .expect("record P2 moodboard snapshot");
    assert_ne!(
        p2.snapshot_id, p1.snapshot_id,
        "changed positions must yield a new snapshot row, not a dedup"
    );
    assert_ne!(
        p2.content_sha256, p1.content_sha256,
        "changed positions must change content_sha256"
    );
    assert_eq!(p2.moodboard.images[0].position.x, 300.0);
    assert_eq!(p2.moodboard.images[0].position.y, 240.0);

    let latest_after_p2 = store
        .latest_moodboard_snapshot(moodboard_doc.document_id)
        .await
        .expect("load latest after P2")
        .expect("latest exists after P2");
    assert_eq!(latest_after_p2.snapshot_id, p2.snapshot_id);
    assert_eq!(latest_after_p2.moodboard.images[0].position.x, 300.0);

    let snapshot_events = store
        .count_events_for_aggregate(
            moodboard_event_family::MOODBOARD_SNAPSHOT_RECORDED,
            "atelier_character_document",
            &moodboard_doc.document_id.to_string(),
        )
        .await
        .expect("count moodboard snapshot events");
    assert_eq!(
        snapshot_events, 2,
        "each distinct-position snapshot appends its own MOODBOARD_SNAPSHOT_RECORDED event"
    );
    harness.shutdown().await;
}

/// WP-CKC MT-012 optimistic concurrency: `expected_document_version_id` pins the snapshot to the
/// moodboard document head; a stale expectation is a typed conflict that writes nothing.
#[tokio::test]
async fn atelier_moodboard_snapshot_guard_rejects_stale_document_version() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-mt012-guard-{}", Uuid::now_v7()),
            display_name: "MT-012 Guard Subject".to_string(),
        })
        .await
        .expect("create character");
    let moodboard_doc = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Moodboard,
            title: "MT-012 Guard Board".to_string(),
            body_raw_text: "shell".to_string(),
            tags: vec![],
            author: "mt-012-author".to_string(),
        })
        .await
        .expect("create moodboard document");
    let asset_id = fresh_asset(store).await;
    let (board_id, layer_id, image_id) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());

    let pinned = store
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: moodboard_doc.document_id,
            raw_json_text: moodboard_json(board_id, layer_id, image_id, asset_id, 1.0, 1.0),
            expected_document_version_id: Some(moodboard_doc.version_id),
            author: "mt-012-author".to_string(),
        })
        .await
        .expect("snapshot pinned to the current document head");
    assert_eq!(pinned.document_version_id, moodboard_doc.version_id);

    let stale = store
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: moodboard_doc.document_id,
            raw_json_text: moodboard_json(board_id, layer_id, image_id, asset_id, 2.0, 2.0),
            expected_document_version_id: Some(Uuid::now_v7()),
            author: "mt-012-author".to_string(),
        })
        .await
        .expect_err("a snapshot pinned to a non-head document version must be refused");
    assert!(
        matches!(stale, AtelierError::Conflict(ref detail) if detail.contains("stale_moodboard_document_version")),
        "stale pin should be a typed conflict: {stale:?}"
    );
    let latest = store
        .latest_moodboard_snapshot(moodboard_doc.document_id)
        .await
        .expect("load latest")
        .expect("latest exists");
    assert_eq!(latest.snapshot_id, pinned.snapshot_id, "the stale write must not become latest");

    let story_doc = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id: character.internal_id,
            doc_type: CharacterDocumentType::Story,
            title: "not a moodboard".to_string(),
            body_raw_text: "story".to_string(),
            tags: vec![],
            author: "mt-012-author".to_string(),
        })
        .await
        .expect("create story document");
    let wrong_kind = store
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id: story_doc.document_id,
            raw_json_text: moodboard_json(board_id, layer_id, image_id, asset_id, 3.0, 3.0),
            expected_document_version_id: None,
            author: "mt-012-author".to_string(),
        })
        .await
        .expect_err("moodboard snapshots must reject story documents");
    assert!(
        matches!(wrong_kind, AtelierError::Validation(_)),
        "wrong document type should be a validation error: {wrong_kind:?}"
    );
    harness.shutdown().await;
}

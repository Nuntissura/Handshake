//! WP-KERNEL-005 MT-016 ArtifactStore media materialization proof.
//!
//! Uses live PostgreSQL/EventLedger only. Media materialization must route
//! through Handshake-native ArtifactStore handles and persist a manifest with
//! hash, size, source, and retention metadata, never `.GOV` or local filesystem
//! output paths.

mod atelier_pg_support;

use futures::future::join_all;
use handshake_core::atelier::media::{
    MEDIA_ARTIFACT_MANIFEST_SCHEMA, MEDIA_ORIGINAL_RETENTION_CLASS,
};
use handshake_core::atelier::{
    event_family, AtelierStore, MediaDerivativeFailure, MediaDerivativeKind,
    MediaDerivativeRequest, MediaDerivativeStatus, MediaReviewMetadataUpdate,
    MediaSidecarRelationKind, NewMediaAsset, NewMediaSidecarRelation,
};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::storage::artifacts::{artifact_root_rel, ArtifactLayer};
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Default)]
struct MemoryFlightRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[async_trait::async_trait]
impl FlightRecorder for MemoryFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self
            .events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .clone())
    }
}

async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

async fn connected_store_with_observability(
    url: &str,
) -> (AtelierStore, Arc<MemoryFlightRecorder>) {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool.clone());
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    let database = database.into_arc();
    let flight_recorder = Arc::new(MemoryFlightRecorder::default());
    let store = AtelierStore::with_observability(pool, database, flight_recorder.clone());
    store.ensure_schema().await.expect("ensure atelier schema");
    (store, flight_recorder)
}

fn sha256_token() -> String {
    let a = Uuid::new_v4().simple();
    let b = Uuid::new_v4().simple();
    format!("sha256:{a}{b}")
}

async fn native_media_asset(
    store: &AtelierStore,
    label: &str,
) -> handshake_core::atelier::MediaAsset {
    let artifact = atelier_pg_support::write_native_media_artifact(label.as_bytes());
    store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash,
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some(format!("artifact-store:{label}")),
            artifact_ref: artifact.artifact_ref,
        })
        .await
        .expect("materialize native media asset")
}

async fn bulk_receipt_count(store: &AtelierStore, operation: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM atelier_bulk_operation_receipt WHERE operation = $1")
        .bind(operation)
        .fetch_one(store.pool())
        .await
        .expect("count bulk operation receipts")
}

async fn canonical_receipt_event_count(store: &AtelierStore, receipt_id: Uuid) -> i64 {
    sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM atelier_event ae
           JOIN kernel_event_ledger kel
             ON kel.event_id = ae.kernel_event_id
            AND kel.event_sequence = ae.kernel_event_sequence
           WHERE ae.event_family = $1
             AND ae.aggregate_type = 'atelier_bulk_operation_receipt'
             AND ae.aggregate_id = $2
             AND kel.aggregate_type = ae.aggregate_type
             AND kel.aggregate_id = ae.aggregate_id
             AND kel.source_component = 'atelier'"#,
    )
    .bind(event_family::BULK_OPERATION_APPLIED)
    .bind(receipt_id.to_string())
    .fetch_one(store.pool())
    .await
    .expect("count canonical receipt event")
}

async fn flight_event_count(recorder: &MemoryFlightRecorder) -> usize {
    recorder
        .list_events(EventFilter::default())
        .await
        .expect("list flight recorder events")
        .len()
}

#[tokio::test]
async fn media_materialization_persists_artifact_manifest_retention_and_event() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_materialization_persists_artifact_manifest_retention_and_event: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let artifact = atelier_pg_support::write_native_media_artifact(b"mt-016 native media bytes");
    let asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("artifact-store:operator-import".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize media asset through ArtifactStore manifest");

    assert_eq!(asset.content_hash, artifact.content_hash);
    assert_eq!(asset.artifact_ref, artifact.artifact_ref);
    assert_eq!(asset.retention_class, MEDIA_ORIGINAL_RETENTION_CLASS);
    assert_eq!(
        asset.artifact_manifest["schema"],
        MEDIA_ARTIFACT_MANIFEST_SCHEMA
    );
    assert_eq!(
        asset.artifact_manifest["asset_id"],
        serde_json::json!(asset.asset_id)
    );
    assert_eq!(
        asset.artifact_manifest["content_hash"],
        artifact.content_hash
    );
    assert_eq!(asset.artifact_manifest["byte_len"], artifact.byte_len);
    assert_eq!(asset.artifact_manifest["size_bytes"], artifact.byte_len);
    assert!(
        asset.artifact_manifest.get("source").is_none(),
        "persisted media artifact manifest must not duplicate raw source text"
    );
    assert!(
        asset.artifact_manifest.get("source_provenance").is_none(),
        "persisted media artifact manifest must not duplicate raw source provenance"
    );
    assert!(
        asset.artifact_manifest["source_provenance_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "persisted media artifact manifest must carry only a content-addressed provenance ref"
    );
    assert!(
        !asset
            .artifact_manifest
            .to_string()
            .contains("artifact-store:operator-import"),
        "persisted media artifact manifest must not leak raw source provenance"
    );
    assert_eq!(
        asset.artifact_manifest["retention_class"],
        MEDIA_ORIGINAL_RETENTION_CLASS
    );
    assert_eq!(
        asset.artifact_manifest["artifact_store"]["handle"],
        artifact.artifact_ref
    );
    assert_eq!(
        asset.artifact_manifest["artifact_store"]["content_hash"],
        artifact.content_hash
    );
    assert_eq!(
        asset.artifact_manifest["artifact_store"]["size_bytes"],
        artifact.byte_len
    );
    assert!(
        !asset
            .artifact_manifest
            .to_string()
            .to_ascii_lowercase()
            .contains(".gov"),
        "media artifact manifest must not route outputs into .GOV"
    );

    let persisted_manifest = store
        .get_media_artifact_manifest(asset.asset_id)
        .await
        .expect("read persisted media artifact manifest");
    assert_eq!(persisted_manifest, asset.artifact_manifest);

    let event_count = store
        .count_events_for_aggregate(
            event_family::MEDIA_ASSET_MATERIALIZED,
            "atelier_media_asset",
            &artifact.content_hash,
        )
        .await
        .expect("count media materialization event");
    assert_eq!(event_count, 1);
    let event_payload: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_media_asset'
             AND aggregate_id = $2
           ORDER BY created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(event_family::MEDIA_ASSET_MATERIALIZED)
    .bind(&artifact.content_hash)
    .fetch_one(store.pool())
    .await
    .expect("read media event payload");
    assert_eq!(
        event_payload["artifact_manifest"]["schema"],
        MEDIA_ARTIFACT_MANIFEST_SCHEMA
    );
    assert_eq!(
        event_payload["artifact_manifest"]["size_bytes"],
        artifact.byte_len
    );
    assert!(
        event_payload["artifact_manifest"].get("source").is_none(),
        "media materialization EventLedger payload must not leak raw source provenance"
    );
    assert!(
        event_payload["artifact_manifest"]
            .get("source_provenance")
            .is_none(),
        "media materialization EventLedger payload must not leak raw source provenance"
    );
    assert!(
        event_payload["artifact_manifest"]
            .get("source_provenance_ref")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value.starts_with("sha256:")),
        "media materialization event must keep only a content-addressed provenance ref"
    );
    assert_eq!(
        event_payload["retention_class"],
        MEDIA_ORIGINAL_RETENTION_CLASS
    );

    let again = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("artifact-store:operator-import".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("re-materialize same content hash");
    assert_eq!(
        again.asset_id, asset.asset_id,
        "same content hash must dedup to existing media asset"
    );
    assert_eq!(
        again.artifact_manifest, asset.artifact_manifest,
        "dedup must preserve the original artifact manifest"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_ASSET_MATERIALIZED,
                "atelier_media_asset",
                &artifact.content_hash,
            )
            .await
            .expect("count media materialization events after dedup"),
        1,
        "dedup must not emit duplicate materialization events"
    );

    let prefixed_upper_hash = format!("sha256:{}", artifact.content_hash.to_ascii_uppercase());
    let canonical_again = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: prefixed_upper_hash,
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("artifact-store:operator-import".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("re-materialize same content hash in prefixed uppercase form");
    assert_eq!(
        canonical_again.asset_id, asset.asset_id,
        "content_hash must canonicalize before dedup lookup"
    );
    assert_eq!(
        canonical_again.content_hash, artifact.content_hash,
        "content_hash must persist in canonical bare lowercase form"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_ASSET_MATERIALIZED,
                "atelier_media_asset",
                &artifact.content_hash,
            )
            .await
            .expect("count media materialization events after canonical dedup"),
        1,
        "canonical hash variants must not emit duplicate materialization events"
    );
}

#[tokio::test]
async fn media_sidecar_visibility_matrix_hides_sidecars_from_gallery_but_keeps_relation_search() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_sidecar_visibility_matrix_hides_sidecars_from_gallery_but_keeps_relation_search: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    assert!(
        event_family::ALL.contains(&event_family::MEDIA_SIDECAR_RECORDED),
        "sidecar event family must be discoverable through the parent atelier registry"
    );

    let primary = native_media_asset(&store, "mt-022-primary-gallery-image").await;
    let openpose_sidecar = native_media_asset(&store, "mt-022-openpose-sidecar").await;
    let workflow_sidecar = native_media_asset(&store, "mt-022-workflow-sidecar").await;

    let openpose_relation = store
        .record_media_sidecar_relation(&NewMediaSidecarRelation {
            parent_asset_id: primary.asset_id,
            sidecar_asset_id: openpose_sidecar.asset_id,
            relation_kind: MediaSidecarRelationKind::OpenPoseJson,
            created_by: "mt-022-operator".to_string(),
        })
        .await
        .expect("record OpenPose sidecar relation");
    let workflow_relation = store
        .record_media_sidecar_relation(&NewMediaSidecarRelation {
            parent_asset_id: primary.asset_id,
            sidecar_asset_id: workflow_sidecar.asset_id,
            relation_kind: MediaSidecarRelationKind::WorkflowJson,
            created_by: "mt-022-operator".to_string(),
        })
        .await
        .expect("record workflow sidecar relation");

    assert!(openpose_relation.hidden_from_gallery);
    assert!(openpose_relation.searchable_by_relation);
    assert!(workflow_relation.hidden_from_gallery);
    assert!(workflow_relation.searchable_by_relation);

    let openpose_rows = store
        .list_media_sidecars_for_asset(
            primary.asset_id,
            Some(MediaSidecarRelationKind::OpenPoseJson),
        )
        .await
        .expect("search OpenPose sidecars by relation");
    assert_eq!(openpose_rows.len(), 1);
    assert_eq!(openpose_rows[0].sidecar_asset_id, openpose_sidecar.asset_id);
    assert_eq!(
        openpose_rows[0].relation_kind,
        MediaSidecarRelationKind::OpenPoseJson
    );

    let all_sidecars = store
        .list_media_sidecars_for_asset(primary.asset_id, None)
        .await
        .expect("list all sidecar relations");
    let sidecar_asset_ids: HashSet<Uuid> = all_sidecars
        .iter()
        .map(|sidecar| sidecar.sidecar_asset_id)
        .collect();
    assert_eq!(sidecar_asset_ids.len(), 2);
    assert!(sidecar_asset_ids.contains(&openpose_sidecar.asset_id));
    assert!(sidecar_asset_ids.contains(&workflow_sidecar.asset_id));

    let gallery = store
        .list_media_gallery_assets(500)
        .await
        .expect("list normal gallery assets");
    let gallery_asset_ids: HashSet<Uuid> = gallery.iter().map(|asset| asset.asset_id).collect();
    assert!(
        gallery_asset_ids.contains(&primary.asset_id),
        "normal gallery must retain the primary media asset"
    );
    assert!(
        !gallery_asset_ids.contains(&openpose_sidecar.asset_id),
        "OpenPose sidecar asset must be hidden from normal gallery lists"
    );
    assert!(
        !gallery_asset_ids.contains(&workflow_sidecar.asset_id),
        "workflow sidecar asset must be hidden from normal gallery lists"
    );

    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_SIDECAR_RECORDED,
                "atelier_media_sidecar",
                &openpose_relation.sidecar_id.to_string(),
            )
            .await
            .expect("count OpenPose sidecar event"),
        1
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_SIDECAR_RECORDED,
                "atelier_media_sidecar",
                &workflow_relation.sidecar_id.to_string(),
            )
            .await
            .expect("count workflow sidecar event"),
        1
    );
}

#[tokio::test]
async fn media_review_metadata_batch_clamps_and_records_receipt() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_review_metadata_batch_clamps_and_records_receipt: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let first = native_media_asset(&store, "mt-018-review-first").await;
    let second = native_media_asset(&store, "mt-018-review-second").await;
    let receipts_before_failure =
        bulk_receipt_count(&store, "bulk_update_media_review_metadata").await;
    let first_events_before_failure = store
        .count_events_for_aggregate(
            event_family::MEDIA_REVIEW_METADATA_UPDATED,
            "atelier_media_review_metadata",
            &first.asset_id.to_string(),
        )
        .await
        .expect("count first review events before failed batch");

    let valid_update = MediaReviewMetadataUpdate {
        asset_id: first.asset_id,
        favorite: true,
        rating: 9,
        frontpage: true,
        carousel: false,
        notes: Some("frontpage candidate".to_string()),
        review_status: "approved".to_string(),
    };
    let missing_update = MediaReviewMetadataUpdate {
        asset_id: Uuid::new_v4(),
        favorite: false,
        rating: 2,
        frontpage: false,
        carousel: true,
        notes: None,
        review_status: "deferred".to_string(),
    };

    let failed = store
        .bulk_update_media_review_metadata(
            &[valid_update.clone(), missing_update],
            "mt-018-reviewer",
        )
        .await;
    assert!(
        failed.is_err(),
        "missing asset must reject the whole review metadata batch"
    );
    assert!(
        store
            .get_media_review_metadata(first.asset_id)
            .await
            .expect("read first metadata after failed batch")
            .is_none(),
        "failed review metadata batch must not partially write valid target"
    );
    assert_eq!(
        receipts_before_failure,
        bulk_receipt_count(&store, "bulk_update_media_review_metadata").await,
        "failed review metadata batch must not write a receipt"
    );
    assert_eq!(
        first_events_before_failure,
        store
            .count_events_for_aggregate(
                event_family::MEDIA_REVIEW_METADATA_UPDATED,
                "atelier_media_review_metadata",
                &first.asset_id.to_string(),
            )
            .await
            .expect("count first review events after failed batch"),
        "failed review metadata batch must not write EventLedger rows"
    );

    let second_update = MediaReviewMetadataUpdate {
        asset_id: second.asset_id,
        favorite: false,
        rating: -4,
        frontpage: false,
        carousel: true,
        notes: Some("carousel backlog".to_string()),
        review_status: "deferred".to_string(),
    };
    let applied = store
        .bulk_update_media_review_metadata(&[valid_update, second_update], "mt-018-reviewer")
        .await
        .expect("apply review metadata batch");
    assert_eq!(
        applied.receipt.operation,
        "bulk_update_media_review_metadata"
    );
    assert_eq!(applied.receipt.target_count, 2);
    assert_eq!(applied.receipt.mutation_count, 2);
    assert_eq!(
        canonical_receipt_event_count(&store, applied.receipt.receipt_id).await,
        1,
        "review metadata receipt must link to canonical EventLedger"
    );
    assert_eq!(applied.metadata[0].rating, 5, "rating clamps high to 5");
    assert_eq!(applied.metadata[1].rating, 0, "rating clamps low to 0");
    assert!(applied.metadata[0].favorite);
    assert!(applied.metadata[0].frontpage);
    assert!(applied.metadata[1].carousel);
    assert_eq!(applied.metadata[0].review_status, "approved");
    assert_eq!(applied.metadata[1].review_status, "deferred");

    let persisted_first = store
        .get_media_review_metadata(first.asset_id)
        .await
        .expect("read persisted first review metadata")
        .expect("first review metadata exists");
    assert_eq!(persisted_first.rating, 5);
    assert_eq!(
        persisted_first.notes.as_deref(),
        Some("frontpage candidate")
    );
    let review_event_count = store
        .count_events_for_aggregate(
            event_family::MEDIA_REVIEW_METADATA_UPDATED,
            "atelier_media_review_metadata",
            &first.asset_id.to_string(),
        )
        .await
        .expect("count first review metadata event");
    assert_eq!(review_event_count, 1);
}

#[tokio::test]
async fn media_review_metadata_preserves_note_text_exactly() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP media_review_metadata_preserves_note_text_exactly: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    let padded = native_media_asset(&store, "mt-018-review-padded-notes").await;
    let whitespace = native_media_asset(&store, "mt-018-review-whitespace-notes").await;
    let padded_notes = "  keep spacing  ".to_string();
    let whitespace_notes = "   ".to_string();
    let applied = store
        .bulk_update_media_review_metadata(
            &[
                MediaReviewMetadataUpdate {
                    asset_id: padded.asset_id,
                    favorite: true,
                    rating: 4,
                    frontpage: false,
                    carousel: false,
                    notes: Some(padded_notes.clone()),
                    review_status: "review".to_string(),
                },
                MediaReviewMetadataUpdate {
                    asset_id: whitespace.asset_id,
                    favorite: false,
                    rating: 1,
                    frontpage: false,
                    carousel: false,
                    notes: Some(whitespace_notes.clone()),
                    review_status: "deferred".to_string(),
                },
            ],
            "mt-018-reviewer",
        )
        .await
        .expect("apply exact note preservation batch");
    assert_eq!(
        applied.metadata[0].notes.as_deref(),
        Some(padded_notes.as_str())
    );
    assert_eq!(
        applied.metadata[1].notes.as_deref(),
        Some(whitespace_notes.as_str())
    );

    let persisted_padded = store
        .get_media_review_metadata(padded.asset_id)
        .await
        .expect("read padded review metadata")
        .expect("padded review metadata exists");
    assert_eq!(
        persisted_padded.notes.as_deref(),
        Some(padded_notes.as_str())
    );
    let persisted_whitespace = store
        .get_media_review_metadata(whitespace.asset_id)
        .await
        .expect("read whitespace review metadata")
        .expect("whitespace review metadata exists");
    assert_eq!(
        persisted_whitespace.notes.as_deref(),
        Some(whitespace_notes.as_str())
    );

    let event_payload: serde_json::Value = sqlx::query(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_media_review_metadata'
             AND aggregate_id = $2
           ORDER BY created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(event_family::MEDIA_REVIEW_METADATA_UPDATED)
    .bind(padded.asset_id.to_string())
    .fetch_one(store.pool())
    .await
    .expect("read padded review metadata event")
    .get("payload");
    assert_eq!(
        event_payload.get("notes_present"),
        Some(&serde_json::json!(true))
    );
    assert!(event_payload
        .get("notes_ref")
        .and_then(serde_json::Value::as_str)
        .is_some());
    assert!(
        event_payload.get("notes").is_none(),
        "raw review notes must not be emitted into EventLedger payloads"
    );
}

#[tokio::test]
async fn media_review_metadata_invalid_status_batch_has_no_side_effects() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_review_metadata_invalid_status_batch_has_no_side_effects: PostgreSQL unavailable"
        );
        return;
    };
    let (store, flight_recorder) = connected_store_with_observability(&url).await;

    let first = native_media_asset(&store, "mt-018-invalid-status-first").await;
    let second = native_media_asset(&store, "mt-018-invalid-status-second").await;
    let receipts_before = bulk_receipt_count(&store, "bulk_update_media_review_metadata").await;
    let first_events_before = store
        .count_events_for_aggregate(
            event_family::MEDIA_REVIEW_METADATA_UPDATED,
            "atelier_media_review_metadata",
            &first.asset_id.to_string(),
        )
        .await
        .expect("count first review events before invalid batch");
    let flight_events_before = flight_event_count(&flight_recorder).await;

    let result = store
        .bulk_update_media_review_metadata(
            &[
                MediaReviewMetadataUpdate {
                    asset_id: first.asset_id,
                    favorite: true,
                    rating: 5,
                    frontpage: true,
                    carousel: false,
                    notes: Some("valid row before invalid status".to_string()),
                    review_status: "approved".to_string(),
                },
                MediaReviewMetadataUpdate {
                    asset_id: second.asset_id,
                    favorite: false,
                    rating: 1,
                    frontpage: false,
                    carousel: true,
                    notes: None,
                    review_status: "published".to_string(),
                },
            ],
            "mt-018-reviewer",
        )
        .await;
    assert!(
        result
            .expect_err("invalid status must reject the review metadata batch")
            .to_string()
            .contains("unsupported review_status"),
        "invalid review_status should be rejected during batch prevalidation"
    );
    assert!(
        store
            .get_media_review_metadata(first.asset_id)
            .await
            .expect("read first metadata after invalid status batch")
            .is_none(),
        "invalid status batch must not partially write an earlier valid target"
    );
    assert_eq!(
        receipts_before,
        bulk_receipt_count(&store, "bulk_update_media_review_metadata").await,
        "invalid status batch must not write a receipt"
    );
    assert_eq!(
        first_events_before,
        store
            .count_events_for_aggregate(
                event_family::MEDIA_REVIEW_METADATA_UPDATED,
                "atelier_media_review_metadata",
                &first.asset_id.to_string(),
            )
            .await
            .expect("count first review events after invalid status batch"),
        "invalid status batch must not write EventLedger rows"
    );
    assert_eq!(
        flight_events_before,
        flight_event_count(&flight_recorder).await,
        "invalid status batch must not emit Flight Recorder side effects"
    );
}

#[tokio::test]
async fn media_derivative_skeleton_tracks_thumbnail_proxy_and_retry_states() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_derivative_skeleton_tracks_thumbnail_proxy_and_retry_states: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let original = native_media_asset(&store, "mt-017-original").await;
    let thumb = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: original.asset_id,
            derivative_kind: MediaDerivativeKind::Thumbnail,
            target_width: 256,
            target_height: 256,
            format: "png".to_string(),
            requested_by: "mt-017-operator".to_string(),
        })
        .await
        .expect("request thumbnail derivative");
    let proxy = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: original.asset_id,
            derivative_kind: MediaDerivativeKind::Proxy,
            target_width: 1024,
            target_height: 1024,
            format: "jpeg".to_string(),
            requested_by: "mt-017-operator".to_string(),
        })
        .await
        .expect("request proxy derivative");
    let skeleton = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: original.asset_id,
            derivative_kind: MediaDerivativeKind::PhotoStudioSkeleton,
            target_width: 512,
            target_height: 768,
            format: "png".to_string(),
            requested_by: "mt-017-operator".to_string(),
        })
        .await
        .expect("request Photo Studio skeleton derivative");

    assert_eq!(thumb.status, MediaDerivativeStatus::Pending);
    assert_eq!(thumb.derivative_kind, MediaDerivativeKind::Thumbnail);
    assert_eq!(proxy.derivative_kind, MediaDerivativeKind::Proxy);
    assert_eq!(
        skeleton.derivative_kind,
        MediaDerivativeKind::PhotoStudioSkeleton
    );
    assert_eq!(
        store
            .list_media_derivatives(original.asset_id)
            .await
            .expect("list media derivatives")
            .len(),
        3,
        "thumbnail, proxy, and Photo Studio skeleton requests must all persist"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_REQUESTED,
                "atelier_media_derivative",
                &thumb.derivative_id.to_string(),
            )
            .await
            .expect("count thumbnail request event"),
        1,
        "derivative request must write EventLedger evidence"
    );

    let generating = store
        .mark_media_derivative_generating(thumb.derivative_id, "mt-017-worker")
        .await
        .expect("mark thumbnail generating");
    assert_eq!(generating.status, MediaDerivativeStatus::Generating);

    let thumbnail_artifact = atelier_pg_support::write_native_media_artifact(b"thumb-png");
    let generated = store
        .record_media_derivative_generated_with_artifact(
            &handshake_core::atelier::MediaDerivativeGenerated {
                derivative_id: thumb.derivative_id,
                artifact_ref: thumbnail_artifact.artifact_ref.clone(),
                artifact_manifest_ref: format!(
                    "artifact://.handshake/artifacts/L1/{}/artifact.json",
                    thumbnail_artifact.artifact_id
                ),
                mime: "image/png".to_string(),
                byte_len: thumbnail_artifact.byte_len,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await
        .expect("record generated thumbnail");
    assert_eq!(generated.status, MediaDerivativeStatus::Generated);
    assert_eq!(
        generated.artifact_ref.as_deref(),
        Some(thumbnail_artifact.artifact_ref.as_str())
    );
    assert_eq!(generated.mime.as_deref(), Some("image/png"));
    assert_eq!(generated.byte_len, Some(thumbnail_artifact.byte_len));
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_GENERATED,
                "atelier_media_derivative",
                &thumb.derivative_id.to_string(),
            )
            .await
            .expect("count thumbnail generated event"),
        1,
        "generated derivative must write EventLedger evidence"
    );

    let failed_proxy = store
        .record_media_derivative_failure(
            proxy.derivative_id,
            &MediaDerivativeFailure {
                error_code: "decode_failed".to_string(),
                error_detail: "C:\\operator\\source\\proxy-input.png decode failed".to_string(),
                retryable: true,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await
        .expect("record retryable proxy failure");
    assert_eq!(failed_proxy.status, MediaDerivativeStatus::RetryableError);
    assert_eq!(failed_proxy.attempt_count, 1);
    assert_eq!(
        failed_proxy.last_error_code.as_deref(),
        Some("decode_failed")
    );
    assert!(
        failed_proxy
            .last_error_ref
            .as_deref()
            .unwrap_or_default()
            .starts_with("sha256:"),
        "failure detail must persist as a hash ref, not raw machine-local text"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_FAILED,
                "atelier_media_derivative",
                &proxy.derivative_id.to_string(),
            )
            .await
            .expect("count proxy failure event"),
        1,
        "retryable derivative failure must write EventLedger evidence"
    );

    let retried_proxy = store
        .retry_media_derivative(proxy.derivative_id, "mt-017-operator")
        .await
        .expect("retry proxy derivative");
    assert_eq!(retried_proxy.status, MediaDerivativeStatus::Pending);
    assert_eq!(retried_proxy.retry_count, 1);
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_RETRIED,
                "atelier_media_derivative",
                &proxy.derivative_id.to_string(),
            )
            .await
            .expect("count proxy retry event"),
        1,
        "retry must write EventLedger evidence"
    );

    let hard_failed_skeleton = store
        .record_media_derivative_failure(
            skeleton.derivative_id,
            &MediaDerivativeFailure {
                error_code: "unsupported_format".to_string(),
                error_detail: "unsupported skeleton generation target".to_string(),
                retryable: false,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await
        .expect("record non-retryable skeleton failure");
    assert_eq!(hard_failed_skeleton.status, MediaDerivativeStatus::Failed);
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_FAILED,
                "atelier_media_derivative",
                &skeleton.derivative_id.to_string(),
            )
            .await
            .expect("count skeleton failure event"),
        1,
        "hard derivative failure must write EventLedger evidence"
    );
    assert!(
        store
            .retry_media_derivative(skeleton.derivative_id, "mt-017-operator")
            .await
            .is_err(),
        "non-retryable derivative failures must not be retried"
    );
}

#[tokio::test]
async fn media_derivative_retryable_error_rejects_duplicate_failure_until_retry() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_derivative_retryable_error_rejects_duplicate_failure_until_retry: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let original = native_media_asset(&store, "mt-017-duplicate-failure").await;
    let derivative = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: original.asset_id,
            derivative_kind: MediaDerivativeKind::Proxy,
            target_width: 512,
            target_height: 512,
            format: "jpeg".to_string(),
            requested_by: "mt-017-operator".to_string(),
        })
        .await
        .expect("request derivative before duplicate failure proof");
    let first_failure = store
        .record_media_derivative_failure(
            derivative.derivative_id,
            &MediaDerivativeFailure {
                error_code: "first_worker_failure".to_string(),
                error_detail: "first worker reported retryable failure".to_string(),
                retryable: true,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await
        .expect("record first retryable failure");
    assert_eq!(first_failure.status, MediaDerivativeStatus::RetryableError);
    assert_eq!(first_failure.attempt_count, 1);
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_FAILED,
                "atelier_media_derivative",
                &derivative.derivative_id.to_string(),
            )
            .await
            .expect("count first failure event"),
        1
    );

    let duplicate_failure = store
        .record_media_derivative_failure(
            derivative.derivative_id,
            &MediaDerivativeFailure {
                error_code: "late_duplicate_failure".to_string(),
                error_detail: "late duplicate worker tried to overwrite retryable state".to_string(),
                retryable: false,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await
        .expect_err("retryable_error must not accept another failure before explicit retry");
    assert!(
        duplicate_failure.to_string().contains("not active"),
        "duplicate failure should be rejected as an invalid state transition: {duplicate_failure}"
    );

    let rows = store
        .list_media_derivatives(original.asset_id)
        .await
        .expect("list derivatives after rejected duplicate failure");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, MediaDerivativeStatus::RetryableError);
    assert_eq!(rows[0].attempt_count, 1);
    assert_eq!(
        rows[0].last_error_code.as_deref(),
        Some("first_worker_failure")
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_FAILED,
                "atelier_media_derivative",
                &derivative.derivative_id.to_string(),
            )
            .await
            .expect("count failure events after duplicate rejection"),
        1,
        "rejected duplicate failure must not write EventLedger evidence"
    );

    let retried = store
        .retry_media_derivative(derivative.derivative_id, "mt-017-operator")
        .await
        .expect("explicit retry should reopen derivative");
    assert_eq!(retried.status, MediaDerivativeStatus::Pending);
    assert_eq!(retried.retry_count, 1);
    let second_failure = store
        .record_media_derivative_failure(
            derivative.derivative_id,
            &MediaDerivativeFailure {
                error_code: "second_worker_failure".to_string(),
                error_detail: "second attempt failed after explicit retry".to_string(),
                retryable: true,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await
        .expect("failure after explicit retry should be accepted");
    assert_eq!(second_failure.status, MediaDerivativeStatus::RetryableError);
    assert_eq!(second_failure.attempt_count, 2);
    assert_eq!(second_failure.retry_count, 1);
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_FAILED,
                "atelier_media_derivative",
                &derivative.derivative_id.to_string(),
            )
            .await
            .expect("count failure events after retried failure"),
        2,
        "failure after explicit retry must write a second EventLedger row"
    );
}

#[tokio::test]
async fn media_derivative_generated_rejects_fake_artifact_refs_without_side_effects() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_derivative_generated_rejects_fake_artifact_refs_without_side_effects: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let original = native_media_asset(&store, "mt-017-fake-generated").await;
    let derivative = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: original.asset_id,
            derivative_kind: MediaDerivativeKind::Thumbnail,
            target_width: 128,
            target_height: 128,
            format: "png".to_string(),
            requested_by: "mt-017-operator".to_string(),
        })
        .await
        .expect("request derivative for fake artifact proof");
    let before_generated_events = store
        .count_events_for_aggregate(
            event_family::MEDIA_DERIVATIVE_GENERATED,
            "atelier_media_derivative",
            &derivative.derivative_id.to_string(),
        )
        .await
        .expect("count generated events before fake artifact proof");

    let fake_artifact_id = Uuid::new_v4();
    let result = store
        .record_media_derivative_generated(
            derivative.derivative_id,
            &format!(
                "artifact://.handshake/artifacts/L1/{}/payload",
                fake_artifact_id
            ),
            &format!(
                "artifact://.handshake/artifacts/L1/{}/artifact.json",
                fake_artifact_id
            ),
            "image/png",
            64,
            "mt-017-worker",
        )
        .await;
    assert!(
        result
            .expect_err("fake ArtifactStore payload must be rejected")
            .to_string()
            .contains("ArtifactStore"),
        "fake ArtifactStore payload should fail the binding check"
    );

    let rows = store
        .list_media_derivatives(original.asset_id)
        .await
        .expect("list derivatives after fake artifact proof");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, MediaDerivativeStatus::Pending);
    assert!(rows[0].artifact_ref.is_none());
    assert_eq!(
        before_generated_events,
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_GENERATED,
                "atelier_media_derivative",
                &derivative.derivative_id.to_string(),
            )
            .await
            .expect("count generated events after fake artifact proof"),
        "fake generated artifact must not write EventLedger evidence"
    );
}

#[tokio::test]
async fn media_derivative_terminal_states_cannot_be_overwritten() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_derivative_terminal_states_cannot_be_overwritten: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let original = native_media_asset(&store, "mt-017-terminal-state").await;
    let derivative = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: original.asset_id,
            derivative_kind: MediaDerivativeKind::Proxy,
            target_width: 640,
            target_height: 640,
            format: "png".to_string(),
            requested_by: "mt-017-operator".to_string(),
        })
        .await
        .expect("request terminal-state derivative");
    store
        .mark_media_derivative_generating(derivative.derivative_id, "mt-017-worker")
        .await
        .expect("mark terminal-state derivative generating");
    let artifact = atelier_pg_support::write_native_media_artifact(b"proxy-generated");
    let generated = store
        .record_media_derivative_generated_with_artifact(
            &handshake_core::atelier::MediaDerivativeGenerated {
                derivative_id: derivative.derivative_id,
                artifact_ref: artifact.artifact_ref.clone(),
                artifact_manifest_ref: format!(
                    "artifact://.handshake/artifacts/L1/{}/artifact.json",
                    artifact.artifact_id
                ),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await
        .expect("record terminal-state generated derivative");
    assert_eq!(generated.status, MediaDerivativeStatus::Generated);

    let result = store
        .record_media_derivative_failure(
            derivative.derivative_id,
            &MediaDerivativeFailure {
                error_code: "late_worker_failure".to_string(),
                error_detail: "worker reported failure after generated".to_string(),
                retryable: true,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await;
    assert!(
        result
            .expect_err("generated derivative must not be overwritten by failure")
            .to_string()
            .contains("not active"),
        "late failure should be rejected as an invalid state transition"
    );

    let after = store
        .list_media_derivatives(original.asset_id)
        .await
        .expect("list derivatives after late failure");
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].status, MediaDerivativeStatus::Generated);
    assert_eq!(
        after[0].artifact_ref.as_deref(),
        Some(artifact.artifact_ref.as_str())
    );
}

#[tokio::test]
async fn media_derivative_duplicate_request_is_idempotent_after_generation() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_derivative_duplicate_request_is_idempotent_after_generation: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let original = native_media_asset(&store, "mt-017-duplicate-request").await;
    let request = MediaDerivativeRequest {
        asset_id: original.asset_id,
        derivative_kind: MediaDerivativeKind::Thumbnail,
        target_width: 256,
        target_height: 256,
        format: "png".to_string(),
        requested_by: "mt-017-operator".to_string(),
    };
    let derivative = store
        .request_media_derivative(&request)
        .await
        .expect("request derivative before duplicate proof");
    store
        .mark_media_derivative_generating(derivative.derivative_id, "mt-017-worker")
        .await
        .expect("mark derivative generating before duplicate proof");
    let artifact = atelier_pg_support::write_native_media_artifact(b"duplicate-generated-thumb");
    let generated = store
        .record_media_derivative_generated_with_artifact(
            &handshake_core::atelier::MediaDerivativeGenerated {
                derivative_id: derivative.derivative_id,
                artifact_ref: artifact.artifact_ref.clone(),
                artifact_manifest_ref: format!(
                    "artifact://.handshake/artifacts/L1/{}/artifact.json",
                    artifact.artifact_id
                ),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await
        .expect("record generated derivative before duplicate proof");
    assert_eq!(generated.status, MediaDerivativeStatus::Generated);
    let requested_events_before = store
        .count_events_for_aggregate(
            event_family::MEDIA_DERIVATIVE_REQUESTED,
            "atelier_media_derivative",
            &derivative.derivative_id.to_string(),
        )
        .await
        .expect("count request events before duplicate proof");

    let duplicate = store
        .request_media_derivative(&MediaDerivativeRequest {
            requested_by: "mt-017-late-operator".to_string(),
            ..request
        })
        .await
        .expect("duplicate derivative request should be idempotent");

    assert_eq!(duplicate.derivative_id, derivative.derivative_id);
    assert_eq!(duplicate.status, MediaDerivativeStatus::Generated);
    assert_eq!(duplicate.requested_by, "mt-017-operator");
    assert_eq!(duplicate.updated_by, "mt-017-worker");
    assert_eq!(
        duplicate.artifact_ref.as_deref(),
        Some(artifact.artifact_ref.as_str())
    );
    assert_eq!(
        requested_events_before,
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_REQUESTED,
                "atelier_media_derivative",
                &derivative.derivative_id.to_string(),
            )
            .await
            .expect("count request events after duplicate proof"),
        "duplicate derivative request must not emit a second request event"
    );
}

#[tokio::test]
async fn media_derivative_retryable_error_requires_explicit_retry_before_generating() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_derivative_retryable_error_requires_explicit_retry_before_generating: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let original = native_media_asset(&store, "mt-017-retry-required").await;
    let derivative = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: original.asset_id,
            derivative_kind: MediaDerivativeKind::Proxy,
            target_width: 640,
            target_height: 640,
            format: "jpeg".to_string(),
            requested_by: "mt-017-operator".to_string(),
        })
        .await
        .expect("request derivative before retry proof");
    let failed = store
        .record_media_derivative_failure(
            derivative.derivative_id,
            &MediaDerivativeFailure {
                error_code: "transient_render_error".to_string(),
                error_detail: "worker reported a retryable render error".to_string(),
                retryable: true,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await
        .expect("record retryable derivative failure");
    assert_eq!(failed.status, MediaDerivativeStatus::RetryableError);
    assert_eq!(failed.retry_count, 0);
    let generating_events_before = store
        .count_events_for_aggregate(
            event_family::MEDIA_DERIVATIVE_GENERATING,
            "atelier_media_derivative",
            &derivative.derivative_id.to_string(),
        )
        .await
        .expect("count generating events before retry bypass proof");

    let bypass = store
        .mark_media_derivative_generating(derivative.derivative_id, "mt-017-worker")
        .await
        .expect_err("retryable_error must not transition to generating without retry");
    assert!(
        bypass.to_string().contains("not pending") && bypass.to_string().contains("retry"),
        "retry bypass denial should require explicit retry: {bypass}"
    );
    let rows = store
        .list_media_derivatives(original.asset_id)
        .await
        .expect("list derivatives after retry bypass denial");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, MediaDerivativeStatus::RetryableError);
    assert_eq!(rows[0].retry_count, 0);
    assert_eq!(
        generating_events_before,
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_GENERATING,
                "atelier_media_derivative",
                &derivative.derivative_id.to_string(),
            )
            .await
            .expect("count generating events after retry bypass proof"),
        "retry bypass denial must not write generating evidence"
    );

    let retried = store
        .retry_media_derivative(derivative.derivative_id, "mt-017-operator")
        .await
        .expect("explicit retry should return derivative to pending");
    assert_eq!(retried.status, MediaDerivativeStatus::Pending);
    assert_eq!(retried.retry_count, 1);
    let generating = store
        .mark_media_derivative_generating(derivative.derivative_id, "mt-017-worker")
        .await
        .expect("retried derivative can transition from pending to generating");
    assert_eq!(generating.status, MediaDerivativeStatus::Generating);
}

#[tokio::test]
async fn media_derivative_generated_rejects_mime_format_mismatch() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_derivative_generated_rejects_mime_format_mismatch: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let original = native_media_asset(&store, "mt-017-mime-format-mismatch").await;
    let derivative = store
        .request_media_derivative(&MediaDerivativeRequest {
            asset_id: original.asset_id,
            derivative_kind: MediaDerivativeKind::Proxy,
            target_width: 640,
            target_height: 640,
            format: "jpeg".to_string(),
            requested_by: "mt-017-operator".to_string(),
        })
        .await
        .expect("request jpeg derivative");
    store
        .mark_media_derivative_generating(derivative.derivative_id, "mt-017-worker")
        .await
        .expect("mark jpeg derivative generating");
    let artifact = atelier_pg_support::write_native_media_artifact(b"png-for-jpeg-request");

    let result = store
        .record_media_derivative_generated_with_artifact(
            &handshake_core::atelier::MediaDerivativeGenerated {
                derivative_id: derivative.derivative_id,
                artifact_ref: artifact.artifact_ref.clone(),
                artifact_manifest_ref: format!(
                    "artifact://.handshake/artifacts/L1/{}/artifact.json",
                    artifact.artifact_id
                ),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                updated_by: "mt-017-worker".to_string(),
            },
        )
        .await;
    assert!(
        result
            .expect_err("jpeg derivative must reject png artifact")
            .to_string()
            .contains("format"),
        "format/MIME mismatch should be rejected before persistence"
    );

    let rows = store
        .list_media_derivatives(original.asset_id)
        .await
        .expect("list derivatives after format mismatch");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, MediaDerivativeStatus::Generating);
    assert!(rows[0].artifact_ref.is_none());
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_DERIVATIVE_GENERATED,
                "atelier_media_derivative",
                &derivative.derivative_id.to_string(),
            )
            .await
            .expect("count generated events after format mismatch"),
        0,
        "format/MIME mismatch must not write generated evidence"
    );
}

#[tokio::test]
async fn media_materialization_rejects_gov_local_network_and_bad_metadata() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_materialization_rejects_gov_local_network_and_bad_metadata: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    for forbidden_ref in [
        "artifact://.GOV/media/leak",
        "file:///tmp/media.png",
        "C:\\operator\\media.png",
        "https://example.test/media.png",
        "artifact://C:/operator/media.png",
        "artifact:///C:/operator/media.png",
        "artifact://localhost/media.png",
        "artifact://127.0.0.1/media.png",
        "artifact://atelier/../media.png",
        "artifact://atelier/media/with space.png",
    ] {
        let content_hash = sha256_token();
        let result = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: 2048,
                source_provenance: Some("negative-path-test".to_string()),
                artifact_ref: forbidden_ref.to_string(),
            })
            .await;
        assert!(
            result.is_err(),
            "forbidden artifact ref must be rejected: {forbidden_ref}"
        );
        assert_rejected_without_row_or_event(&store, &content_hash).await;
    }

    let artifact = atelier_pg_support::write_native_media_artifact(b"mt-016 bad hash bytes");
    for content_hash in [
        String::new(),
        "sha256:not-hex".to_string(),
        "sha256:abc".to_string(),
        "z".repeat(64),
    ] {
        let result = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                source_provenance: Some("negative-hash-test".to_string()),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await;
        assert!(result.is_err(), "bad hash must reject: {content_hash:?}");
        if !content_hash.is_empty() {
            assert_rejected_without_row_or_event(&store, &content_hash).await;
        }
    }

    for source_provenance in [None, Some("".to_string()), Some(" padded ".to_string())] {
        let artifact = atelier_pg_support::write_native_media_artifact(b"mt-016 bad source bytes");
        let content_hash = artifact.content_hash.clone();
        let result = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                source_provenance,
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await;
        assert!(result.is_err(), "source_provenance must be required");
        assert_rejected_without_row_or_event(&store, &content_hash).await;
    }

    for source_provenance in [
        "file:///operator/Leaky-Name.png",
        "C:\\operator\\Leaky-Name.png",
        "http://localhost:9000/Leaky-Name.png",
        ".GOV/leaky-name.png",
    ] {
        let artifact =
            atelier_pg_support::write_native_media_artifact(b"mt-007 bad provenance ref bytes");
        let content_hash = artifact.content_hash.clone();
        let result = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                source_provenance: Some(source_provenance.to_string()),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await;
        assert!(
            result.is_err(),
            "legacy or machine-local source_provenance must reject: {source_provenance}"
        );
        assert_rejected_without_row_or_event(&store, &content_hash).await;
    }

    let identity_mismatch =
        atelier_pg_support::write_native_media_artifact(b"mt-016 manifest identity mismatch");
    let identity_mismatch_hash = identity_mismatch.content_hash.clone();
    let manifest_path = identity_mismatch
        .workspace_root
        .join(artifact_root_rel(
            ArtifactLayer::L1,
            identity_mismatch.artifact_id,
        ))
        .join("artifact.json");
    let mut manifest_json: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&manifest_path).expect("read native ArtifactStore manifest"),
    )
    .expect("parse native ArtifactStore manifest");
    manifest_json["artifact_id"] = serde_json::json!(Uuid::now_v7());
    std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest_json).expect("serialize corrupt manifest"),
    )
    .expect("write corrupt native ArtifactStore manifest");
    let identity_result = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: identity_mismatch.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: identity_mismatch.byte_len,
            source_provenance: Some("negative-manifest-identity-test".to_string()),
            artifact_ref: identity_mismatch.artifact_ref.clone(),
        })
        .await;
    assert!(
        identity_result.is_err(),
        "ArtifactStore manifest identity mismatch must reject"
    );
    assert_rejected_without_row_or_event(&store, &identity_mismatch_hash).await;

    let wrong_size = atelier_pg_support::write_native_media_artifact(b"mt-016 wrong size bytes");
    let wrong_size_hash = wrong_size.content_hash.clone();
    let size_result = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: wrong_size.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: wrong_size.byte_len + 1,
            source_provenance: Some("negative-size-test".to_string()),
            artifact_ref: wrong_size.artifact_ref.clone(),
        })
        .await;
    assert!(
        size_result.is_err(),
        "ArtifactStore size mismatch must reject"
    );
    assert_rejected_without_row_or_event(&store, &wrong_size_hash).await;

    let wrong_mime = atelier_pg_support::write_native_media_artifact(b"mt-016 wrong mime bytes");
    let wrong_mime_hash = wrong_mime.content_hash.clone();
    let mime_result = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: wrong_mime.content_hash.clone(),
            mime: "image/jpeg".to_string(),
            byte_len: wrong_mime.byte_len,
            source_provenance: Some("negative-mime-test".to_string()),
            artifact_ref: wrong_mime.artifact_ref.clone(),
        })
        .await;
    assert!(
        mime_result.is_err(),
        "ArtifactStore MIME mismatch must reject"
    );
    assert_rejected_without_row_or_event(&store, &wrong_mime_hash).await;

    let padded_mime = atelier_pg_support::write_native_media_artifact(b"mt-016 padded mime bytes");
    let padded_mime_hash = padded_mime.content_hash.clone();
    let padded_mime_result = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: padded_mime.content_hash.clone(),
            mime: " image/png ".to_string(),
            byte_len: padded_mime.byte_len,
            source_provenance: Some("negative-padded-mime-test".to_string()),
            artifact_ref: padded_mime.artifact_ref.clone(),
        })
        .await;
    assert!(
        padded_mime_result.is_err(),
        "padded MIME must reject before persistence"
    );
    assert_rejected_without_row_or_event(&store, &padded_mime_hash).await;
}

#[tokio::test]
async fn media_manifest_backfill_repairs_existing_rows_on_schema_ensure() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP media_manifest_backfill_repairs_existing_rows_on_schema_ensure: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let native_artifact =
        atelier_pg_support::write_native_media_artifact(b"mt-016 native legacy media bytes");
    let native_asset_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO atelier_media_asset
             (asset_id, content_hash, mime, byte_len, source_provenance,
              artifact_ref, retention_class, artifact_manifest)
           VALUES ($1, $2, 'image/png', $3, 'legacy-import', $4, $5,
              jsonb_build_object(
                  'schema', $6::text,
                  'asset_id', $1::text,
                  'artifact_ref', 'artifact://stale/old',
                  'content_hash', 'stale',
                  'mime', 'image/png',
                  'byte_len', 1,
                  'size_bytes', 1,
                  'source', 'stale',
                  'source_provenance', 'stale',
                  'retention_class', $4::text,
                  'artifact_store', jsonb_build_object(
                      'handle', 'artifact://stale/old',
                      'content_hash', 'stale',
                      'size_bytes', 1,
                      'retention_class', $5::text
                  )
              ))"#,
    )
    .bind(native_asset_id)
    .bind(&native_artifact.content_hash)
    .bind(native_artifact.byte_len)
    .bind(&native_artifact.artifact_ref)
    .bind(MEDIA_ORIGINAL_RETENTION_CLASS)
    .bind(MEDIA_ARTIFACT_MANIFEST_SCHEMA)
    .execute(store.pool())
    .await
    .expect("insert native media row with stale manifest");

    let invalid_asset_id = Uuid::now_v7();
    let invalid_content_hash = sha256_token();
    let invalid_artifact_ref = "artifact://.GOV/media/leak";
    sqlx::query(
        r#"INSERT INTO atelier_media_asset
             (asset_id, content_hash, mime, byte_len, source_provenance,
              artifact_ref, retention_class, artifact_manifest)
           VALUES ($1, $2, 'image/png', 4096, 'legacy-import', $3, $4,
              jsonb_build_object(
                  'schema', $5::text,
                  'asset_id', $1::text,
                  'artifact_ref', $3::text,
                  'content_hash', 'stale',
                  'mime', 'image/png',
                  'byte_len', 1,
                  'size_bytes', 1,
                  'source', 'stale',
                  'source_provenance', 'stale',
                  'retention_class', $4::text,
                  'artifact_store', jsonb_build_object(
                      'handle', $3::text,
                      'content_hash', 'stale',
                      'size_bytes', 1,
                      'retention_class', $4::text
                  )
              ))"#,
    )
    .bind(invalid_asset_id)
    .bind(&invalid_content_hash)
    .bind(invalid_artifact_ref)
    .bind(MEDIA_ORIGINAL_RETENTION_CLASS)
    .bind(MEDIA_ARTIFACT_MANIFEST_SCHEMA)
    .execute(store.pool())
    .await
    .expect("insert invalid legacy media row with stale manifest");

    let bad_native_shape_asset_id = Uuid::now_v7();
    let bad_native_shape_hash = format!("not-a-sha256-{}", Uuid::new_v4());
    let bad_native_shape_ref = format!(
        "artifact://.handshake/artifacts/L1/{}/payload",
        Uuid::new_v4()
    );
    sqlx::query(
        r#"INSERT INTO atelier_media_asset
             (asset_id, content_hash, mime, byte_len, source_provenance,
              artifact_ref, retention_class, artifact_manifest)
           VALUES ($1, $5, ' image/png ', 0, 'legacy-import', $2, $3,
              jsonb_build_object(
                  'schema', $4::text,
                  'asset_id', $1::text,
                  'artifact_ref', $2::text,
                  'content_hash', $5::text,
                  'mime', ' image/png ',
                  'byte_len', 0,
                  'size_bytes', 0,
                  'source', 'legacy-import',
                  'source_provenance', 'legacy-import',
                  'retention_class', $3::text,
                  'artifact_store', jsonb_build_object(
                      'handle', $2::text,
                      'content_hash', $5::text,
                      'size_bytes', 0,
                      'retention_class', $3::text
                  )
              ))"#,
    )
    .bind(bad_native_shape_asset_id)
    .bind(&bad_native_shape_ref)
    .bind(MEDIA_ORIGINAL_RETENTION_CLASS)
    .bind(MEDIA_ARTIFACT_MANIFEST_SCHEMA)
    .bind(&bad_native_shape_hash)
    .execute(store.pool())
    .await
    .expect("insert native-shaped legacy row with bad metadata");

    let missing_native_asset_id = Uuid::now_v7();
    let missing_native_hash = sha256_token();
    let missing_native_ref = format!(
        "artifact://.handshake/artifacts/L1/{}/payload",
        Uuid::new_v4()
    );
    sqlx::query(
        r#"INSERT INTO atelier_media_asset
             (asset_id, content_hash, mime, byte_len, source_provenance,
              artifact_ref, retention_class, artifact_manifest)
           VALUES ($1, $2, 'image/png', 128, 'legacy-import', $3, $4, '{}'::jsonb)"#,
    )
    .bind(missing_native_asset_id)
    .bind(&missing_native_hash)
    .bind(&missing_native_ref)
    .bind(MEDIA_ORIGINAL_RETENTION_CLASS)
    .execute(store.pool())
    .await
    .expect("insert native-shaped legacy row without ArtifactStore payload");

    store
        .ensure_schema()
        .await
        .expect("ensure_schema repairs media manifests");

    let repaired_native = store
        .get_media_asset_by_hash(&native_artifact.content_hash)
        .await
        .expect("fetch repaired native media row")
        .expect("native media row exists");
    assert_eq!(
        repaired_native.artifact_manifest["schema"],
        MEDIA_ARTIFACT_MANIFEST_SCHEMA
    );
    assert_eq!(
        repaired_native.artifact_manifest["artifact_ref"],
        native_artifact.artifact_ref
    );
    assert_eq!(
        repaired_native.artifact_manifest["content_hash"],
        native_artifact.content_hash
    );
    assert_eq!(
        repaired_native.artifact_manifest["size_bytes"],
        native_artifact.byte_len
    );
    assert!(
        repaired_native.artifact_manifest.get("source").is_none(),
        "repaired native manifest must remove raw source text"
    );
    assert!(
        repaired_native
            .artifact_manifest
            .get("source_provenance")
            .is_none(),
        "repaired native manifest must remove raw source provenance"
    );
    assert!(
        repaired_native.artifact_manifest["source_provenance_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "repaired native manifest must retain a content-addressed provenance ref"
    );
    assert_eq!(
        repaired_native.artifact_manifest["artifact_store"]["handle"],
        native_artifact.artifact_ref
    );
    assert_eq!(
        repaired_native.artifact_manifest["retention_class"],
        MEDIA_ORIGINAL_RETENTION_CLASS
    );

    let repaired_invalid = store
        .get_media_asset_by_hash(&invalid_content_hash)
        .await
        .expect("fetch repaired invalid legacy row")
        .expect("invalid legacy media row exists");
    assert_eq!(
        repaired_invalid.artifact_manifest["schema"],
        MEDIA_ARTIFACT_MANIFEST_SCHEMA
    );
    assert_eq!(
        repaired_invalid.artifact_manifest["validation_state"],
        "invalid_legacy_artifact_ref"
    );
    assert_eq!(
        repaired_invalid.artifact_manifest["artifact_store"]["status"],
        "unresolved"
    );
    assert!(
        repaired_invalid
            .artifact_manifest
            .get("artifact_ref")
            .is_none(),
        "invalid legacy manifest must not preserve forbidden artifact_ref"
    );
    assert!(
        repaired_invalid.artifact_manifest["artifact_store"]
            .get("handle")
            .is_none(),
        "invalid legacy manifest must not preserve forbidden ArtifactStore handle"
    );
    assert!(
        !repaired_invalid
            .artifact_manifest
            .to_string()
            .to_ascii_lowercase()
            .contains(".gov"),
        "invalid legacy manifest must not copy .GOV handles"
    );

    let bad_native_shape_manifest: serde_json::Value =
        sqlx::query_scalar("SELECT artifact_manifest FROM atelier_media_asset WHERE asset_id = $1")
            .bind(bad_native_shape_asset_id)
            .fetch_one(store.pool())
            .await
            .expect("fetch bad native-shaped manifest");
    assert_eq!(
        bad_native_shape_manifest["validation_state"], "invalid_legacy_artifact_ref",
        "native-shaped refs with invalid row metadata must be quarantined"
    );
    assert!(
        bad_native_shape_manifest["artifact_store"]
            .get("handle")
            .is_none(),
        "bad native-shaped legacy manifest must not claim a validated handle"
    );

    let missing_native_manifest: serde_json::Value =
        sqlx::query_scalar("SELECT artifact_manifest FROM atelier_media_asset WHERE asset_id = $1")
            .bind(missing_native_asset_id)
            .fetch_one(store.pool())
            .await
            .expect("fetch missing native-shaped manifest");
    assert_eq!(
        missing_native_manifest["validation_state"], "invalid_artifact_store_binding",
        "native-shaped refs without reachable ArtifactStore payloads must be quarantined"
    );
    assert!(!missing_native_manifest
        .get("artifact_ref")
        .is_some_and(|value| value == &serde_json::json!(missing_native_ref)));
    assert!(
        missing_native_manifest["artifact_store"]
            .get("handle")
            .is_none(),
        "unverified native-shaped manifest must not claim a validated handle"
    );
}

#[tokio::test]
async fn materialization_upgrades_same_hash_quarantined_legacy_row_to_native_artifact() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP materialization_upgrades_same_hash_quarantined_legacy_row_to_native_artifact: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let artifact =
        atelier_pg_support::write_native_media_artifact(b"mt-016 same hash legacy upgrade");
    let asset_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO atelier_media_asset
             (asset_id, content_hash, mime, byte_len, source_provenance,
              artifact_ref, retention_class, artifact_manifest)
           VALUES ($1, $2, 'image/png', $3, 'legacy-import',
              'artifact://.GOV/media/leak', $4,
              jsonb_build_object(
                  'schema', $5::text,
                  'asset_id', $1::text,
                  'content_hash', $2::text,
                  'mime', 'image/png',
                  'byte_len', $3,
                  'size_bytes', $3,
                  'source', 'legacy-import',
                  'source_provenance', 'legacy-import',
                  'retention_class', $4::text,
                  'validation_state', 'invalid_legacy_artifact_ref',
                  'artifact_store', jsonb_build_object(
                      'status', 'unresolved',
                      'reason', 'legacy artifact_ref is not a native ArtifactStore payload handle'
                  )
              ))"#,
    )
    .bind(asset_id)
    .bind(&artifact.content_hash)
    .bind(artifact.byte_len)
    .bind(MEDIA_ORIGINAL_RETENTION_CLASS)
    .bind(MEDIA_ARTIFACT_MANIFEST_SCHEMA)
    .execute(store.pool())
    .await
    .expect("insert quarantined legacy row with same content hash");

    let upgraded = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("artifact-store:legacy-upgrade".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("upgrade same-hash legacy media row through native ArtifactStore");

    assert_eq!(upgraded.asset_id, asset_id);
    assert_eq!(upgraded.artifact_ref, artifact.artifact_ref);
    assert_eq!(
        upgraded.artifact_manifest["artifact_store"]["handle"],
        artifact.artifact_ref
    );
    assert_eq!(
        upgraded.artifact_manifest["validation_state"],
        serde_json::Value::Null,
        "upgraded native manifest must not remain quarantined"
    );
    assert!(
        !upgraded
            .artifact_manifest
            .to_string()
            .to_ascii_lowercase()
            .contains(".gov"),
        "upgraded manifest must not preserve the old .GOV handle"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_ASSET_MATERIALIZED,
                "atelier_media_asset",
                &artifact.content_hash,
            )
            .await
            .expect("count upgrade materialization event"),
        1,
        "legacy upgrade must append one canonical materialization event"
    );
}

#[tokio::test]
async fn materialization_repairs_same_hash_unverified_native_row_to_valid_artifact() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP materialization_repairs_same_hash_unverified_native_row_to_valid_artifact: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let artifact =
        atelier_pg_support::write_native_media_artifact(b"mt-016 unverified native repair");
    let stale_asset_id = Uuid::now_v7();
    let fake_artifact_ref = format!(
        "artifact://.handshake/artifacts/L1/{}/payload",
        Uuid::new_v4()
    );
    sqlx::query(
        r#"INSERT INTO atelier_media_asset
             (asset_id, content_hash, mime, byte_len, source_provenance,
              artifact_ref, retention_class, artifact_manifest)
           VALUES ($1, $2, 'image/png', $3, 'legacy-import', $4, $5, '{}'::jsonb)"#,
    )
    .bind(stale_asset_id)
    .bind(&artifact.content_hash)
    .bind(artifact.byte_len)
    .bind(&fake_artifact_ref)
    .bind(MEDIA_ORIGINAL_RETENTION_CLASS)
    .execute(store.pool())
    .await
    .expect("insert same-hash row with unverified native ArtifactStore handle");

    let repaired = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("artifact-store:unverified-native-repair".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("repair same-hash unverified native row through valid ArtifactStore");

    assert_eq!(repaired.asset_id, stale_asset_id);
    assert_eq!(repaired.artifact_ref, artifact.artifact_ref);
    assert_eq!(
        repaired.artifact_manifest["artifact_store"]["handle"],
        artifact.artifact_ref
    );
    assert_eq!(
        repaired.artifact_manifest["validation_state"],
        serde_json::Value::Null,
        "repaired native manifest must not remain quarantined"
    );
    assert!(
        !repaired
            .artifact_manifest
            .to_string()
            .contains(&fake_artifact_ref),
        "repaired manifest must not preserve the old unverified ArtifactStore handle"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_ASSET_MATERIALIZED,
                "atelier_media_asset",
                &artifact.content_hash,
            )
            .await
            .expect("count unverified native repair materialization event"),
        1,
        "unverified native repair must append one canonical materialization event"
    );
}

#[tokio::test]
async fn concurrent_legacy_media_upgrade_records_one_event_and_manifest() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP concurrent_legacy_media_upgrade_records_one_event_and_manifest: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let artifact = atelier_pg_support::write_native_media_artifact(
        b"mt-016 concurrent same hash legacy upgrade",
    );
    let asset_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO atelier_media_asset
             (asset_id, content_hash, mime, byte_len, source_provenance,
              artifact_ref, retention_class, artifact_manifest)
           VALUES ($1, $2, 'image/png', $3, 'legacy-import',
              'artifact://.GOV/media/concurrent-leak', $4,
              jsonb_build_object(
                  'schema', $5::text,
                  'asset_id', $1::text,
                  'content_hash', $2::text,
                  'mime', 'image/png',
                  'byte_len', $3,
                  'size_bytes', $3,
                  'source', 'legacy-import',
                  'source_provenance', 'legacy-import',
                  'retention_class', $4::text,
                  'validation_state', 'invalid_legacy_artifact_ref',
                  'artifact_store', jsonb_build_object(
                      'status', 'unresolved',
                      'reason', 'legacy artifact_ref is not a native ArtifactStore payload handle'
                  )
              ))"#,
    )
    .bind(asset_id)
    .bind(&artifact.content_hash)
    .bind(artifact.byte_len)
    .bind(MEDIA_ORIGINAL_RETENTION_CLASS)
    .bind(MEDIA_ARTIFACT_MANIFEST_SCHEMA)
    .execute(store.pool())
    .await
    .expect("insert quarantined legacy row for concurrent upgrade");

    let content_hash = artifact.content_hash.clone();
    let artifact_ref = artifact.artifact_ref.clone();
    let byte_len = artifact.byte_len;
    let requests = (0..12).map(|_| {
        let store = store.clone();
        let content_hash = content_hash.clone();
        let artifact_ref = artifact_ref.clone();
        async move {
            store
                .materialize_media_asset(&NewMediaAsset {
                    content_hash,
                    mime: "image/png".to_string(),
                    byte_len,
                    source_provenance: Some("concurrent-legacy-upgrade-mt-016".to_string()),
                    artifact_ref,
                })
                .await
                .expect("concurrent legacy media upgrade")
        }
    });
    let assets = join_all(requests).await;
    assert!(assets.iter().all(|asset| asset.asset_id == asset_id));
    assert!(assets.iter().all(|asset| {
        asset.artifact_ref == artifact.artifact_ref
            && asset.artifact_manifest["artifact_store"]["handle"] == artifact.artifact_ref
            && !asset
                .artifact_manifest
                .to_string()
                .to_ascii_lowercase()
                .contains(".gov")
    }));
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_ASSET_MATERIALIZED,
                "atelier_media_asset",
                &artifact.content_hash,
            )
            .await
            .expect("count concurrent legacy upgrade event"),
        1,
        "concurrent legacy upgrade must emit exactly one materialization event"
    );
}

#[tokio::test]
async fn concurrent_media_materialization_records_one_event_and_manifest() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP concurrent_media_materialization_records_one_event_and_manifest: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let artifact =
        atelier_pg_support::write_native_media_artifact(b"mt-016 concurrent media bytes");
    let content_hash = artifact.content_hash.clone();
    let artifact_ref = artifact.artifact_ref.clone();
    let byte_len = artifact.byte_len;
    let requests = (0..12).map(|_| {
        let store = store.clone();
        let content_hash = content_hash.clone();
        let artifact_ref = artifact_ref.clone();
        async move {
            store
                .materialize_media_asset(&NewMediaAsset {
                    content_hash,
                    mime: "image/png".to_string(),
                    byte_len,
                    source_provenance: Some("concurrent-mt-016".to_string()),
                    artifact_ref,
                })
                .await
                .expect("concurrent materialization")
        }
    });
    let assets = join_all(requests).await;
    let first_id = assets[0].asset_id;
    assert!(assets.iter().all(|asset| asset.asset_id == first_id));
    assert!(assets.iter().all(|asset| {
        asset.artifact_manifest["schema"] == MEDIA_ARTIFACT_MANIFEST_SCHEMA
            && asset.artifact_manifest["size_bytes"] == byte_len
            && asset.artifact_manifest.get("source").is_none()
            && asset.artifact_manifest["source_provenance_ref"]
                .as_str()
                .is_some_and(|value| value.starts_with("sha256:"))
    }));
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_ASSET_MATERIALIZED,
                "atelier_media_asset",
                &content_hash,
            )
            .await
            .expect("count concurrent materialization event"),
        1,
        "concurrent insert conflict path must not emit duplicate events"
    );
}

async fn assert_rejected_without_row_or_event(store: &AtelierStore, content_hash: &str) {
    assert!(
        store
            .get_media_asset_by_hash(content_hash)
            .await
            .expect("query rejected media hash")
            .is_none(),
        "rejected media materialization must not create rows"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_ASSET_MATERIALIZED,
                "atelier_media_asset",
                content_hash,
            )
            .await
            .expect("count leaked materialization event"),
        0,
        "rejected media materialization must not create EventLedger rows"
    );
}

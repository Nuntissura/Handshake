//! WP-KERNEL-005 MT-025 clipboard and URL image import proof.
//!
//! Uses the embedded SurrealDB/EventLedger path. Clipboard import must materialize an
//! operator-provided ArtifactStore image with provenance; URL import must record
//! a governed fetch request with media-downloader capability proof and SSRF
//! preflight, without opening sockets in the atelier repository layer.

mod atelier_surreal_support;

use handshake_core::atelier::{
    event_family, AtelierStore, ClipboardImageImportRequest, UrlImageImportRequest,
};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::Database;
use serde_json::Value;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;

async fn connected_store() -> (AtelierStore, atelier_surreal_support::AtelierSurrealHarness) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    (harness.atelier.clone(), harness)
}

async fn connected_store_with_recorder(
    recorder: Arc<dyn FlightRecorder>,
) -> (AtelierStore, atelier_surreal_support::AtelierSurrealHarness) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    let store = AtelierStore::with_observability(
        harness.storage.clone(),
        harness.database.clone(),
        recorder,
    );
    (store, harness)
}

#[derive(Clone, Default)]
struct FailingFlightRecorder;

#[async_trait::async_trait]
impl FlightRecorder for FailingFlightRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Err(RecorderError::SinkError("forced recorder failure".into()))
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(vec![])
    }
}

async fn connected_store_with_failing_recorder(
) -> (AtelierStore, atelier_surreal_support::AtelierSurrealHarness) {
    connected_store_with_recorder(Arc::new(FailingFlightRecorder)).await
}

struct FailAfterFlightRecorder {
    remaining_successes: AtomicUsize,
}

impl FailAfterFlightRecorder {
    fn new(remaining_successes: usize) -> Self {
        Self {
            remaining_successes: AtomicUsize::new(remaining_successes),
        }
    }
}

#[async_trait::async_trait]
impl FlightRecorder for FailAfterFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        let update = self.remaining_successes.fetch_update(
            Ordering::SeqCst,
            Ordering::SeqCst,
            |remaining| {
                if remaining > 0 {
                    Some(remaining - 1)
                } else {
                    None
                }
            },
        );
        if update.is_ok() {
            Ok(())
        } else {
            Err(RecorderError::SinkError("forced recorder failure".into()))
        }
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(vec![])
    }
}

fn media_downloader_grant(profile: &str) -> String {
    format!(
        "capgrant://media_downloader/{profile}/evidence-{}",
        Uuid::new_v4()
    )
}

async fn image_import_request_count_by_key(
    harness: &atelier_surreal_support::AtelierSurrealHarness,
    idempotency_key: &str,
) -> i64 {
    harness
        .row_count_by_field(
            "atelier_image_import_request",
            "idempotency_key",
            idempotency_key,
        )
        .await as i64
}

async fn image_import_request_count_by_requester(
    harness: &atelier_surreal_support::AtelierSurrealHarness,
    requested_by: &str,
) -> i64 {
    harness
        .row_count_by_field("atelier_image_import_request", "requested_by", requested_by)
        .await as i64
}

async fn image_import_event_count_by_requester(
    harness: &atelier_surreal_support::AtelierSurrealHarness,
    requested_by: &str,
) -> i64 {
    harness
        .row_count_by_field("atelier_image_import_request", "requested_by", requested_by)
        .await as i64
}

async fn media_asset_count_for_hash(store: &AtelierStore, content_hash: &str) -> i64 {
    if store
        .get_media_asset_by_hash(content_hash)
        .await
        .expect("lookup media asset by hash")
        .is_some()
    {
        1
    } else {
        0
    }
}

async fn import_event_payload(database: &Arc<dyn Database>, import_id: Uuid) -> Value {
    database
        .list_kernel_events_for_aggregate("atelier_image_import_request", &import_id.to_string())
        .await
        .expect("list image import EventLedger events")
        .into_iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"] == event_family::IMAGE_IMPORT_RECORDED
        })
        .map(|event| event.payload["atelier_payload"].clone())
        .expect("read image import event payload")
}

#[tokio::test]
async fn clipboard_image_import_materializes_artifactstore_asset_with_provenance() {
    let (store, _harness) = connected_store().await;
    let artifact = atelier_surreal_support::write_native_media_artifact(b"mt-025 clipboard bytes");

    let record = store
        .import_clipboard_image(&ClipboardImageImportRequest {
            idempotency_key: format!("clipboard-import-{}", Uuid::new_v4()),
            mime: "image/png".to_string(),
            content_hash: artifact.content_hash.clone(),
            byte_len: artifact.byte_len,
            artifact_ref: artifact.artifact_ref.clone(),
            source_application: Some("system-clipboard".to_string()),
            requested_by: "operator-clipboard".to_string(),
        })
        .await
        .expect("clipboard import materializes media asset");

    assert_eq!(record.source_kind, "clipboard");
    assert_eq!(record.status, "materialized");
    assert_eq!(record.requested_by, "operator-clipboard");
    let asset_id = record.asset_id.expect("clipboard import returns asset_id");

    let asset = store
        .get_media_asset_by_hash(&artifact.content_hash)
        .await
        .expect("lookup materialized asset")
        .expect("materialized asset exists");
    assert_eq!(asset.asset_id, asset_id);
    assert_eq!(asset.artifact_ref, artifact.artifact_ref);
    assert_eq!(
        asset.source_provenance.as_deref(),
        Some("clipboard:system-clipboard")
    );
    assert!(
        !asset
            .artifact_manifest
            .to_string()
            .to_ascii_lowercase()
            .contains(".gov"),
        "clipboard import must preserve ArtifactStore provenance without .GOV paths"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::MEDIA_ASSET_MATERIALIZED,
                "atelier_media_asset",
                &artifact.content_hash,
            )
            .await
            .expect("count media materialization event"),
        1
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::IMAGE_IMPORT_RECORDED,
                "atelier_image_import_request",
                &record.import_id.to_string(),
            )
            .await
            .expect("count image import event"),
        1
    );
}

#[tokio::test]
async fn url_image_import_records_capability_gated_fetch_request_without_materializing() {
    let (store, harness) = connected_store().await;
    let raw_url = format!(
        "https://example.com/images/{}.png?token=raw-secret#fragment",
        Uuid::new_v4()
    );

    let record = store
        .record_url_image_import(&UrlImageImportRequest {
            idempotency_key: format!("url-import-{}", Uuid::new_v4()),
            source_url: raw_url.clone(),
            expected_mime: Some("image/png".to_string()),
            source_label: Some("example url image".to_string()),
            capability_profile_id: "MediaDownloader".to_string(),
            capability_grant_ref: media_downloader_grant("MediaDownloader"),
            requested_by: "operator-url".to_string(),
        })
        .await
        .expect("URL import request is accepted after SSRF and capability preflight");

    assert_eq!(record.source_kind, "url");
    assert_eq!(record.status, "queued");
    assert_eq!(record.requested_by, "operator-url");
    assert!(
        record.asset_id.is_none(),
        "URL import preflight must not fetch or materialize bytes"
    );
    assert!(
        record.source_url_hash.starts_with("sha256:"),
        "URL provenance must be represented by a stable hash ref"
    );
    let normalized_url = record
        .normalized_url
        .as_deref()
        .expect("redacted display URL");
    assert!(
        !normalized_url.contains("raw-secret")
            && !normalized_url.contains("fragment")
            && !normalized_url.contains(&raw_url),
        "URL import record must not expose query secrets or fragments"
    );

    let event_payload = import_event_payload(&harness.database, record.import_id).await;
    assert_eq!(event_payload["source_url_ref"], record.source_url_hash);
    assert_eq!(event_payload["network_fetch_allowed"], false);
    assert_eq!(event_payload["requires_fetch_worker_revalidation"], true);
    assert_eq!(
        event_payload["redirect_policy"],
        "disabled_until_revalidated"
    );
    let serialized_payload = event_payload.to_string();
    assert!(
        !serialized_payload.contains("raw-secret")
            && !serialized_payload.contains("/images/")
            && !serialized_payload.contains(&raw_url),
        "EventLedger payload must not leak the raw URL or query token"
    );

    let denied_key = format!("url-import-denied-{}", Uuid::new_v4());
    let denied_actor = format!("operator-url-denied-{}", Uuid::new_v4());
    let capability_err = store
        .record_url_image_import(&UrlImageImportRequest {
            idempotency_key: denied_key.clone(),
            source_url: format!("https://example.com/images/{}.png", Uuid::new_v4()),
            expected_mime: Some("image/png".to_string()),
            source_label: None,
            capability_profile_id: "Analyst".to_string(),
            capability_grant_ref: media_downloader_grant("Analyst"),
            requested_by: denied_actor.clone(),
        })
        .await
        .expect_err("URL import must reject profiles lacking media-downloader capabilities");
    assert!(
        capability_err
            .to_string()
            .contains("capability profile Analyst"),
        "unexpected capability error: {capability_err}"
    );
    assert_eq!(
        image_import_request_count_by_key(&harness, &denied_key).await,
        0,
        "capability failure must not persist the rejected idempotency key"
    );
    assert_eq!(
        image_import_event_count_by_requester(&harness, &denied_actor).await,
        0,
        "capability failure must happen before persistence"
    );
}

#[tokio::test]
async fn url_image_import_blocks_ssrf_targets_before_persistence() {
    let (store, harness) = connected_store().await;
    let blocked_actor = format!("operator-url-blocked-{}", Uuid::new_v4());
    let mut blocked_keys = Vec::new();

    let blocked_targets = [
        "http://localhost/image.png",
        "http://127.0.0.1/image.png",
        "http://10.10.1.2/image.png",
        "http://172.16.5.4/image.png",
        "http://192.168.1.40/image.png",
        "http://169.254.169.254/latest/meta-data",
        "http://[::1]/image.png",
        "http://[fe80::1]/image.png",
        "http://2130706433/image.png",
        "http://0x7f000001/image.png",
        "http://0177.0.0.1/image.png",
        "http://127.1/image.png",
        "file:///tmp/image.png",
        "https://user:pass@example.com/private.png",
    ];

    for source_url in blocked_targets {
        let idempotency_key = format!("url-import-blocked-{}", Uuid::new_v4());
        blocked_keys.push(idempotency_key.clone());
        let err = store
            .record_url_image_import(&UrlImageImportRequest {
                idempotency_key,
                source_url: source_url.to_string(),
                expected_mime: Some("image/png".to_string()),
                source_label: None,
                capability_profile_id: "MediaDownloader".to_string(),
                capability_grant_ref: media_downloader_grant("MediaDownloader"),
                requested_by: blocked_actor.clone(),
            })
            .await
            .expect_err("blocked URL target must fail preflight");
        assert!(
            err.to_string().contains("url")
                || err.to_string().contains("SSRF")
                || err.to_string().contains("credentials"),
            "unexpected SSRF preflight error for {source_url}: {err}"
        );
    }

    assert_eq!(
        image_import_request_count_by_requester(&harness, &blocked_actor).await,
        0,
        "SSRF failures must not create import request rows for the blocked actor"
    );
    for idempotency_key in blocked_keys {
        assert_eq!(
            image_import_request_count_by_key(&harness, &idempotency_key).await,
            0,
            "SSRF failure must not persist rejected idempotency key {idempotency_key}"
        );
    }
    assert_eq!(
        image_import_event_count_by_requester(&harness, &blocked_actor).await,
        0,
        "SSRF failures must not emit EventLedger events"
    );
}

#[tokio::test]
async fn url_image_import_redacts_path_material_before_persistence() {
    let (store, harness) = connected_store().await;
    let secret_slug = format!("private-token-{}", Uuid::new_v4());
    let raw_url = format!(
        "https://cdn.example.com/{secret_slug}/faces/source.png?sig=query-secret#fragment-secret"
    );

    let record = store
        .record_url_image_import(&UrlImageImportRequest {
            idempotency_key: format!("url-import-redaction-{}", Uuid::new_v4()),
            source_url: raw_url.clone(),
            expected_mime: Some("image/png".to_string()),
            source_label: Some("path redaction proof".to_string()),
            capability_profile_id: "MediaDownloader".to_string(),
            capability_grant_ref: media_downloader_grant("MediaDownloader"),
            requested_by: "operator-url".to_string(),
        })
        .await
        .expect("URL import request is recorded");

    let normalized_url = record
        .normalized_url
        .as_deref()
        .expect("redacted display URL is stored");
    assert_eq!(record.source_host.as_deref(), Some("cdn.example.com"));
    assert!(
        normalized_url.starts_with("https://cdn.example.com/"),
        "redacted display URL should keep scheme and host: {normalized_url}"
    );
    assert!(
        !normalized_url.contains(&secret_slug)
            && !normalized_url.contains("faces")
            && !normalized_url.contains("source.png")
            && !normalized_url.contains("query-secret")
            && !normalized_url.contains("fragment-secret")
            && !normalized_url.contains(&raw_url),
        "stored normalized URL must redact path, query, and fragment material: {normalized_url}"
    );

    let inspector = harness.storage.test_inspector();
    let table = inspector
        .table_selector("atelier_image_import_request")
        .await
        .expect("inspect image import table");
    let projected = inspector
        .project(
            &table,
            &[table
                .field("normalized_url")
                .expect("inspect normalized_url field")],
            handshake_core::storage::surreal::RowFilter::IdEquals(record.import_id.to_string()),
        )
        .await
        .expect("read persisted normalized URL");
    assert_eq!(projected.len(), 1);
    assert_eq!(
        projected[0].values["normalized_url"].as_str(),
        Some(normalized_url)
    );
}

#[tokio::test]
async fn url_image_import_rejects_padded_capability_refs_before_persistence() {
    let (store, harness) = connected_store().await;
    let padded_actor = format!("operator-url-padded-{}", Uuid::new_v4());
    let padded_profile_key = format!("url-import-padded-profile-{}", Uuid::new_v4());
    let padded_grant_key = format!("url-import-padded-grant-{}", Uuid::new_v4());

    let padded_profile = store
        .record_url_image_import(&UrlImageImportRequest {
            idempotency_key: padded_profile_key.clone(),
            source_url: format!("https://example.com/imports/{}.png", Uuid::new_v4()),
            expected_mime: Some("image/png".to_string()),
            source_label: None,
            capability_profile_id: " MediaDownloader ".to_string(),
            capability_grant_ref: media_downloader_grant("MediaDownloader"),
            requested_by: padded_actor.clone(),
        })
        .await
        .expect_err("padded capability profile must be rejected before persistence");
    assert!(
        padded_profile.to_string().contains("capability_profile_id"),
        "unexpected padded profile error: {padded_profile}"
    );

    let padded_grant = store
        .record_url_image_import(&UrlImageImportRequest {
            idempotency_key: padded_grant_key.clone(),
            source_url: format!("https://example.com/imports/{}.png", Uuid::new_v4()),
            expected_mime: Some("image/png".to_string()),
            source_label: None,
            capability_profile_id: "MediaDownloader".to_string(),
            capability_grant_ref: format!(" {} ", media_downloader_grant("MediaDownloader")),
            requested_by: padded_actor.clone(),
        })
        .await
        .expect_err("padded capability grant must be rejected before persistence");
    assert!(
        padded_grant.to_string().contains("capability_grant_ref"),
        "unexpected padded grant error: {padded_grant}"
    );

    assert_eq!(
        image_import_request_count_by_requester(&harness, &padded_actor).await,
        0,
        "padded capability refs must not create import request rows for the padded actor"
    );
    assert_eq!(
        image_import_request_count_by_key(&harness, &padded_profile_key).await,
        0,
        "padded capability profile must not persist its idempotency key"
    );
    assert_eq!(
        image_import_request_count_by_key(&harness, &padded_grant_key).await,
        0,
        "padded capability grant must not persist its idempotency key"
    );
    assert_eq!(
        image_import_event_count_by_requester(&harness, &padded_actor).await,
        0,
        "padded capability refs must not emit EventLedger events"
    );
}

#[tokio::test]
async fn image_import_idempotency_key_rejects_mismatched_replays() {
    let (store, _harness) = connected_store().await;

    let url_key = format!("url-import-replay-{}", Uuid::new_v4());
    let first = store
        .record_url_image_import(&UrlImageImportRequest {
            idempotency_key: url_key.clone(),
            source_url: format!("https://example.com/imports/{}.png", Uuid::new_v4()),
            expected_mime: Some("image/png".to_string()),
            source_label: Some("first url".to_string()),
            capability_profile_id: "MediaDownloader".to_string(),
            capability_grant_ref: media_downloader_grant("MediaDownloader"),
            requested_by: "operator-url-replay".to_string(),
        })
        .await
        .expect("first URL import succeeds");
    let replay_event_count = store
        .count_events_for_aggregate(
            event_family::IMAGE_IMPORT_RECORDED,
            "atelier_image_import_request",
            &first.import_id.to_string(),
        )
        .await
        .expect("count initial URL import events");
    let mismatch = store
        .record_url_image_import(&UrlImageImportRequest {
            idempotency_key: url_key,
            source_url: format!("https://example.com/imports/{}.png", Uuid::new_v4()),
            expected_mime: Some("image/png".to_string()),
            source_label: Some("second url".to_string()),
            capability_profile_id: "MediaDownloader".to_string(),
            capability_grant_ref: media_downloader_grant("MediaDownloader"),
            requested_by: "operator-url-replay".to_string(),
        })
        .await
        .expect_err("mismatched URL replay must be rejected");
    assert!(
        mismatch.to_string().contains("idempotency_key"),
        "unexpected URL replay error: {mismatch}"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::IMAGE_IMPORT_RECORDED,
                "atelier_image_import_request",
                &first.import_id.to_string(),
            )
            .await
            .expect("count final URL import events"),
        replay_event_count,
        "mismatched URL replay must not emit a second import event"
    );

    let artifact =
        atelier_surreal_support::write_native_media_artifact(b"mt-025 replay clipboard one");
    let clipboard_key = format!("clipboard-import-replay-{}", Uuid::new_v4());
    store
        .import_clipboard_image(&ClipboardImageImportRequest {
            idempotency_key: clipboard_key.clone(),
            mime: "image/png".to_string(),
            content_hash: artifact.content_hash.clone(),
            byte_len: artifact.byte_len,
            artifact_ref: artifact.artifact_ref.clone(),
            source_application: Some("system-clipboard".to_string()),
            requested_by: "operator-clipboard-replay".to_string(),
        })
        .await
        .expect("first clipboard import succeeds");
    let other_artifact =
        atelier_surreal_support::write_native_media_artifact(b"mt-025 replay clipboard two");
    let clipboard_mismatch = store
        .import_clipboard_image(&ClipboardImageImportRequest {
            idempotency_key: clipboard_key,
            mime: "image/png".to_string(),
            content_hash: other_artifact.content_hash,
            byte_len: other_artifact.byte_len,
            artifact_ref: other_artifact.artifact_ref,
            source_application: Some("system-clipboard".to_string()),
            requested_by: "operator-clipboard-replay".to_string(),
        })
        .await
        .expect_err("mismatched clipboard replay must be rejected");
    assert!(
        clipboard_mismatch.to_string().contains("idempotency_key"),
        "unexpected clipboard replay error: {clipboard_mismatch}"
    );
}

#[tokio::test]
async fn clipboard_replay_survives_media_dedupe_with_distinct_artifact_refs() {
    let (store, _harness) = connected_store().await;
    let first_artifact =
        atelier_surreal_support::write_native_media_artifact(b"mt-025 clipboard dedupe bytes");
    let second_artifact = atelier_surreal_support::write_native_media_artifact_from_stored_payload(
        &first_artifact.stored_payload,
    );

    store
        .import_clipboard_image(&ClipboardImageImportRequest {
            idempotency_key: format!("clipboard-import-dedupe-source-{}", Uuid::new_v4()),
            mime: "image/png".to_string(),
            content_hash: first_artifact.content_hash.clone(),
            byte_len: first_artifact.byte_len,
            artifact_ref: first_artifact.artifact_ref,
            source_application: Some("system-clipboard".to_string()),
            requested_by: "operator-clipboard-dedupe".to_string(),
        })
        .await
        .expect("first import creates the canonical media asset");

    let request = ClipboardImageImportRequest {
        idempotency_key: format!("clipboard-import-dedupe-replay-{}", Uuid::new_v4()),
        mime: "image/png".to_string(),
        content_hash: second_artifact.content_hash.clone(),
        byte_len: second_artifact.byte_len,
        artifact_ref: second_artifact.artifact_ref.clone(),
        source_application: Some("system-clipboard-alt".to_string()),
        requested_by: "operator-clipboard-dedupe".to_string(),
    };
    let record = store
        .import_clipboard_image(&request)
        .await
        .expect("deduped clipboard import records the operator-provided artifact ref");
    assert_eq!(
        record.artifact_ref.as_deref(),
        Some(request.artifact_ref.as_str())
    );
    assert_eq!(record.source_provenance, "clipboard:system-clipboard-alt");

    let replay = store
        .import_clipboard_image(&request)
        .await
        .expect("exact replay must not be rejected because media asset was deduped");
    assert_eq!(replay.import_id, record.import_id);
    assert_eq!(replay.artifact_ref, record.artifact_ref);
}

#[tokio::test]
async fn clipboard_import_recovers_after_post_materialization_event_failure() {
    let (failing_store, failing_harness) =
        connected_store_with_recorder(Arc::new(FailAfterFlightRecorder::new(1))).await;
    let artifact = atelier_surreal_support::write_native_media_artifact(
        b"mt-025 clipboard post materialization failure",
    );
    let request = ClipboardImageImportRequest {
        idempotency_key: format!("clipboard-import-post-media-fail-{}", Uuid::new_v4()),
        mime: "image/png".to_string(),
        content_hash: artifact.content_hash.clone(),
        byte_len: artifact.byte_len,
        artifact_ref: artifact.artifact_ref.clone(),
        source_application: Some("system-clipboard".to_string()),
        requested_by: "operator-clipboard-failure".to_string(),
    };
    let result = failing_store.import_clipboard_image(&request).await;
    assert!(
        result.is_err(),
        "clipboard import returns an error when the image-import event mirror fails"
    );
    assert_eq!(
        image_import_request_count_by_key(&failing_harness, &request.idempotency_key).await,
        0,
        "failed image-import event write must roll back the clipboard import row"
    );
    assert_eq!(
        media_asset_count_for_hash(&failing_store, &artifact.content_hash).await,
        1,
        "media materialization is a recoverable committed prerequisite"
    );

    let good_store = AtelierStore::with_event_ledger(
        failing_harness.storage.clone(),
        failing_harness.database.clone(),
    );
    let recovered = good_store
        .import_clipboard_image(&request)
        .await
        .expect("retry recovers by binding the already materialized media asset");
    assert_eq!(
        recovered.artifact_ref.as_deref(),
        Some(request.artifact_ref.as_str())
    );
    assert_eq!(
        good_store
            .count_events_for_aggregate(
                event_family::IMAGE_IMPORT_RECORDED,
                "atelier_image_import_request",
                &recovered.import_id.to_string(),
            )
            .await
            .expect("count recovered image import event"),
        1
    );
}

#[tokio::test]
async fn url_image_import_rolls_back_row_when_eventledger_write_fails() {
    let (store, harness) = connected_store_with_failing_recorder().await;
    let idempotency_key = format!("url-import-event-fails-{}", Uuid::new_v4());

    let result = store
        .record_url_image_import(&UrlImageImportRequest {
            idempotency_key: idempotency_key.clone(),
            source_url: format!("https://example.com/event-fail/{}.png", Uuid::new_v4()),
            expected_mime: Some("image/png".to_string()),
            source_label: Some("event failure".to_string()),
            capability_profile_id: "MediaDownloader".to_string(),
            capability_grant_ref: media_downloader_grant("MediaDownloader"),
            requested_by: "operator-url".to_string(),
        })
        .await;
    assert!(
        result.is_err(),
        "URL import returns an error when EventLedger evidence fails"
    );
    assert_eq!(
        image_import_request_count_by_key(&harness, &idempotency_key).await,
        0,
        "failed EventLedger write must roll back the URL import row"
    );
}

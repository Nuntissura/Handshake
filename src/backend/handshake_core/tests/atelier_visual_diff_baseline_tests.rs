//! WP-KERNEL-005 MT-155 (Visual Diff Baseline Contract) live PostgreSQL
//! round-trip proof.
//!
//! Integration validation v2 failed MT-155 because baselines and the diff
//! request schema only existed as embedded fields of an in-memory loop
//! projection. This file proves the durable contract: screenshot baselines
//! persist in `kernel_visual_diff_baseline` (migration 0124, applied by the
//! kernel migration runner), standalone diff requests persist threshold +
//! metadata in `kernel_visual_diff_request` with the baseline-or-previous
//! reference contract, both RE-READ from PostgreSQL through the real
//! `AtelierStore`, and every record emits its `kernel.visual_diff.*`
//! EventLedger family. Baseline hashes are computed from real payload bytes,
//! not hardcoded literals.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use chrono::Utc;
use handshake_core::atelier::AtelierStore;
use handshake_core::kernel::visual_debugging_loop::{
    VisualComparisonMode, VisualDebuggingThresholdConfigV1,
};
use handshake_core::kernel::visual_diff_baseline::{
    kernel_visual_diff_event_family, NewVisualDiffBaseline, NewVisualDiffRequest,
    VisualDiffReference, KERNEL_VISUAL_DIFF_BASELINE_SCHEMA, KERNEL_VISUAL_DIFF_REQUEST_SCHEMA,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

async fn connected_store_with_ledger(url: &str) -> (AtelierStore, Arc<dyn Database>) {
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
    let store = AtelierStore::with_event_ledger(pool, database.clone());
    store.ensure_schema().await.expect("ensure atelier schema");
    (store, database)
}

fn sha256_token(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

fn unique_surface() -> String {
    format!("dcc.session_panel.{}", Uuid::new_v4())
}

fn new_baseline(surface_id: &str, payload: &[u8], label: &str) -> NewVisualDiffBaseline {
    NewVisualDiffBaseline {
        surface_id: surface_id.to_string(),
        baseline_ref: format!("artifact://baselines/{surface_id}/{label}.png"),
        content_sha256: sha256_token(payload),
        captured_by: "mt155-proof".to_string(),
        captured_at_utc: Utc::now(),
    }
}

fn threshold_config() -> VisualDebuggingThresholdConfigV1 {
    VisualDebuggingThresholdConfigV1 {
        threshold_config_ref: "packet://WP-KERNEL-005/visual-thresholds".to_string(),
        max_pixel_diff_basis_points: 250,
        max_layout_shift_basis_points: 100,
        structural_mismatch_limit: 2,
    }
}

#[tokio::test]
async fn mt155_visual_diff_baseline_persists_re_reads_and_tracks_latest() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt155_visual_diff_baseline_persists_re_reads_and_tracks_latest: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let surface_id = unique_surface();
    // Real payload bytes drive the persisted content hash.
    let first_payload: Vec<u8> = (0..512u32).map(|i| (i % 251) as u8).collect();

    let first = store
        .record_visual_diff_baseline(&new_baseline(&surface_id, &first_payload, "v1"))
        .await
        .expect("record first baseline");
    assert_eq!(first.surface_id, surface_id);
    assert_eq!(first.content_sha256, sha256_token(&first_payload));

    // RE-READ from PostgreSQL by id: full fidelity.
    let reloaded = store
        .get_visual_diff_baseline(first.baseline_id)
        .await
        .expect("reload baseline")
        .expect("baseline exists");
    assert_eq!(reloaded, first);

    // A newer capture for the same surface becomes the latest baseline (the
    // "baseline" side of the baseline-or-previous contract).
    let mut second_payload = first_payload.clone();
    second_payload[10] ^= 0xFF;
    let second = store
        .record_visual_diff_baseline(&new_baseline(&surface_id, &second_payload, "v2"))
        .await
        .expect("record second baseline");
    let latest = store
        .latest_visual_diff_baseline_for_surface(&surface_id)
        .await
        .expect("latest baseline query")
        .expect("latest baseline exists");
    assert_eq!(latest.baseline_id, second.baseline_id, "newest capture wins");
    assert_ne!(
        latest.content_sha256, first.content_sha256,
        "distinct payload bytes must persist distinct hashes"
    );

    // EventLedger proof on the baseline aggregate.
    let events = database
        .list_kernel_events_for_aggregate(
            "kernel_visual_diff_baseline",
            &first.baseline_id.to_string(),
        )
        .await
        .expect("list baseline ledger events");
    assert!(
        events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == json!(kernel_visual_diff_event_family::BASELINE_RECORDED)
                && event.payload["atelier_payload"]["schema"]
                    == json!(KERNEL_VISUAL_DIFF_BASELINE_SCHEMA)
                && event.payload["atelier_payload"]["surface_id"] == json!(surface_id)
                && event.payload["atelier_payload"]["content_sha256"]
                    == json!(sha256_token(&first_payload))
        }),
        "recording a baseline must emit canonical EventLedger evidence"
    );
}

#[tokio::test]
async fn mt155_visual_diff_request_persists_threshold_and_metadata_for_both_reference_kinds() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt155_visual_diff_request_persists_threshold_and_metadata_for_both_reference_kinds: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let surface_id = unique_surface();
    let baseline_payload: Vec<u8> = (0..256u32).map(|i| (i % 97) as u8).collect();
    let baseline = store
        .record_visual_diff_baseline(&new_baseline(&surface_id, &baseline_payload, "v1"))
        .await
        .expect("record baseline");

    // Baseline-bound request with threshold + metadata.
    let metadata = json!({
        "trigger": "post_commit",
        "commit_ref": "git://commit/abc123",
        "wp_id": "WP-KERNEL-005",
    });
    let request = store
        .record_visual_diff_request(&NewVisualDiffRequest {
            surface_id: surface_id.clone(),
            reference: VisualDiffReference::Baseline {
                baseline_id: baseline.baseline_id,
            },
            candidate_screenshot_ref: format!(
                "artifact://screenshots/{surface_id}/candidate-1.png"
            ),
            comparison_mode: VisualComparisonMode::PixelDiff,
            threshold_config: threshold_config(),
            metadata: metadata.clone(),
            requested_by: "mt155-proof".to_string(),
        })
        .await
        .expect("record baseline-bound diff request");

    // RE-READ from PostgreSQL: threshold config and metadata round-trip.
    let reloaded = store
        .get_visual_diff_request(request.request_id)
        .await
        .expect("reload diff request")
        .expect("diff request exists");
    assert_eq!(reloaded, request);
    assert_eq!(reloaded.threshold_config, threshold_config());
    assert_eq!(reloaded.metadata, metadata);
    assert_eq!(
        reloaded.reference,
        VisualDiffReference::Baseline {
            baseline_id: baseline.baseline_id
        }
    );

    // Previous-screenshot request (no registered baseline) — the "previous"
    // side of the baseline-or-previous contract.
    let previous = store
        .record_visual_diff_request(&NewVisualDiffRequest {
            surface_id: surface_id.clone(),
            reference: VisualDiffReference::PreviousScreenshot {
                previous_screenshot_ref: format!(
                    "artifact://screenshots/{surface_id}/previous-1.png"
                ),
            },
            candidate_screenshot_ref: format!(
                "artifact://screenshots/{surface_id}/candidate-2.png"
            ),
            comparison_mode: VisualComparisonMode::StructuralDom,
            threshold_config: threshold_config(),
            metadata: json!({ "trigger": "post_action" }),
            requested_by: "mt155-proof".to_string(),
        })
        .await
        .expect("record previous-screenshot diff request");
    let reloaded_previous = store
        .get_visual_diff_request(previous.request_id)
        .await
        .expect("reload previous-screenshot request")
        .expect("previous-screenshot request exists");
    assert_eq!(
        reloaded_previous.reference,
        VisualDiffReference::PreviousScreenshot {
            previous_screenshot_ref: format!(
                "artifact://screenshots/{surface_id}/previous-1.png"
            ),
        }
    );
    assert_eq!(
        reloaded_previous.comparison_mode,
        VisualComparisonMode::StructuralDom
    );

    // EventLedger proof on the request aggregate.
    let events = database
        .list_kernel_events_for_aggregate(
            "kernel_visual_diff_request",
            &request.request_id.to_string(),
        )
        .await
        .expect("list diff request ledger events");
    assert!(
        events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == json!(kernel_visual_diff_event_family::REQUEST_RECORDED)
                && event.payload["atelier_payload"]["schema"]
                    == json!(KERNEL_VISUAL_DIFF_REQUEST_SCHEMA)
                && event.payload["atelier_payload"]["comparison_mode"] == json!("pixel_diff")
                && event.payload["atelier_payload"]["max_pixel_diff_basis_points"] == json!(250)
        }),
        "recording a diff request must emit canonical EventLedger evidence"
    );
}

#[tokio::test]
async fn mt155_visual_diff_request_rejects_unknown_baseline_and_nonportable_refs() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt155_visual_diff_request_rejects_unknown_baseline_and_nonportable_refs: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, _database) = connected_store_with_ledger(&url).await;

    let surface_id = unique_surface();

    // A request bound to a baseline the store has never registered fails
    // typed and persists nothing.
    let err = store
        .record_visual_diff_request(&NewVisualDiffRequest {
            surface_id: surface_id.clone(),
            reference: VisualDiffReference::Baseline {
                baseline_id: Uuid::now_v7(),
            },
            candidate_screenshot_ref: format!(
                "artifact://screenshots/{surface_id}/candidate.png"
            ),
            comparison_mode: VisualComparisonMode::PixelDiff,
            threshold_config: threshold_config(),
            metadata: json!({}),
            requested_by: "mt155-proof".to_string(),
        })
        .await
        .expect_err("unknown baseline must be rejected");
    assert!(
        err.to_string().contains("unknown baseline"),
        "rejection must name the missing baseline: {err}"
    );

    // Zero pixel threshold is rejected before anything persists.
    let mut zero_threshold = threshold_config();
    zero_threshold.max_pixel_diff_basis_points = 0;
    let err = store
        .record_visual_diff_request(&NewVisualDiffRequest {
            surface_id: surface_id.clone(),
            reference: VisualDiffReference::PreviousScreenshot {
                previous_screenshot_ref: format!(
                    "artifact://screenshots/{surface_id}/previous.png"
                ),
            },
            candidate_screenshot_ref: format!(
                "artifact://screenshots/{surface_id}/candidate.png"
            ),
            comparison_mode: VisualComparisonMode::PixelDiff,
            threshold_config: zero_threshold,
            metadata: json!({}),
            requested_by: "mt155-proof".to_string(),
        })
        .await
        .expect_err("zero pixel threshold must be rejected");
    assert!(err.to_string().contains("max_pixel_diff_basis_points"));

    // Non-portable baseline refs never persist.
    let err = store
        .record_visual_diff_baseline(&NewVisualDiffBaseline {
            surface_id: surface_id.clone(),
            baseline_ref: "artifact://screenshots/not-a-baseline.png".to_string(),
            content_sha256: sha256_token(b"payload"),
            captured_by: "mt155-proof".to_string(),
            captured_at_utc: Utc::now(),
        })
        .await
        .expect_err("baseline_ref outside artifact://baselines/ must be rejected");
    assert!(err.to_string().contains("baseline_ref"));

    // The failed writes left the surface without any baseline.
    let latest = store
        .latest_visual_diff_baseline_for_surface(&surface_id)
        .await
        .expect("latest baseline query");
    assert!(
        latest.is_none(),
        "rejected writes must not leave partial baseline rows"
    );
}

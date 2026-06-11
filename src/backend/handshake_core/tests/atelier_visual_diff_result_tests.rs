//! WP-KERNEL-005 MT-157 (Pixel Versus Structural Comparison) live PostgreSQL
//! proof.
//!
//! Integration validation v2 failed MT-157 because the comparison strategy
//! enum was missing the required `Manual` variant, there were no diff RESULT
//! fields, and the proof never touched a managed resource. This file proves
//! the completed contract end-to-end: real comparisons are COMPUTED by
//! [`compute_visual_comparison`] from actual payload bytes (pixel) and real
//! JSON DOM snapshots (structural) — never hardcoded literals — for all
//! three strategy variants (`PixelDiff` / `StructuralDom` / `Manual`), the
//! computed result fields persist in `kernel_visual_diff_result` (migration
//! 0124) against their request through the real `AtelierStore`, are RE-READ
//! from PostgreSQL, and the `kernel.visual_diff.result_recorded` EventLedger
//! family is asserted on the request aggregate.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use chrono::Utc;
use handshake_core::atelier::AtelierStore;
use handshake_core::kernel::visual_debugging_loop::{
    compute_visual_comparison, VisualComparisonMode, VisualDebuggingThresholdConfigV1,
    VisualDiffOutcome,
};
use handshake_core::kernel::visual_diff_baseline::{
    kernel_visual_diff_event_family, NewVisualDiffBaseline, NewVisualDiffRequest,
    VisualDiffReference, KERNEL_VISUAL_DIFF_RESULT_SCHEMA,
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
    format!("dcc.inspector_panel.{}", Uuid::new_v4())
}

fn threshold_config() -> VisualDebuggingThresholdConfigV1 {
    VisualDebuggingThresholdConfigV1 {
        threshold_config_ref: "packet://WP-KERNEL-005/visual-thresholds".to_string(),
        max_pixel_diff_basis_points: 250,
        max_layout_shift_basis_points: 100,
        structural_mismatch_limit: 2,
    }
}

/// Register a baseline from real payload bytes and a diff request in the
/// given comparison mode bound to that baseline.
async fn baseline_bound_request(
    store: &AtelierStore,
    surface_id: &str,
    baseline_payload: &[u8],
    mode: VisualComparisonMode,
) -> Uuid {
    let baseline = store
        .record_visual_diff_baseline(&NewVisualDiffBaseline {
            surface_id: surface_id.to_string(),
            baseline_ref: format!("artifact://baselines/{surface_id}/{}.png", Uuid::new_v4()),
            content_sha256: sha256_token(baseline_payload),
            captured_by: "mt157-proof".to_string(),
            captured_at_utc: Utc::now(),
        })
        .await
        .expect("record baseline");
    let request = store
        .record_visual_diff_request(&NewVisualDiffRequest {
            surface_id: surface_id.to_string(),
            reference: VisualDiffReference::Baseline {
                baseline_id: baseline.baseline_id,
            },
            candidate_screenshot_ref: format!(
                "artifact://screenshots/{surface_id}/{}.png",
                Uuid::new_v4()
            ),
            comparison_mode: mode,
            threshold_config: threshold_config(),
            metadata: json!({ "trigger": "post_commit" }),
            requested_by: "mt157-proof".to_string(),
        })
        .await
        .expect("record diff request");
    request.request_id
}

#[tokio::test]
async fn mt157_pixel_diff_computation_persists_result_fields_and_ledger_event() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt157_pixel_diff_computation_persists_result_fields_and_ledger_event: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let surface_id = unique_surface();
    // Real pixel payloads: 1000 bytes, 30 of them flipped in the candidate.
    // 30/1000 differing = 300 basis points > 250 threshold => Fail.
    let reference: Vec<u8> = vec![0xAA; 1000];
    let mut candidate = reference.clone();
    for byte in candidate.iter_mut().take(30) {
        *byte = 0x55;
    }

    let request_id = baseline_bound_request(
        &store,
        &surface_id,
        &reference,
        VisualComparisonMode::PixelDiff,
    )
    .await;

    let computation = compute_visual_comparison(
        VisualComparisonMode::PixelDiff,
        &reference,
        &candidate,
        &threshold_config(),
    )
    .expect("pixel comparison computes");
    assert_eq!(computation.units_compared, 1000);
    assert_eq!(computation.units_differing, 30);
    assert_eq!(computation.mismatch_basis_points, 300);
    assert!(computation.threshold_exceeded);
    assert_eq!(computation.outcome, VisualDiffOutcome::Fail);

    let recorded = store
        .record_visual_diff_result(request_id, &computation, Utc::now())
        .await
        .expect("persist pixel comparison result");
    assert_eq!(recorded.computation, computation);

    // RE-READ from PostgreSQL: the result fields are durable evidence.
    let results = store
        .list_visual_diff_results_for_request(request_id)
        .await
        .expect("re-read results for request");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], recorded);
    assert_eq!(results[0].computation.mismatch_basis_points, 300);
    assert_eq!(results[0].computation.outcome, VisualDiffOutcome::Fail);

    // EventLedger proof on the request aggregate.
    let events = database
        .list_kernel_events_for_aggregate("kernel_visual_diff_request", &request_id.to_string())
        .await
        .expect("list result ledger events");
    assert!(
        events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == json!(kernel_visual_diff_event_family::RESULT_RECORDED)
                && event.payload["atelier_payload"]["schema"]
                    == json!(KERNEL_VISUAL_DIFF_RESULT_SCHEMA)
                && event.payload["atelier_payload"]["comparison_mode"] == json!("pixel_diff")
                && event.payload["atelier_payload"]["mismatch_basis_points"] == json!(300)
                && event.payload["atelier_payload"]["threshold_exceeded"] == json!(true)
                && event.payload["atelier_payload"]["outcome"] == json!("fail")
        }),
        "recording a comparison result must emit canonical EventLedger evidence"
    );
}

#[tokio::test]
async fn mt157_structural_dom_computation_persists_within_limit_pass() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt157_structural_dom_computation_persists_within_limit_pass: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, _database) = connected_store_with_ledger(&url).await;

    let surface_id = unique_surface();
    // Real DOM snapshots: one text node differs; the structural limit is 2,
    // so a single mismatched node passes without exceeding the threshold.
    let reference_dom = json!({
        "tag": "main",
        "children": [
            { "tag": "header", "text": "Sessions" },
            { "tag": "button", "text": "Refresh" },
        ],
    });
    let mut candidate_dom = reference_dom.clone();
    candidate_dom["children"][1]["text"] = json!("Reload");
    let reference_bytes = serde_json::to_vec(&reference_dom).expect("serialize reference DOM");
    let candidate_bytes = serde_json::to_vec(&candidate_dom).expect("serialize candidate DOM");

    let request_id = baseline_bound_request(
        &store,
        &surface_id,
        &reference_bytes,
        VisualComparisonMode::StructuralDom,
    )
    .await;

    let computation = compute_visual_comparison(
        VisualComparisonMode::StructuralDom,
        &reference_bytes,
        &candidate_bytes,
        &threshold_config(),
    )
    .expect("structural comparison computes");
    assert_eq!(computation.units_differing, 1, "exactly one node differs");
    assert!(computation.units_compared > 1, "whole DOM tree is compared");
    assert!(!computation.threshold_exceeded, "1 mismatch <= limit of 2");
    assert_eq!(computation.outcome, VisualDiffOutcome::Pass);

    let recorded = store
        .record_visual_diff_result(request_id, &computation, Utc::now())
        .await
        .expect("persist structural comparison result");

    let results = store
        .list_visual_diff_results_for_request(request_id)
        .await
        .expect("re-read structural results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], recorded);
    assert_eq!(
        results[0].computation.comparison_mode,
        VisualComparisonMode::StructuralDom
    );
    assert_eq!(results[0].computation.outcome, VisualDiffOutcome::Pass);
}

#[tokio::test]
async fn mt157_manual_mode_persists_manual_review_required_and_rejects_mode_mismatch() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt157_manual_mode_persists_manual_review_required_and_rejects_mode_mismatch: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let surface_id = unique_surface();
    let payload: Vec<u8> = (0..128u32).map(|i| (i % 113) as u8).collect();
    let request_id =
        baseline_bound_request(&store, &surface_id, &payload, VisualComparisonMode::Manual)
            .await;

    // The Manual strategy never auto-decides: the computed result parks in
    // ManualReviewRequired until an operator verdict is recorded.
    let manual = compute_visual_comparison(
        VisualComparisonMode::Manual,
        &payload,
        &payload,
        &threshold_config(),
    )
    .expect("manual comparison computes");
    assert_eq!(manual.outcome, VisualDiffOutcome::ManualReviewRequired);
    assert!(!manual.threshold_exceeded);

    let recorded = store
        .record_visual_diff_result(request_id, &manual, Utc::now())
        .await
        .expect("persist manual comparison result");
    let results = store
        .list_visual_diff_results_for_request(request_id)
        .await
        .expect("re-read manual results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], recorded);
    assert_eq!(
        results[0].computation.outcome,
        VisualDiffOutcome::ManualReviewRequired,
        "manual review state must be durable across re-reads"
    );

    let events = database
        .list_kernel_events_for_aggregate("kernel_visual_diff_request", &request_id.to_string())
        .await
        .expect("list manual result ledger events");
    assert!(
        events.iter().any(|event| {
            event.payload["event_family"]
                == json!(kernel_visual_diff_event_family::RESULT_RECORDED)
                && event.payload["atelier_payload"]["outcome"] == json!("manual_review_required")
        }),
        "manual result must emit EventLedger evidence with the parked outcome"
    );

    // Mode-mismatch defense: a pixel computation cannot be recorded against
    // the manual request, and the rejected write persists nothing new.
    let pixel = compute_visual_comparison(
        VisualComparisonMode::PixelDiff,
        &payload,
        &payload,
        &threshold_config(),
    )
    .expect("pixel comparison computes");
    let err = store
        .record_visual_diff_result(request_id, &pixel, Utc::now())
        .await
        .expect_err("mode mismatch must be rejected");
    assert!(
        err.to_string().contains("comparison mode mismatch"),
        "rejection must name the mismatch: {err}"
    );
    let results = store
        .list_visual_diff_results_for_request(request_id)
        .await
        .expect("re-read results after rejected write");
    assert_eq!(results.len(), 1, "rejected mode-mismatch write must not persist");

    // Unknown request defense.
    let err = store
        .record_visual_diff_result(Uuid::now_v7(), &manual, Utc::now())
        .await
        .expect_err("unknown request must be rejected");
    assert!(err.to_string().contains("unknown request"));
}

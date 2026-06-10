//! WP-KERNEL-005 MT-115 / MT-116 / MT-117: real PostgreSQL round-trip proofs
//! for the typed pose deferred-feature registry.
//!
//! These MTs are "record a deferred/blocked pose feature with a reason" as a
//! TYPED RUNTIME RECORD, not governance markdown. Each test connects the real
//! `AtelierStore` to a live Postgres, ensures the schema, records the specific
//! features each MT names, reloads them, and asserts they persist with a
//! non-empty reason and the correct status. A negative test proves a blank
//! `deferral_reason` is rejected, so a deferral can never be silent.
//!
//! Gated on `atelier_pg_support::database_url()`: when no PostgreSQL is
//! available the test prints SKIP and returns (never SQLite).

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::pose::{
    pose_deferred_feature_catalog, NewPoseDeferredFeature, PoseDeferredStatus,
};
use handshake_core::atelier::{AtelierError, AtelierStore};
use std::collections::HashMap;

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres. The deferred-feature table has no character FK, so no fixture
/// entity is needed.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// The deferred-feature table is keyed by a stable `feature_id` PK and persists
/// between runs. Record the catalog (idempotent upsert) and return a map of the
/// reloaded rows by `feature_id` so each MT can assert on its own ids.
async fn record_catalog_and_reload(
    store: &AtelierStore,
) -> HashMap<String, handshake_core::atelier::pose::PoseDeferredFeature> {
    for new in pose_deferred_feature_catalog() {
        store
            .record_pose_deferred_feature(&new)
            .await
            .expect("record pose deferred feature");
    }
    store
        .list_pose_deferred_features()
        .await
        .expect("list pose deferred features")
        .into_iter()
        .map(|f| (f.feature_id.clone(), f))
        .collect()
}

/// MT-115: the five non-calibration WP-0133 pose-workspace items persist as
/// BLOCKED with a non-empty reason.
#[tokio::test]
async fn mt115_pose_workspace_blocked_features_persist_with_reasons() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt115_pose_workspace_blocked_features_persist_with_reasons: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let by_id = record_catalog_and_reload(&store).await;

    let expected = [
        "mt-115.pose-workspace.draggable-overlay",
        "mt-115.pose-workspace.missing-marker-placement",
        "mt-115.pose-workspace.3d-live-split",
        "mt-115.pose-workspace.forked-history",
        "mt-115.pose-workspace.history-tab",
    ];
    for feature_id in expected {
        let feature = by_id
            .get(feature_id)
            .unwrap_or_else(|| panic!("MT-115 feature {feature_id} must be present"));
        assert_eq!(
            feature.status,
            PoseDeferredStatus::Blocked,
            "MT-115 feature {feature_id} must be BLOCKED"
        );
        assert!(
            !feature.deferral_reason.trim().is_empty(),
            "MT-115 feature {feature_id} must carry a non-empty reason"
        );
        assert!(
            feature.carry_forward,
            "MT-115 feature {feature_id} must be carried forward"
        );
    }

    // The MT-115 set is exactly the five non-calibration WP-0133 items.
    let kind_rows = store
        .list_pose_deferred_features_by_kind("MT-115.pose-workspace-blocked")
        .await
        .expect("list MT-115 features by kind");
    assert_eq!(
        kind_rows.len(),
        expected.len(),
        "MT-115 must record exactly five pose-workspace blocked items"
    );
}

/// MT-116: the RigData v2 multi-subject record persists as a carry-forward
/// DEFERRED record with a non-empty reason.
#[tokio::test]
async fn mt116_rigdata_v2_multi_subject_carry_forward_deferred() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt116_rigdata_v2_multi_subject_carry_forward_deferred: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let by_id = record_catalog_and_reload(&store).await;

    let feature = by_id
        .get("mt-116.rigdata-v2.multi-subject")
        .expect("MT-116 RigData v2 multi-subject record must be present");
    assert_eq!(
        feature.status,
        PoseDeferredStatus::Deferred,
        "MT-116 RigData v2 multi-subject must be DEFERRED (planned/deferred carry-forward)"
    );
    assert!(
        feature.carry_forward,
        "MT-116 RigData v2 multi-subject must be carried forward"
    );
    assert!(
        !feature.deferral_reason.trim().is_empty(),
        "MT-116 RigData v2 multi-subject must carry a non-empty deferral reason"
    );
    assert!(
        feature.feature_label.contains("RigData v2"),
        "MT-116 record must name RigData v2 multi-subject"
    );
}

/// MT-117: the named Pose tab polish features persist as PLANNED deferred.
#[tokio::test]
async fn mt117_pose_tab_polish_features_deferred() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt117_pose_tab_polish_features_deferred: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let by_id = record_catalog_and_reload(&store).await;

    let expected = [
        "mt-117.pose-tab-polish.multi-file-dnd",
        "mt-117.pose-tab-polish.multi-angle-export",
        "mt-117.pose-tab-polish.clear-workspace",
        "mt-117.pose-tab-polish.sync-zoom",
        "mt-117.pose-tab-polish.import-openpose-json",
        "mt-117.pose-tab-polish.shortcuts",
        "mt-117.pose-tab-polish.stylized-landmark-router",
    ];
    for feature_id in expected {
        let feature = by_id
            .get(feature_id)
            .unwrap_or_else(|| panic!("MT-117 feature {feature_id} must be present"));
        assert_eq!(
            feature.status,
            PoseDeferredStatus::Planned,
            "MT-117 feature {feature_id} must be PLANNED/RESEARCH deferred"
        );
        assert!(
            !feature.deferral_reason.trim().is_empty(),
            "MT-117 feature {feature_id} must carry a non-empty reason"
        );
    }

    let kind_rows = store
        .list_pose_deferred_features_by_kind("MT-117.pose-tab-polish-carry-forward")
        .await
        .expect("list MT-117 features by kind");
    assert_eq!(
        kind_rows.len(),
        expected.len(),
        "MT-117 must record exactly the seven named pose-tab polish features"
    );
}

/// Negative: an empty `deferral_reason` is rejected with a Validation error so a
/// blank deferral can never be persisted.
#[tokio::test]
async fn empty_deferral_reason_is_rejected() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP empty_deferral_reason_is_rejected: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    let blank = NewPoseDeferredFeature {
        feature_id: "mt-115.pose-workspace.blank-reason-probe".to_string(),
        feature_kind: "MT-115.pose-workspace-blocked".to_string(),
        status: PoseDeferredStatus::Blocked,
        feature_label: "Blank reason probe".to_string(),
        deferral_reason: "   ".to_string(),
        carry_forward: true,
        source_ref: None,
    };
    let err = store
        .record_pose_deferred_feature(&blank)
        .await
        .expect_err("empty deferral_reason must be rejected");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "empty deferral_reason must produce a Validation error, got {err:?}"
    );
}

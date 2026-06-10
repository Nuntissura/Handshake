//! WP-KERNEL-005 MT-147 / MT-148 / MT-153 / MT-167: real PostgreSQL round-trip
//! proofs for the typed Model-Workflow-Diagnostics projection surfaces.
//!
//! These MTs are TYPED RUNTIME surfaces (Postgres rows + EventLedger events),
//! never governance markdown:
//!   * MT-147 -- model work-state projection (active MT, owner, status, blocker,
//!     receipts, next action, evidence) into a Locus/MT diagnostics row.
//!   * MT-148 -- DCC session/lease/command-log/recovery panel projections, one
//!     typed row per panel kind.
//!   * MT-153 -- screenshot artifact storage: a stealth capture promoted to a
//!     governed, retained screenshot artifact with metadata + retention.
//!   * MT-167 -- stale README/spec drift detector: a typed drift finding recorded
//!     only when a doc-claimed surface differs from the code/spec surface.
//!
//! Gated on `atelier_pg_support::database_url()`: when no PostgreSQL is
//! available the test prints SKIP and returns (never SQLite).
//!
//! NOTE: migration 0115 is not yet wired into `ensure_schema` (the orchestrator
//! wires it after this MT lands). The shared preamble therefore applies the 0115
//! migration itself; `CREATE TABLE IF NOT EXISTS` makes this idempotent and safe
//! once the orchestrator has wired it in.
//!
//! Proof command (all four tests):
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_diagnostics_projections_tests -- --nocapture

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::state_probe::{
    DccPanelKind, NewDccPanelProjection, NewScreenshotArtifactStorage, NewWorkStateProjection,
};
use handshake_core::atelier::stealth_window::{NewStealthWindow, QuietFlags, VisibilityFlag};
use handshake_core::atelier::{AtelierError, AtelierStore};
use uuid::Uuid;

/// Connect, ensure the wired schema, then apply the (not-yet-wired) 0115
/// diagnostics-projection migration. Idempotent.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    sqlx::raw_sql(include_str!(
        "../migrations/0115_atelier_diagnostics_projections.sql"
    ))
    .execute(store.pool())
    .await
    .expect("apply 0115 diagnostics-projection migration");
    store
}

/// MT-147: a model work-state projection round-trips through Postgres with all
/// fields preserved.
#[tokio::test]
async fn mt147_work_state_projection_round_trips() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt147_work_state_projection_round_trips: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    let projection_id = format!("wsp-{}", Uuid::new_v4());
    let input = NewWorkStateProjection {
        projection_id: projection_id.clone(),
        active_mt: "MT-147".to_string(),
        owner: "KB-KERNEL-005-CLOSEOUT".to_string(),
        status: "READY_FOR_VALIDATION".to_string(),
        blocker: Some("waiting on MT-146 dependency".to_string()),
        receipts_ref: Some("ledger-event-receipt-abc123".to_string()),
        next_action: Some("validator round-trips the projection".to_string()),
        evidence_ref: Some("artifact-manifest-evidence-001".to_string()),
    };

    let recorded = store
        .record_work_state_projection(&input)
        .await
        .expect("record work-state projection");
    assert_eq!(recorded.projection_id, projection_id);
    assert_eq!(recorded.active_mt, "MT-147");
    assert_eq!(recorded.owner, "KB-KERNEL-005-CLOSEOUT");
    assert_eq!(recorded.status, "READY_FOR_VALIDATION");
    assert_eq!(
        recorded.blocker.as_deref(),
        Some("waiting on MT-146 dependency")
    );
    assert_eq!(
        recorded.evidence_ref.as_deref(),
        Some("artifact-manifest-evidence-001")
    );

    // Reload via the list projection and find our row.
    let reloaded = store
        .list_work_state_projections()
        .await
        .expect("list work-state projections");
    let found = reloaded
        .into_iter()
        .find(|p| p.projection_id == projection_id)
        .expect("recorded work-state projection must be listed");
    assert_eq!(found, recorded, "round-trip must preserve every field");

    // Idempotent re-record with an updated status keeps the same PK.
    let updated = store
        .record_work_state_projection(&NewWorkStateProjection {
            status: "VALIDATED".to_string(),
            ..input.clone()
        })
        .await
        .expect("re-record work-state projection");
    assert_eq!(updated.projection_id, projection_id);
    assert_eq!(updated.status, "VALIDATED");

    // A .GOV / machine-local evidence ref is rejected.
    let bad = NewWorkStateProjection {
        projection_id: format!("wsp-bad-{}", Uuid::new_v4()),
        evidence_ref: Some("C:/Users/op/evidence.json".to_string()),
        ..input.clone()
    };
    let err = store
        .record_work_state_projection(&bad)
        .await
        .expect_err("machine-local evidence ref must be rejected");
    assert!(
        matches!(
            err,
            AtelierError::Validation(_) | AtelierError::ForbiddenStorage(_)
        ),
        "machine-local evidence ref must be rejected, got {err:?}"
    );
}

/// MT-148: a DCC panel projection round-trips for each of the four panel kinds.
#[tokio::test]
async fn mt148_dcc_panel_projection_round_trips_each_kind() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt148_dcc_panel_projection_round_trips_each_kind: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    for kind in DccPanelKind::ALL.iter().copied() {
        let panel_id = format!("dcc-{}-{}", kind.as_token(), Uuid::new_v4());
        let state = serde_json::json!({
            "panel_kind": kind.as_token(),
            "rows": [{ "id": "row-1", "value": 42 }],
        });
        let recorded = store
            .record_dcc_panel_projection(&NewDccPanelProjection {
                panel_id: panel_id.clone(),
                panel_kind: kind,
                state_json: state.clone(),
            })
            .await
            .unwrap_or_else(|err| panic!("record DCC panel projection for {kind:?}: {err:?}"));
        assert_eq!(recorded.panel_id, panel_id);
        assert_eq!(recorded.panel_kind, kind);
        assert_eq!(recorded.state_json, state);

        // list_by_kind returns our row and never a row of a different kind.
        let listed = store
            .list_dcc_panel_projections_by_kind(kind)
            .await
            .unwrap_or_else(|err| panic!("list DCC panel projections for {kind:?}: {err:?}"));
        assert!(
            listed.iter().any(|p| p.panel_id == panel_id),
            "recorded {kind:?} panel must be listed by its kind"
        );
        assert!(
            listed.iter().all(|p| p.panel_kind == kind),
            "list_by_kind({kind:?}) must only return rows of that kind"
        );
    }
}

/// MT-153: a stealth screenshot capture is promoted to a governed, retained
/// screenshot artifact with metadata, and round-trips through Postgres.
///
/// This proves the MT-153 EXTENSION of the existing stealth capture receipt:
/// the base `record_stealth_capture` only proves a capture happened, with no
/// metadata or retention; this storage row adds mime/dimensions/byte-len/label
/// and a retention policy (ttl + pinned) keyed to the capture.
#[tokio::test]
async fn mt153_stealth_capture_extension() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt153_stealth_capture_extension: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    // Base: create a real stealth window + capture receipt (existing surface).
    let window = store
        .create_stealth_window(&NewStealthWindow {
            owner_actor: format!("operator-{}", Uuid::new_v4()),
            title: format!("stealth-window-{}", Uuid::new_v4()),
            visibility: VisibilityFlag::OffScreenOnly,
            quiet: QuietFlags::default(),
            layout: None,
        })
        .await
        .expect("create stealth window");
    let manifest_id = format!("artifact-manifest-{}", Uuid::new_v4());
    let sha = format!("sha256-{}", Uuid::new_v4());
    let capture = store
        .record_stealth_capture(window.window_ref_id, &manifest_id, &sha)
        .await
        .expect("record stealth capture receipt");

    // Extension: store the capture as a governed, retained screenshot artifact.
    let storage_id = format!("sas-{}", Uuid::new_v4());
    let recorded = store
        .record_screenshot_artifact_storage(&NewScreenshotArtifactStorage {
            storage_id: storage_id.clone(),
            capture_id: capture.capture_id,
            artifact_manifest_id: manifest_id.clone(),
            content_sha256: sha.clone(),
            mime: "image/png".to_string(),
            width_px: Some(1920),
            height_px: Some(1080),
            byte_len: Some(204_800),
            label: Some("dcc-diagnostics-capture".to_string()),
            retention_ttl_days: Some(30),
            pinned: false,
        })
        .await
        .expect("store screenshot artifact");
    assert_eq!(recorded.storage_id, storage_id);
    assert_eq!(recorded.capture_id, capture.capture_id);
    assert_eq!(recorded.artifact_manifest_id, manifest_id);
    assert_eq!(recorded.mime, "image/png");
    assert_eq!(recorded.width_px, Some(1920));
    assert_eq!(recorded.height_px, Some(1080));
    assert_eq!(recorded.byte_len, Some(204_800));
    assert_eq!(recorded.retention_ttl_days, Some(30));
    assert!(!recorded.pinned);

    // Round-trip via the list read path.
    let reloaded = store
        .list_screenshot_artifact_storage()
        .await
        .expect("list screenshot artifact storage");
    let found = reloaded
        .into_iter()
        .find(|s| s.storage_id == storage_id)
        .expect("stored screenshot artifact must be listed");
    assert_eq!(found, recorded, "round-trip must preserve every field");

    // Idempotent on capture_id: re-storing the same capture updates in place
    // (e.g. pinning it) without creating a second row.
    let pinned = store
        .record_screenshot_artifact_storage(&NewScreenshotArtifactStorage {
            storage_id: storage_id.clone(),
            capture_id: capture.capture_id,
            artifact_manifest_id: manifest_id.clone(),
            content_sha256: sha.clone(),
            mime: "image/png".to_string(),
            width_px: Some(1920),
            height_px: Some(1080),
            byte_len: Some(204_800),
            label: Some("dcc-diagnostics-capture".to_string()),
            retention_ttl_days: None,
            pinned: true,
        })
        .await
        .expect("re-store screenshot artifact (pin)");
    assert!(pinned.pinned, "re-store must update the retention policy");
    assert_eq!(pinned.retention_ttl_days, None);

    // A machine-local artifact manifest id is rejected.
    let err = store
        .record_screenshot_artifact_storage(&NewScreenshotArtifactStorage {
            storage_id: format!("sas-bad-{}", Uuid::new_v4()),
            capture_id: Uuid::now_v7(),
            artifact_manifest_id: "C:/Users/op/shot.png".to_string(),
            content_sha256: sha.clone(),
            mime: "image/png".to_string(),
            width_px: None,
            height_px: None,
            byte_len: None,
            label: None,
            retention_ttl_days: None,
            pinned: false,
        })
        .await
        .expect_err("machine-local manifest id must be rejected");
    assert!(
        matches!(
            err,
            AtelierError::Validation(_) | AtelierError::ForbiddenStorage(_)
        ),
        "machine-local manifest id must be rejected, got {err:?}"
    );
}

/// MT-167: the drift detector records a finding for a mismatch and records
/// nothing for a match.
#[tokio::test]
async fn mt167_spec_drift_detector_records_mismatch_only() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt167_spec_drift_detector_records_mismatch_only: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    // MATCH: doc surface equals code surface -> no finding recorded.
    let match_finding_id = format!("drift-match-{}", Uuid::new_v4());
    let none = store
        .detect_and_record_spec_drift(
            &match_finding_id,
            "README.md#stealth-window",
            "spec:10.18.2",
            "StealthReferenceWindow",
            "StealthReferenceWindow",
            "surface_mismatch",
        )
        .await
        .expect("run drift detector on a match");
    assert!(none.is_none(), "a matching doc/code surface records no finding");

    // MISMATCH: doc claims a stale surface -> a finding is recorded.
    let drift_finding_id = format!("drift-mismatch-{}", Uuid::new_v4());
    let some = store
        .detect_and_record_spec_drift(
            &drift_finding_id,
            "README.md#ckc-window",
            "spec:10.18.2",
            "CkcReferenceWindow",
            "StealthReferenceWindow",
            "surface_mismatch",
        )
        .await
        .expect("run drift detector on a mismatch")
        .expect("a mismatch must record a drift finding");
    assert_eq!(some.finding_id, drift_finding_id);
    assert_eq!(some.doc_ref, "README.md#ckc-window");
    assert_eq!(some.spec_ref, "spec:10.18.2");
    assert_eq!(some.drift_kind, "surface_mismatch");
    assert!(
        some.detail.contains("CkcReferenceWindow")
            && some.detail.contains("StealthReferenceWindow"),
        "drift detail must name both the doc-claimed and code surfaces"
    );

    // The recorded finding is listed; the match id is never listed.
    let findings = store
        .list_spec_drift_findings()
        .await
        .expect("list spec drift findings");
    assert!(
        findings.iter().any(|f| f.finding_id == drift_finding_id),
        "the mismatch finding must be listed"
    );
    assert!(
        findings.iter().all(|f| f.finding_id != match_finding_id),
        "the matching surface must never produce a listed finding"
    );
}

//! WP-KERNEL-005 MT-158: visual evidence retention and redaction proofs.
//!
//! Bound visual artifacts and protect sensitive captures (retention class,
//! exportable, redaction, cleanup):
//!   * capture layer (`record_native_product_screenshot` +
//!     `VisualEvidenceProtectionV1`): declared sensitive regions are blacked
//!     out BEFORE the PNG is hashed/stored, redacted captures classify `High`,
//!     the retention class/TTL/exportable policy lands on the ArtifactStore
//!     manifest, and an unredacted capture must never be exportable;
//!   * storage layer (`atelier_screenshot_artifact_storage`, migration 0129):
//!     retention_class / exportable / redaction_applied round-trip through
//!     PostgreSQL, the exportable-without-redaction invariant is enforced on
//!     the row, and `cleanup_expired_screenshot_artifacts` prunes expired,
//!     unpinned rows while emitting `SCREENSHOT_ARTIFACT_RETENTION_CLEANED`
//!     EventLedger events (pinned / no-TTL rows survive).
//!
//! PG tests are gated on `atelier_pg_support::database_url()` (Handshake-
//! managed PostgreSQL; never SQLite).

mod atelier_pg_support;

use std::io::Cursor;

use atelier_pg_support::database_url;
use handshake_core::atelier::state_probe::{
    diagnostics_projection_event_family, NewScreenshotArtifactStorage,
};
use handshake_core::atelier::stealth_window::{NewStealthWindow, QuietFlags, VisibilityFlag};
use handshake_core::atelier::{AtelierError, AtelierStore};
use handshake_core::kernel::product_screenshot_capture::{
    record_native_product_screenshot, NativeScreenshotEvidence, ProductScreenshotExecutionError,
    ProductScreenshotRequestV1, ScreenshotCaptureExecutionSurface, ScreenshotCaptureScope,
    ScreenshotCaptureTriggerKind, VisualEvidenceProtectionV1, VisualRedactionRegionV1,
};
use handshake_core::storage::artifacts::{
    read_artifact_manifest, ArtifactClassification, ArtifactLayer,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// MT-158 (redaction): declared sensitive regions are blacked out before the
/// PNG is hashed and stored; the stored artifact carries the protection policy.
#[test]
fn mt158_redaction_blacks_out_sensitive_regions_before_storage() {
    let workspace_root = tempfile::tempdir().expect("temp workspace root");
    let request = request("request.native.redacted", "module://operator/secrets-panel");
    let png_bytes = white_png_bytes(4, 4);

    let result = record_native_product_screenshot(
        &request,
        evidence(png_bytes.clone()),
        None,
        &VisualEvidenceProtectionV1 {
            retention_class: "visual-sensitive".to_string(),
            retention_ttl_days: Some(7),
            exportable: true,
            redact_regions: vec![VisualRedactionRegionV1 {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            }],
        },
        workspace_root.path(),
    )
    .expect("redacted native capture should record a governed artifact");
    assert!(result.redaction_applied, "redaction must be recorded");
    assert_eq!(result.artifact.retention_class, "visual-sensitive");

    // The stored payload differs from the unredacted capture: sensitive pixels
    // never reach disk.
    let payload_path = workspace_root
        .path()
        .join(&result.artifact.screenshot_path);
    let stored_bytes = std::fs::read(&payload_path).expect("stored payload bytes");
    assert_ne!(
        stored_bytes, png_bytes,
        "the unredacted PNG must never be stored"
    );
    assert_ne!(
        result.png_sha256,
        format!("sha256:{}", sha256_hex(&png_bytes)),
        "the stored content hash must be the redacted payload's hash"
    );

    // The redacted region is black; pixels outside it are untouched.
    let stored = image::load_from_memory_with_format(&stored_bytes, image::ImageFormat::Png)
        .expect("stored payload decodes as PNG")
        .to_rgba8();
    assert_eq!(stored.dimensions(), (4, 4));
    assert_eq!(stored.get_pixel(0, 0).0, [0, 0, 0, 255]);
    assert_eq!(stored.get_pixel(1, 1).0, [0, 0, 0, 255]);
    assert_eq!(stored.get_pixel(3, 3).0, [255, 255, 255, 255]);
    assert_eq!(stored.get_pixel(2, 0).0, [255, 255, 255, 255]);

    // The manifest carries the MT-158 protection policy: High classification,
    // exportable only because redaction was applied, bounded by the TTL.
    let manifest = read_artifact_manifest(
        workspace_root.path(),
        ArtifactLayer::L1,
        result.artifact_store_id,
    )
    .expect("artifact manifest must be persisted and readable");
    assert_eq!(manifest.classification, ArtifactClassification::High);
    assert!(manifest.exportable);
    assert_eq!(manifest.retention_ttl_days, Some(7));
    assert_eq!(manifest.size_bytes, stored_bytes.len() as u64);
}

/// MT-158 (protection invariants): an unredacted capture must never be
/// exportable, and redaction regions must lie within the capture bounds. A
/// rejected capture writes no artifact.
#[test]
fn mt158_unredacted_export_and_out_of_bounds_regions_rejected() {
    let workspace_root = tempfile::tempdir().expect("temp workspace root");
    let request = request("request.native.invalid", "module://operator/secrets-panel");
    let png_bytes = white_png_bytes(4, 4);

    // (a) exportable without any redaction region -> rejected.
    let unredacted_export = record_native_product_screenshot(
        &request,
        evidence(png_bytes.clone()),
        None,
        &VisualEvidenceProtectionV1 {
            exportable: true,
            ..VisualEvidenceProtectionV1::default()
        },
        workspace_root.path(),
    );
    assert!(
        matches!(
            unredacted_export,
            Err(ProductScreenshotExecutionError::Validation(_))
        ),
        "exportable-without-redaction must be rejected, got {unredacted_export:?}"
    );

    // (b) a redaction region outside the capture bounds -> rejected.
    let out_of_bounds = record_native_product_screenshot(
        &request,
        evidence(png_bytes),
        None,
        &VisualEvidenceProtectionV1 {
            redact_regions: vec![VisualRedactionRegionV1 {
                x: 3,
                y: 3,
                width: 2,
                height: 2,
            }],
            ..VisualEvidenceProtectionV1::default()
        },
        workspace_root.path(),
    );
    assert!(
        matches!(
            out_of_bounds,
            Err(ProductScreenshotExecutionError::Validation(_))
        ),
        "out-of-bounds redaction region must be rejected, got {out_of_bounds:?}"
    );

    // Neither rejected capture wrote an artifact.
    let l1_dir = workspace_root
        .path()
        .join(".handshake")
        .join("artifacts")
        .join("L1");
    let wrote_anything = l1_dir
        .read_dir()
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);
    assert!(
        !wrote_anything,
        "rejected captures must not persist any artifact"
    );
}

/// MT-158 (retention + cleanup, PostgreSQL): retention/redaction metadata
/// round-trips through the real store, the exportable-without-redaction
/// invariant is enforced on the row, and the cleanup pass prunes expired
/// unpinned rows (emitting EventLedger events) while pinned / no-TTL rows
/// survive.
#[tokio::test]
async fn mt158_retention_cleanup_prunes_expired_unpinned_rows() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt158_retention_cleanup_prunes_expired_unpinned_rows: PostgreSQL unavailable");
        return;
    };
    let store = AtelierStore::connect(&url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");

    // An exportable row without redaction_applied is rejected before any write.
    let rejected = store
        .record_screenshot_artifact_storage(&stored_capture_input(
            &store,
            "unredacted-exportable",
            Some(30),
            false,
            true,
            false,
        )
        .await)
        .await
        .expect_err("exportable-without-redaction row must be rejected");
    assert!(
        matches!(rejected, AtelierError::Validation(_)),
        "exportable-without-redaction must be a Validation error, got {rejected:?}"
    );

    // Three real capture rows: expired (ttl 0), pinned (ttl 0), and no-TTL.
    let expired = store
        .record_screenshot_artifact_storage(
            &stored_capture_input(&store, "expired", Some(0), false, true, true).await,
        )
        .await
        .expect("store expired capture row");
    assert_eq!(expired.retention_class, "visual-sensitive");
    assert!(expired.exportable);
    assert!(expired.redaction_applied);
    let pinned = store
        .record_screenshot_artifact_storage(
            &stored_capture_input(&store, "pinned", Some(0), true, false, false).await,
        )
        .await
        .expect("store pinned capture row");
    let unbounded = store
        .record_screenshot_artifact_storage(
            &stored_capture_input(&store, "unbounded", None, false, false, false).await,
        )
        .await
        .expect("store no-ttl capture row");

    // Cleanup prunes the expired row, never the pinned or no-TTL rows.
    let cleaned = store
        .cleanup_expired_screenshot_artifacts()
        .await
        .expect("run retention cleanup");
    assert!(
        cleaned.iter().any(|row| row.storage_id == expired.storage_id),
        "the expired unpinned row must be pruned"
    );
    assert!(
        cleaned.iter().all(|row| row.storage_id != pinned.storage_id),
        "a pinned row must never be pruned"
    );
    assert!(
        cleaned
            .iter()
            .all(|row| row.storage_id != unbounded.storage_id),
        "a row without a TTL must never be pruned"
    );

    // Re-read from PostgreSQL: the expired row is gone, the survivors remain
    // with their retention metadata intact.
    let remaining = store
        .list_screenshot_artifact_storage()
        .await
        .expect("list screenshot artifact storage after cleanup");
    assert!(
        remaining.iter().all(|row| row.storage_id != expired.storage_id),
        "the pruned row must not be readable after cleanup"
    );
    let surviving_pinned = remaining
        .iter()
        .find(|row| row.storage_id == pinned.storage_id)
        .expect("pinned row must survive cleanup");
    assert!(surviving_pinned.pinned);
    assert_eq!(surviving_pinned.retention_ttl_days, Some(0));
    assert!(
        remaining.iter().any(|row| row.storage_id == unbounded.storage_id),
        "the no-ttl row must survive cleanup"
    );

    // The cleanup emitted exactly one EventLedger event for the pruned row and
    // none for the survivors.
    let cleaned_events = store
        .count_events_for_aggregate(
            diagnostics_projection_event_family::SCREENSHOT_ARTIFACT_RETENTION_CLEANED,
            "atelier_screenshot_artifact_storage",
            &expired.storage_id,
        )
        .await
        .expect("count retention-cleanup events for the pruned row");
    assert_eq!(
        cleaned_events, 1,
        "pruning a row must emit exactly one SCREENSHOT_ARTIFACT_RETENTION_CLEANED event"
    );
    for survivor in [&pinned.storage_id, &unbounded.storage_id] {
        let events = store
            .count_events_for_aggregate(
                diagnostics_projection_event_family::SCREENSHOT_ARTIFACT_RETENTION_CLEANED,
                "atelier_screenshot_artifact_storage",
                survivor,
            )
            .await
            .expect("count retention-cleanup events for a surviving row");
        assert_eq!(events, 0, "surviving rows must not emit cleanup events");
    }
}

// --- helpers -----------------------------------------------------------------

/// Create a real stealth window + capture receipt and build the storage input
/// for it (the MT-153 base surface the MT-158 retention policy governs).
async fn stored_capture_input(
    store: &AtelierStore,
    label: &str,
    retention_ttl_days: Option<i32>,
    pinned: bool,
    exportable: bool,
    redaction_applied: bool,
) -> NewScreenshotArtifactStorage {
    let window = store
        .create_stealth_window(&NewStealthWindow {
            owner_actor: format!("operator-{}", Uuid::new_v4()),
            title: format!("retention-window-{label}-{}", Uuid::new_v4()),
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
    NewScreenshotArtifactStorage {
        storage_id: format!("sas-{label}-{}", Uuid::new_v4()),
        capture_id: capture.capture_id,
        artifact_manifest_id: manifest_id,
        content_sha256: sha,
        mime: "image/png".to_string(),
        width_px: Some(4),
        height_px: Some(4),
        byte_len: Some(128),
        label: Some(format!("retention-{label}")),
        retention_ttl_days,
        pinned,
        retention_class: "visual-sensitive".to_string(),
        exportable,
        redaction_applied,
    }
}

fn request(request_id: &str, target_ref: &str) -> ProductScreenshotRequestV1 {
    ProductScreenshotRequestV1 {
        request_id: request_id.to_string(),
        scope: ScreenshotCaptureScope::Module,
        target_ref: target_ref.to_string(),
        requested_by_role: "CODER".to_string(),
        trigger_kind: ScreenshotCaptureTriggerKind::DccApi,
        window_title: "Handshake Desktop Shell".to_string(),
        width: 4,
        height: 4,
        capture_adapter_ref: "capture-adapter://tauri-webview2-cdp".to_string(),
        flight_recorder_ref: format!("FR-EVT-VISUAL-CAPTURE-{}", request_id.replace('.', "-")),
        execution_surface: ScreenshotCaptureExecutionSurface::GovernedAdapterApi,
        workdir_ref: "repo-root://".to_string(),
    }
}

fn evidence(png_bytes: Vec<u8>) -> NativeScreenshotEvidence {
    NativeScreenshotEvidence {
        png_bytes,
        width: 4,
        height: 4,
        scope: ScreenshotCaptureScope::Module,
        captured_at_utc: "2026-06-10T12:00:00Z".to_string(),
        from_surface: true,
        focus_audit_clean: true,
    }
}

fn white_png_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        width,
        height,
        image::Rgba([255, 255, 255, 255]),
    ));
    image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("white png writes");
    bytes
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

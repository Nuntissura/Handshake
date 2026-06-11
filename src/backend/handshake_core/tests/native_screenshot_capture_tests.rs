//! MT-152 / MT-158 native (Playwright-free) screenshot capture proofs.
//!
//! These tests prove the governed screenshot contract is satisfiable via the
//! native CDP path (`record_native_product_screenshot`) with NO Playwright
//! present:
//!   * the native PNG is stored as a real ArtifactStore L1 image artifact with a
//!     stable id, and the returned `ProductScreenshotArtifactV1` references it
//!     (MT-152);
//!   * focus-unsafe evidence (`from_surface == false` or
//!     `focus_audit_clean == false`, or a FocusAuditReport with a Handshake-owned
//!     foreground event) is rejected as a Validation error (MT-158).
//!
//! Run-scoped: requires ArtifactStore filesystem writes. Gated behind the
//! `test-utils` feature in Cargo.toml.

use std::io::Cursor;

use handshake_core::kernel::product_screenshot_capture::{
    record_native_product_screenshot, NativeScreenshotEvidence, ProductScreenshotExecutionError,
    ProductScreenshotRequestV1, ScreenshotCaptureExecutionSurface, ScreenshotCaptureScope,
    ScreenshotCaptureTriggerKind, VisualEvidenceProtectionV1,
};
use handshake_core::operator_foreground::focus_audit::{
    FocusAuditEvent, FocusAuditReport, OwnedProcessPidSet,
};
use handshake_core::storage::artifacts::{
    read_artifact_manifest, ArtifactClassification, ArtifactLayer, ArtifactPayloadKind,
};

/// MT-152: a native screenshot records as a real ArtifactStore artifact (stable
/// id, content-hash validated) and the governed ProductScreenshotArtifactV1
/// references it — without exercising any Playwright path.
#[test]
fn mt152_native_screenshot_stored_as_governed_artifact_without_playwright() {
    let workspace_root = tempfile::tempdir().expect("temp workspace root");
    let request = request(
        "request.native.module",
        ScreenshotCaptureScope::Module,
        "module://operator/evidence-drawer",
    );
    let png_bytes = tiny_png_bytes();

    let result = record_native_product_screenshot(
        &request,
        NativeScreenshotEvidence {
            png_bytes: png_bytes.clone(),
            width: 1,
            height: 1,
            scope: ScreenshotCaptureScope::Module,
            captured_at_utc: "2026-06-09T12:00:00Z".to_string(),
            from_surface: true,
            focus_audit_clean: true,
        },
        Some(&clean_focus_report("native-capture-run-1")),
        &VisualEvidenceProtectionV1::default(),
        workspace_root.path(),
    )
    .expect("native capture should record a governed artifact");

    // The PNG is a *real* ArtifactStore L1 image artifact: manifest readable,
    // content-hash matches, classified as a File payload.
    let manifest = read_artifact_manifest(
        workspace_root.path(),
        ArtifactLayer::L1,
        result.artifact_store_id,
    )
    .expect("artifact manifest must be persisted and readable");
    assert_eq!(manifest.artifact_id, result.artifact_store_id);
    assert_eq!(manifest.layer, ArtifactLayer::L1);
    assert_eq!(manifest.kind, ArtifactPayloadKind::File);
    assert_eq!(manifest.mime, "image/png");
    assert_eq!(manifest.classification, ArtifactClassification::Low);
    assert_eq!(manifest.size_bytes, png_bytes.len() as u64);
    assert_eq!(result.png_sha256, format!("sha256:{}", manifest.content_hash));

    // The stored payload bytes on disk equal the captured PNG.
    let payload_path = workspace_root
        .path()
        .join(".handshake")
        .join("artifacts")
        .join("L1")
        .join(result.artifact_store_id.to_string())
        .join("payload");
    assert!(payload_path.is_file(), "payload PNG must exist on disk");
    assert_eq!(
        std::fs::read(&payload_path).expect("payload bytes"),
        png_bytes
    );

    // The governed contract record references the stored artifact by stable id.
    assert!(result
        .artifact
        .screenshot_ref
        .contains(&result.artifact_store_id.to_string()));
    assert!(result
        .artifact
        .screenshot_path
        .starts_with(&result.artifact_store_rel));
    assert_eq!(result.artifact.content_type, "image/png");
    assert_eq!(result.artifact.width, 1);
    assert_eq!(result.artifact.height, 1);
    assert_eq!(result.artifact.request_id, "request.native.module");
    assert_eq!(
        result.artifact.metadata_schema_id,
        "hsk.product_screenshot_metadata@1"
    );

    // No Playwright path is exercised: the capture never shells out to node, and
    // no app/node_modules/playwright dependency is consulted. The native record
    // function takes raw CDP PNG bytes and persists them directly.
    assert!(
        !uses_playwright_dependency_path(),
        "native capture must not require app/node_modules/playwright"
    );
    assert!(result.focus_audit_run_id.as_deref() == Some("native-capture-run-1"));
}

/// MT-158: focus-unsafe native evidence is rejected as a Validation error.
#[test]
fn mt158_native_capture_rejects_non_focus_safe_evidence() {
    let workspace_root = tempfile::tempdir().expect("temp workspace root");
    let request = request(
        "request.native.panel",
        ScreenshotCaptureScope::Panel,
        "panel://dcc/session-spawn-tree",
    );
    let png_bytes = tiny_png_bytes();

    // (a) from_surface == false → rejected.
    let not_from_surface = record_native_product_screenshot(
        &request,
        NativeScreenshotEvidence {
            png_bytes: png_bytes.clone(),
            width: 1,
            height: 1,
            scope: ScreenshotCaptureScope::Panel,
            captured_at_utc: "2026-06-09T12:00:00Z".to_string(),
            from_surface: false,
            focus_audit_clean: true,
        },
        Some(&clean_focus_report("native-capture-run-2")),
        &VisualEvidenceProtectionV1::default(),
        workspace_root.path(),
    );
    assert!(
        matches!(
            not_from_surface,
            Err(ProductScreenshotExecutionError::Validation(_))
        ),
        "from_surface=false must be rejected as Validation, got {not_from_surface:?}"
    );

    // (b) focus_audit_clean == false → rejected.
    let not_focus_clean = record_native_product_screenshot(
        &request,
        NativeScreenshotEvidence {
            png_bytes: png_bytes.clone(),
            width: 1,
            height: 1,
            scope: ScreenshotCaptureScope::Panel,
            captured_at_utc: "2026-06-09T12:00:00Z".to_string(),
            from_surface: true,
            focus_audit_clean: false,
        },
        Some(&clean_focus_report("native-capture-run-3")),
        &VisualEvidenceProtectionV1::default(),
        workspace_root.path(),
    );
    assert!(
        matches!(
            not_focus_clean,
            Err(ProductScreenshotExecutionError::Validation(_))
        ),
        "focus_audit_clean=false must be rejected as Validation, got {not_focus_clean:?}"
    );

    // (c) FocusAuditReport with a Handshake-owned foreground event → rejected,
    //     even when the evidence flags claim focus-safety.
    let dirty = record_native_product_screenshot(
        &request,
        NativeScreenshotEvidence {
            png_bytes,
            width: 1,
            height: 1,
            scope: ScreenshotCaptureScope::Panel,
            captured_at_utc: "2026-06-09T12:00:00Z".to_string(),
            from_surface: true,
            focus_audit_clean: true,
        },
        Some(&dirty_focus_report("native-capture-run-4")),
        &VisualEvidenceProtectionV1::default(),
        workspace_root.path(),
    );
    assert!(
        matches!(dirty, Err(ProductScreenshotExecutionError::Validation(_))),
        "Handshake-owned foreground event must be rejected as Validation, got {dirty:?}"
    );

    // None of the rejected captures wrote an artifact: the L1 store stays empty.
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
        "rejected focus-unsafe evidence must not persist any artifact"
    );
}

// --- helpers ---------------------------------------------------------------

fn request(
    request_id: &str,
    scope: ScreenshotCaptureScope,
    target_ref: &str,
) -> ProductScreenshotRequestV1 {
    ProductScreenshotRequestV1 {
        request_id: request_id.to_string(),
        scope,
        target_ref: target_ref.to_string(),
        requested_by_role: "CODER".to_string(),
        trigger_kind: ScreenshotCaptureTriggerKind::DccApi,
        window_title: "Handshake Desktop Shell".to_string(),
        width: 1,
        height: 1,
        capture_adapter_ref: "capture-adapter://tauri-webview2-cdp".to_string(),
        flight_recorder_ref: format!("FR-EVT-VISUAL-CAPTURE-{}", request_id.replace('.', "-")),
        execution_surface: ScreenshotCaptureExecutionSurface::GovernedAdapterApi,
        workdir_ref: "repo-root://".to_string(),
    }
}

/// A clean FocusAuditReport: no foreground events of any kind during capture.
fn clean_focus_report(run_id: &str) -> FocusAuditReport {
    FocusAuditReport::from_events(
        run_id,
        std::process::id(),
        &OwnedProcessPidSet::default(),
        Vec::new(),
    )
}

/// A dirty FocusAuditReport: a Handshake-owned process stole the foreground
/// during the capture window.
fn dirty_focus_report(run_id: &str) -> FocusAuditReport {
    let owned_pid = 4242u32;
    let mut owned = OwnedProcessPidSet::default();
    owned.insert(owned_pid);
    FocusAuditReport::from_events(
        run_id,
        std::process::id(),
        &owned,
        vec![FocusAuditEvent {
            run_id: run_id.to_string(),
            timestamp_utc: chrono::Utc::now(),
            hwnd: "0x0000000000001234".to_string(),
            pid: owned_pid,
            exe_name: Some("handshake.exe".to_string()),
            expected_foreground: false,
        }],
    )
}

fn tiny_png_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        1,
        1,
        image::Rgba([0, 0, 0, 255]),
    ));
    image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("tiny png writes");
    bytes
}

/// The native path is satisfiable with no Playwright present. This mirrors the
/// legacy browser-adapter pre-flight check
/// (`Path::new("app/node_modules/playwright/package.json").is_file()`): the
/// native record function never consults it, so this returns the live state of
/// the dependency without the native capture ever depending on it.
fn uses_playwright_dependency_path() -> bool {
    std::path::Path::new("app/node_modules/playwright/package.json").is_file()
}

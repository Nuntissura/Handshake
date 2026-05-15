use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    product_screenshot_capture::{
        project_product_screenshot_capture, validate_product_screenshot_capture,
        ProductScreenshotArtifactV1, ProductScreenshotCaptureV1, ProductScreenshotRequestV1,
        ScreenshotCaptureScope, ScreenshotCaptureTriggerKind,
    },
};

#[test]
fn kernel_product_screenshot_capture_projects_all_capture_scopes() {
    let capture = sample_capture();
    validate_product_screenshot_capture(&capture).expect("screenshot capture validates");

    let projection = project_product_screenshot_capture(&capture).expect("projection builds");

    assert!(projection.full_app_capture_available);
    assert!(projection.panel_capture_available);
    assert!(projection.module_capture_available);
    assert_eq!(projection.request_ids.len(), 3);
    assert_eq!(projection.artifact_ids.len(), 3);
    assert!(projection.metadata_complete);
    assert!(!projection.mutates_authority);
}

#[test]
fn kernel_product_screenshot_capture_preserves_metadata_and_artifact_refs() {
    let projection = project_product_screenshot_capture(&sample_capture()).expect("projection");

    assert!(projection
        .screenshot_refs
        .iter()
        .all(|artifact_ref| artifact_ref.starts_with("artifact://screenshots/")));
    assert!(projection
        .metadata_refs
        .iter()
        .all(|metadata_ref| metadata_ref.starts_with("artifact://metadata/screenshots/")));
    assert!(projection
        .trigger_kinds
        .contains(&ScreenshotCaptureTriggerKind::GovernedCoderCli));
    assert!(projection
        .trigger_kinds
        .contains(&ScreenshotCaptureTriggerKind::GovernedValidatorCli));
}

#[test]
fn kernel_product_screenshot_capture_rejects_missing_scope_metadata_or_artifacts() {
    let mut capture = sample_capture();
    capture
        .supported_scopes
        .retain(|scope| *scope != ScreenshotCaptureScope::Module);
    capture.requests[1].target_ref = "bad-panel-target".to_string();
    capture.artifacts[0].metadata_ref.clear();
    capture.artifacts[1].content_type = "image/jpeg".to_string();
    capture.artifacts[2].request_id = "request.missing".to_string();

    let errors =
        validate_product_screenshot_capture(&capture).expect_err("unsafe capture must fail");

    assert!(errors.iter().any(|error| error.field == "supported_scopes"));
    assert!(errors
        .iter()
        .any(|error| error.field == "requests.target_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "artifacts.metadata_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "artifacts.content_type"));
    assert!(errors
        .iter()
        .any(|error| error.field == "artifacts.request_id"));
}

#[test]
fn kernel_product_screenshot_capture_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.product_screenshot_capture.project")
        .expect("product screenshot capture projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "product_screenshot_artifact_refs"));
}

fn sample_capture() -> ProductScreenshotCaptureV1 {
    ProductScreenshotCaptureV1 {
        schema_id: "hsk.kernel.product_screenshot_capture@1".to_string(),
        capture_id: "product-screenshot-mt045".to_string(),
        folded_stub_ids: vec!["WP-1-Product-Screenshot-Visual-Validation-v1".to_string()],
        supported_scopes: vec![
            ScreenshotCaptureScope::FullApp,
            ScreenshotCaptureScope::Panel,
            ScreenshotCaptureScope::Module,
        ],
        requests: vec![
            request(
                "request.full",
                ScreenshotCaptureScope::FullApp,
                "app://handshake",
                ScreenshotCaptureTriggerKind::GovernedCoderCli,
            ),
            request(
                "request.panel",
                ScreenshotCaptureScope::Panel,
                "panel://dcc/session-spawn-tree",
                ScreenshotCaptureTriggerKind::GovernedValidatorCli,
            ),
            request(
                "request.module",
                ScreenshotCaptureScope::Module,
                "module://operator/evidence-drawer",
                ScreenshotCaptureTriggerKind::DccApi,
            ),
        ],
        artifacts: vec![
            artifact("artifact.full", "request.full"),
            artifact("artifact.panel", "request.panel"),
            artifact("artifact.module", "request.module"),
        ],
        artifact_store_ref: "artifact-store://.handshake/artifacts/screenshots".to_string(),
        product_authority_refs: vec![
            "kernel.dcc_mvp_runtime_surface".to_string(),
            "kernel.action_catalog".to_string(),
            "artifact_store.screenshots".to_string(),
            "flight_recorder.visual_validation".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Product-Screenshot-Visual-Validation-v1.contract.json"
                .to_string(),
        ],
    }
}

fn request(
    request_id: &str,
    scope: ScreenshotCaptureScope,
    target_ref: &str,
    trigger_kind: ScreenshotCaptureTriggerKind,
) -> ProductScreenshotRequestV1 {
    ProductScreenshotRequestV1 {
        request_id: request_id.to_string(),
        scope,
        target_ref: target_ref.to_string(),
        requested_by_role: "CODER".to_string(),
        trigger_kind,
        window_title: "Handshake Desktop Shell".to_string(),
        width: 1440,
        height: 960,
        capture_adapter_ref: "capture-adapter://tauri-webview-or-browser-dom".to_string(),
        flight_recorder_ref: format!("FR-EVT-VISUAL-CAPTURE-{}", request_id.replace('.', "-")),
    }
}

fn artifact(artifact_id: &str, request_id: &str) -> ProductScreenshotArtifactV1 {
    ProductScreenshotArtifactV1 {
        artifact_id: artifact_id.to_string(),
        request_id: request_id.to_string(),
        screenshot_ref: format!("artifact://screenshots/{artifact_id}.png"),
        metadata_ref: format!("artifact://metadata/screenshots/{artifact_id}.json"),
        content_type: "image/png".to_string(),
        width: 1440,
        height: 960,
        captured_at_utc: "2026-05-14T20:00:00Z".to_string(),
        retention_class: "visual-validation".to_string(),
    }
}

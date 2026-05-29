use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    product_screenshot_capture::{
        capture_product_screenshot_from_browser_adapter, execute_product_screenshot_capture,
        project_product_screenshot_capture, validate_product_screenshot_capture,
        ProductScreenshotAdapterCaptureV1, ProductScreenshotArtifactV1,
        ProductScreenshotBrowserAdapterConfigV1, ProductScreenshotCaptureV1,
        ProductScreenshotDurableReceiptV1, ProductScreenshotExecutionError,
        ProductScreenshotExecutionProofV1, ProductScreenshotRequestV1,
        ScreenshotCaptureExecutionSurface, ScreenshotCaptureScope, ScreenshotCaptureTriggerKind,
    },
};
use std::io::Cursor;

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
    assert_eq!(projection.durable_receipt_refs.len(), 3);
    assert_eq!(projection.execution_proof_ids.len(), 3);
    assert!(projection.metadata_complete);
    assert!(projection.real_execution_required);
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
    capture.execution_proofs[0].execution_path = "external://browser-screenshot".to_string();
    capture.execution_proofs[1].command_or_api_ref = "cli://screenshot".to_string();
    capture.durable_receipts[2].receipt_path = "target/screenshots/receipt.json".to_string();

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
    assert!(errors
        .iter()
        .any(|error| error.field == "execution_proofs.execution_path"));
    assert!(errors
        .iter()
        .any(|error| error.field == "execution_proofs.command_or_api_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "durable_receipts.receipt_path"));
}

#[test]
fn kernel_product_screenshot_capture_requires_governed_execution_proof_per_scope() {
    let capture = sample_capture();
    validate_product_screenshot_capture(&capture).expect("screenshot capture validates");

    for request in &capture.requests {
        let artifact = capture
            .artifacts
            .iter()
            .find(|artifact| artifact.request_id == request.request_id)
            .expect("artifact for request");
        let proof = capture
            .execution_proofs
            .iter()
            .find(|proof| proof.request_id == request.request_id)
            .expect("execution proof for request");
        assert_eq!(proof.artifact_ref, artifact.screenshot_ref);
        assert_eq!(proof.metadata_ref, artifact.metadata_ref);
        assert!(proof
            .execution_path
            .starts_with("kernel://product-screenshot-capture/"));
        assert!(proof
            .receipt_ref
            .starts_with("receipt://product-screenshot-capture/"));
        assert_eq!(proof.workdir_ref, "repo-root://");
        assert_eq!(proof.writes_screenshot_ref, proof.artifact_ref);
        assert_eq!(proof.writes_metadata_ref, proof.metadata_ref);
        assert_eq!(proof.writes_receipt_ref, proof.receipt_ref);
    }
}

#[test]
fn kernel_product_screenshot_capture_requires_real_write_receipts_for_all_scopes() {
    let capture = sample_capture();
    validate_product_screenshot_capture(&capture).expect("screenshot capture validates");

    for scope in [
        ScreenshotCaptureScope::FullApp,
        ScreenshotCaptureScope::Panel,
        ScreenshotCaptureScope::Module,
    ] {
        let request = capture
            .requests
            .iter()
            .find(|request| request.scope == scope)
            .expect("request for scope");
        let artifact = capture
            .artifacts
            .iter()
            .find(|artifact| artifact.request_id == request.request_id)
            .expect("artifact for scope");
        let receipt = capture
            .durable_receipts
            .iter()
            .find(|receipt| receipt.request_id == request.request_id)
            .expect("receipt for scope");

        assert!(artifact
            .screenshot_path
            .starts_with("../Handshake_Artifacts/handshake-product/screenshots/"));
        assert!(artifact
            .metadata_path
            .starts_with("../Handshake_Artifacts/handshake-product/screenshots/metadata/"));
        assert!(receipt
            .receipt_path
            .starts_with("../Handshake_Artifacts/handshake-product/screenshots/receipts/"));
        assert!(receipt.records_screenshot_sha256);
        assert!(receipt.records_metadata_sha256);
        assert!(receipt.records_adapter_exit_status);
        assert_eq!(receipt.workdir_ref, "repo-root://");
    }
}

#[test]
fn kernel_product_screenshot_capture_executes_and_writes_png_metadata_and_receipt() {
    let artifact_root = tempfile::tempdir().expect("temp artifact root");
    let request = request(
        "request.execute.panel",
        ScreenshotCaptureScope::Panel,
        "panel://dcc/session-spawn-tree",
        ScreenshotCaptureTriggerKind::GovernedValidatorCli,
    );
    let png_bytes = tiny_png_bytes();

    let execution = execute_product_screenshot_capture(
        &request,
        ProductScreenshotAdapterCaptureV1 {
            png_bytes: png_bytes.clone(),
            adapter_exit_status: 0,
            captured_at_utc: "2026-05-16T10:00:00Z".to_string(),
            command_or_api_ref:
                "cli://handshake screenshot capture --scope panel --target panel://dcc/session-spawn-tree --write-metadata --write-receipt"
                    .to_string(),
        },
        artifact_root.path(),
    )
    .expect("screenshot execution should write artifacts");

    assert!(execution.screenshot_path.exists());
    assert!(execution.metadata_path.exists());
    assert!(execution.receipt_path.exists());
    assert_eq!(
        std::fs::read(&execution.screenshot_path).expect("screenshot bytes"),
        png_bytes
    );
    assert_eq!(execution.artifact.content_type, "image/png");
    assert_eq!(execution.artifact.width, 1);
    assert_eq!(execution.artifact.height, 1);
    assert_eq!(
        execution.durable_receipt.scope,
        ScreenshotCaptureScope::Panel
    );
    assert!(execution.receipt.screenshot_sha256.starts_with("sha256:"));
    assert!(execution.receipt.metadata_sha256.starts_with("sha256:"));
    assert_eq!(execution.receipt.adapter_exit_status, 0);
    assert_eq!(execution.receipt.scope, ScreenshotCaptureScope::Panel);
    assert_eq!(
        execution.receipt.command_or_api_ref,
        execution.proof.command_or_api_ref
    );
    assert_eq!(
        execution.receipt.artifact_ref,
        execution.artifact.screenshot_ref
    );
    assert_eq!(
        execution.receipt.metadata_ref,
        execution.artifact.metadata_ref
    );
    assert_eq!(
        execution.receipt.receipt_ref,
        execution.durable_receipt.receipt_ref
    );
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

    let execute_action = catalog
        .action("kernel.product_screenshot_capture.execute")
        .expect("product screenshot capture execute action must be cataloged");
    assert_eq!(
        execute_action.input_schema_id,
        "hsk.kernel.product_screenshot_capture_execute_request@1"
    );
    assert_eq!(
        execute_action.result_schema_id,
        "hsk.kernel.product_screenshot_capture_execute_result@1"
    );
    assert_eq!(
        execute_action.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert!(execute_action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "product_screenshot_capture_png_artifact_written"));
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
            artifact("artifact.full", "request.full", "full-app"),
            artifact("artifact.panel", "request.panel", "panel"),
            artifact("artifact.module", "request.module", "module"),
        ],
        durable_receipts: vec![
            receipt("receipt.full", "request.full", ScreenshotCaptureScope::FullApp),
            receipt("receipt.panel", "request.panel", ScreenshotCaptureScope::Panel),
            receipt("receipt.module", "request.module", ScreenshotCaptureScope::Module),
        ],
        execution_proofs: vec![
            proof(
                "proof.full",
                "request.full",
                "artifact://screenshots/artifact.full.png",
                "artifact://metadata/screenshots/artifact.full.json",
                "receipt://product-screenshot-capture/receipt.full",
                ScreenshotCaptureExecutionSurface::GovernedAdapterCli,
                "cli://handshake screenshot capture --scope full-app --write-metadata --write-receipt",
            ),
            proof(
                "proof.panel",
                "request.panel",
                "artifact://screenshots/artifact.panel.png",
                "artifact://metadata/screenshots/artifact.panel.json",
                "receipt://product-screenshot-capture/receipt.panel",
                ScreenshotCaptureExecutionSurface::GovernedAdapterCli,
                "cli://handshake screenshot capture --scope panel --target panel://dcc/session-spawn-tree --write-metadata --write-receipt",
            ),
            proof(
                "proof.module",
                "request.module",
                "artifact://screenshots/artifact.module.png",
                "artifact://metadata/screenshots/artifact.module.json",
                "receipt://product-screenshot-capture/receipt.module",
                ScreenshotCaptureExecutionSurface::GovernedAdapterApi,
                "api://kernel.product_screenshot_capture.execute",
            ),
        ],
        artifact_store_ref: "artifact-store://../Handshake_Artifacts/handshake-product/screenshots"
            .to_string(),
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

fn proof(
    proof_id: &str,
    request_id: &str,
    artifact_ref: &str,
    metadata_ref: &str,
    receipt_ref: &str,
    execution_surface: ScreenshotCaptureExecutionSurface,
    command_or_api_ref: &str,
) -> ProductScreenshotExecutionProofV1 {
    ProductScreenshotExecutionProofV1 {
        proof_id: proof_id.to_string(),
        request_id: request_id.to_string(),
        adapter_ref: "capture-adapter://tauri-webview-or-browser-dom".to_string(),
        execution_surface,
        execution_path: format!("kernel://product-screenshot-capture/{request_id}"),
        command_or_api_ref: command_or_api_ref.to_string(),
        workdir_ref: "repo-root://".to_string(),
        metadata_ref: metadata_ref.to_string(),
        artifact_ref: artifact_ref.to_string(),
        receipt_ref: receipt_ref.to_string(),
        writes_screenshot_ref: artifact_ref.to_string(),
        writes_metadata_ref: metadata_ref.to_string(),
        writes_receipt_ref: receipt_ref.to_string(),
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
        execution_surface: match trigger_kind {
            ScreenshotCaptureTriggerKind::DccApi => {
                ScreenshotCaptureExecutionSurface::GovernedAdapterApi
            }
            _ => ScreenshotCaptureExecutionSurface::GovernedAdapterCli,
        },
        workdir_ref: "repo-root://".to_string(),
    }
}

fn artifact(artifact_id: &str, request_id: &str, file_stem: &str) -> ProductScreenshotArtifactV1 {
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
        screenshot_path: format!(
            "../Handshake_Artifacts/handshake-product/screenshots/{file_stem}.png"
        ),
        metadata_path: format!(
            "../Handshake_Artifacts/handshake-product/screenshots/metadata/{file_stem}.json"
        ),
        metadata_schema_id: "hsk.product_screenshot_metadata@1".to_string(),
    }
}

// --- MT-045 capture E2E + dep checks -----------------------------------------

#[test]
fn kernel_product_screenshot_capture_browser_adapter_returns_typed_dependency_missing_for_unavailable_node(
) {
    let artifact_root = tempfile::tempdir().expect("temp artifact root");
    let request = request(
        "request.dep.missing.node",
        ScreenshotCaptureScope::Module,
        "module://kernel-dcc-module",
        ScreenshotCaptureTriggerKind::GovernedCoderCli,
    );

    let result = capture_product_screenshot_from_browser_adapter(
        &request,
        ProductScreenshotBrowserAdapterConfigV1 {
            source_url: "http://127.0.0.1:65535/".to_string(),
            adapter_script_path: "app/scripts/handshake-screenshot-capture.mjs".to_string(),
            // Deliberately unresolvable binary so the pre-flight node check
            // fails before we ever try to spawn the adapter.
            node_binary: "definitely-not-node-handshake-test-xyz".to_string(),
            command_or_api_ref: "test://node-missing".to_string(),
        },
        artifact_root.path(),
    );

    match result {
        Err(ProductScreenshotExecutionError::AdapterDependencyMissing { dep, hint }) => {
            assert_eq!(dep, "node");
            assert!(
                hint.contains("Node 20+"),
                "node hint should mention Node 20+, got {hint:?}"
            );
        }
        other => panic!("expected AdapterDependencyMissing(node), got {other:?}"),
    }
}

// E2E smoke: boot a tiny axum server, point the capture adapter at it, and
// verify the produced PNG decodes successfully at the requested dimensions for
// each capture scope (FullApp, Panel, Module). Gated with #[ignore] so test
// runners without Node + Playwright skip it cleanly; opt in with
// `cargo test --test kernel_product_screenshot_capture_tests -- --ignored
// capture_browser_adapter_e2e`.
#[test]
#[ignore = "requires Node + Playwright; opt in with --ignored"]
fn kernel_product_screenshot_capture_browser_adapter_e2e_produces_decodable_pngs_for_every_scope() {
    use axum::{routing::get, Router};
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tokio::runtime::Builder;

    let artifact_root = tempfile::tempdir().expect("temp artifact root");
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let listener = runtime
        .block_on(async { TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await })
        .expect("bind ephemeral port");
    let local_addr = listener.local_addr().expect("local addr");
    let app: Router = Router::new().route(
        "/",
        get(|| async {
            axum::response::Html(
                r#"<!doctype html><html><body>
                    <div data-testid="kernel-dcc-module" style="width:200px;height:200px;background:#abcdef;">DCC Module</div>
                    <div id="kernel-dcc-panel" aria-labelledby="kernel-dcc-panel" style="width:200px;height:200px;background:#fedcba;">DCC Panel</div>
                </body></html>"#,
            )
        }),
    );
    let server_handle = runtime.spawn(async move {
        axum::serve(listener, app).await.expect("server runs");
    });

    let source_url = format!("http://{}/", local_addr);
    for (scope, target_ref) in [
        (
            ScreenshotCaptureScope::FullApp,
            "app://handshake".to_string(),
        ),
        (
            ScreenshotCaptureScope::Panel,
            "panel://kernel-dcc-panel".to_string(),
        ),
        (
            ScreenshotCaptureScope::Module,
            "module://kernel-dcc-module".to_string(),
        ),
    ] {
        let request = request(
            &format!("request.e2e.{}", scope_label(scope)),
            scope,
            &target_ref,
            ScreenshotCaptureTriggerKind::GovernedCoderCli,
        );
        let result = capture_product_screenshot_from_browser_adapter(
            &request,
            ProductScreenshotBrowserAdapterConfigV1 {
                source_url: source_url.clone(),
                adapter_script_path: "app/scripts/handshake-screenshot-capture.mjs".to_string(),
                node_binary: "node".to_string(),
                command_or_api_ref: format!("e2e://capture-{:?}", scope),
            },
            artifact_root.path(),
        )
        .unwrap_or_else(|err| panic!("capture for {:?} failed: {err:?}", scope));

        let png_bytes = std::fs::read(&result.screenshot_path).expect("png bytes on disk");
        let decoded = image::load_from_memory(&png_bytes).expect("png decodes");
        assert!(decoded.width() > 0 && decoded.height() > 0);
    }

    server_handle.abort();
}

fn scope_label(scope: ScreenshotCaptureScope) -> &'static str {
    match scope {
        ScreenshotCaptureScope::FullApp => "full-app",
        ScreenshotCaptureScope::Panel => "panel",
        ScreenshotCaptureScope::Module => "module",
    }
}

fn receipt(
    receipt_id: &str,
    request_id: &str,
    scope: ScreenshotCaptureScope,
) -> ProductScreenshotDurableReceiptV1 {
    ProductScreenshotDurableReceiptV1 {
        receipt_id: receipt_id.to_string(),
        request_id: request_id.to_string(),
        scope,
        receipt_ref: format!("receipt://product-screenshot-capture/{receipt_id}"),
        receipt_path: format!(
            "../Handshake_Artifacts/handshake-product/screenshots/receipts/{receipt_id}.json"
        ),
        workdir_ref: "repo-root://".to_string(),
        execution_surface: match scope {
            ScreenshotCaptureScope::Module => ScreenshotCaptureExecutionSurface::GovernedAdapterApi,
            _ => ScreenshotCaptureExecutionSurface::GovernedAdapterCli,
        },
        records_screenshot_sha256: true,
        records_metadata_sha256: true,
        records_adapter_exit_status: true,
    }
}

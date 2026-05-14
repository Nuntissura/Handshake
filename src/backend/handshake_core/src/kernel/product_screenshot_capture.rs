use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const FOLDED_PRODUCT_SCREENSHOT_VISUAL_VALIDATION_STUB_ID: &str =
    "WP-1-Product-Screenshot-Visual-Validation-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScreenshotCaptureScope {
    FullApp,
    Panel,
    Module,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScreenshotCaptureTriggerKind {
    GovernedCoderCli,
    GovernedValidatorCli,
    DccApi,
    LocalModelCli,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductScreenshotRequestV1 {
    pub request_id: String,
    pub scope: ScreenshotCaptureScope,
    pub target_ref: String,
    pub requested_by_role: String,
    pub trigger_kind: ScreenshotCaptureTriggerKind,
    pub window_title: String,
    pub width: u32,
    pub height: u32,
    pub capture_adapter_ref: String,
    pub flight_recorder_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductScreenshotArtifactV1 {
    pub artifact_id: String,
    pub request_id: String,
    pub screenshot_ref: String,
    pub metadata_ref: String,
    pub content_type: String,
    pub width: u32,
    pub height: u32,
    pub captured_at_utc: String,
    pub retention_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductScreenshotCaptureV1 {
    pub schema_id: String,
    pub capture_id: String,
    pub folded_stub_ids: Vec<String>,
    pub supported_scopes: Vec<ScreenshotCaptureScope>,
    pub requests: Vec<ProductScreenshotRequestV1>,
    pub artifacts: Vec<ProductScreenshotArtifactV1>,
    pub artifact_store_ref: String,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductScreenshotCaptureProjectionV1 {
    pub schema_id: String,
    pub capture_id: String,
    pub request_ids: Vec<String>,
    pub artifact_ids: Vec<String>,
    pub screenshot_refs: Vec<String>,
    pub metadata_refs: Vec<String>,
    pub trigger_kinds: Vec<ScreenshotCaptureTriggerKind>,
    pub full_app_capture_available: bool,
    pub panel_capture_available: bool,
    pub module_capture_available: bool,
    pub metadata_complete: bool,
    pub mutates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductScreenshotCaptureValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_product_screenshot_capture(
    capture: &ProductScreenshotCaptureV1,
) -> Result<(), Vec<ProductScreenshotCaptureValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &capture.schema_id);
    require_non_empty(&mut errors, "capture_id", &capture.capture_id);
    require_vec(&mut errors, "folded_stub_ids", &capture.folded_stub_ids);
    require_vec(&mut errors, "supported_scopes", &capture.supported_scopes);
    require_vec(&mut errors, "requests", &capture.requests);
    require_vec(&mut errors, "artifacts", &capture.artifacts);
    require_non_empty(
        &mut errors,
        "artifact_store_ref",
        &capture.artifact_store_ref,
    );
    require_vec(
        &mut errors,
        "product_authority_refs",
        &capture.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &capture.folded_source_refs,
    );

    if !contains_exact(
        &capture.folded_stub_ids,
        FOLDED_PRODUCT_SCREENSHOT_VISUAL_VALIDATION_STUB_ID,
    ) {
        errors.push(ProductScreenshotCaptureValidationError {
            field: "folded_stub_ids",
            message: "product screenshot capture must preserve the folded stub id",
        });
    }
    if !contains_text(
        &capture.folded_source_refs,
        FOLDED_PRODUCT_SCREENSHOT_VISUAL_VALIDATION_STUB_ID,
    ) {
        errors.push(ProductScreenshotCaptureValidationError {
            field: "folded_source_refs",
            message: "product screenshot capture must preserve the folded source reference",
        });
    }

    validate_supported_scopes(&mut errors, capture);
    validate_authority_refs(&mut errors, capture);
    validate_requests(&mut errors, capture);
    validate_artifacts(&mut errors, capture);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_product_screenshot_capture(
    capture: &ProductScreenshotCaptureV1,
) -> Result<ProductScreenshotCaptureProjectionV1, Vec<ProductScreenshotCaptureValidationError>> {
    validate_product_screenshot_capture(capture)?;

    Ok(ProductScreenshotCaptureProjectionV1 {
        schema_id: "hsk.kernel.product_screenshot_capture_projection@1".to_string(),
        capture_id: capture.capture_id.clone(),
        request_ids: capture
            .requests
            .iter()
            .map(|request| request.request_id.clone())
            .collect(),
        artifact_ids: capture
            .artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.clone())
            .collect(),
        screenshot_refs: capture
            .artifacts
            .iter()
            .map(|artifact| artifact.screenshot_ref.clone())
            .collect(),
        metadata_refs: capture
            .artifacts
            .iter()
            .map(|artifact| artifact.metadata_ref.clone())
            .collect(),
        trigger_kinds: ordered_trigger_kinds(capture),
        full_app_capture_available: scope_supported_and_requested(
            capture,
            ScreenshotCaptureScope::FullApp,
        ),
        panel_capture_available: scope_supported_and_requested(
            capture,
            ScreenshotCaptureScope::Panel,
        ),
        module_capture_available: scope_supported_and_requested(
            capture,
            ScreenshotCaptureScope::Module,
        ),
        metadata_complete: capture.artifacts.iter().all(|artifact| {
            !artifact.metadata_ref.is_empty()
                && artifact.width > 0
                && artifact.height > 0
                && !artifact.captured_at_utc.is_empty()
        }),
        mutates_authority: false,
    })
}

fn validate_supported_scopes(
    errors: &mut Vec<ProductScreenshotCaptureValidationError>,
    capture: &ProductScreenshotCaptureV1,
) {
    for scope in [
        ScreenshotCaptureScope::FullApp,
        ScreenshotCaptureScope::Panel,
        ScreenshotCaptureScope::Module,
    ] {
        if !capture.supported_scopes.contains(&scope) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "supported_scopes",
                message: "screenshot capture must support full app, panel, and module scopes",
            });
        }
        if !capture
            .requests
            .iter()
            .any(|request| request.scope == scope)
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "requests.scope",
                message: "screenshot capture must include a request for each supported scope",
            });
        }
    }
}

fn validate_authority_refs(
    errors: &mut Vec<ProductScreenshotCaptureValidationError>,
    capture: &ProductScreenshotCaptureV1,
) {
    for required_ref in [
        "kernel.dcc_mvp_runtime_surface",
        "kernel.action_catalog",
        "artifact_store.screenshots",
        "flight_recorder.visual_validation",
    ] {
        if !contains_exact(&capture.product_authority_refs, required_ref) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "product_authority_refs",
                message: "screenshot capture must cite DCC, action catalog, artifact store, and visual Flight Recorder authorities",
            });
        }
    }
}

fn validate_requests(
    errors: &mut Vec<ProductScreenshotCaptureValidationError>,
    capture: &ProductScreenshotCaptureV1,
) {
    let mut request_ids = HashSet::new();

    for request in &capture.requests {
        if !request_ids.insert(request.request_id.as_str()) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "requests.request_id",
                message: "screenshot request ids must be unique",
            });
        }
        require_non_empty(errors, "requests.request_id", &request.request_id);
        require_non_empty(errors, "requests.target_ref", &request.target_ref);
        require_non_empty(
            errors,
            "requests.requested_by_role",
            &request.requested_by_role,
        );
        require_non_empty(errors, "requests.window_title", &request.window_title);
        require_non_empty(
            errors,
            "requests.capture_adapter_ref",
            &request.capture_adapter_ref,
        );
        require_non_empty(
            errors,
            "requests.flight_recorder_ref",
            &request.flight_recorder_ref,
        );

        if !target_matches_scope(request.scope, &request.target_ref) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "requests.target_ref",
                message: "screenshot target ref must match its capture scope",
            });
        }
        if request.width == 0 || request.height == 0 {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "requests.dimensions",
                message: "screenshot request dimensions must be positive",
            });
        }
        if !request
            .capture_adapter_ref
            .starts_with("capture-adapter://")
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "requests.capture_adapter_ref",
                message: "screenshot capture must use a typed capture adapter ref",
            });
        }
        if !request
            .flight_recorder_ref
            .starts_with("FR-EVT-VISUAL-CAPTURE-")
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "requests.flight_recorder_ref",
                message: "screenshot capture must emit visual capture Flight Recorder refs",
            });
        }
    }
}

fn validate_artifacts(
    errors: &mut Vec<ProductScreenshotCaptureValidationError>,
    capture: &ProductScreenshotCaptureV1,
) {
    let request_ids: HashSet<&str> = capture
        .requests
        .iter()
        .map(|request| request.request_id.as_str())
        .collect();
    let mut artifact_ids = HashSet::new();

    for artifact in &capture.artifacts {
        if !artifact_ids.insert(artifact.artifact_id.as_str()) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "artifacts.artifact_id",
                message: "screenshot artifact ids must be unique",
            });
        }
        require_non_empty(errors, "artifacts.artifact_id", &artifact.artifact_id);
        require_non_empty(errors, "artifacts.request_id", &artifact.request_id);
        require_non_empty(errors, "artifacts.screenshot_ref", &artifact.screenshot_ref);
        require_non_empty(errors, "artifacts.metadata_ref", &artifact.metadata_ref);
        require_non_empty(errors, "artifacts.content_type", &artifact.content_type);
        require_non_empty(
            errors,
            "artifacts.captured_at_utc",
            &artifact.captured_at_utc,
        );
        require_non_empty(
            errors,
            "artifacts.retention_class",
            &artifact.retention_class,
        );

        if !request_ids.contains(artifact.request_id.as_str()) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "artifacts.request_id",
                message: "screenshot artifact must link an existing capture request",
            });
        }
        if !artifact
            .screenshot_ref
            .starts_with("artifact://screenshots/")
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "artifacts.screenshot_ref",
                message: "screenshot artifacts must use screenshot artifact refs",
            });
        }
        if !artifact
            .metadata_ref
            .starts_with("artifact://metadata/screenshots/")
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "artifacts.metadata_ref",
                message: "screenshot artifacts must include metadata artifact refs",
            });
        }
        if artifact.content_type != "image/png" {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "artifacts.content_type",
                message: "screenshot artifacts must be PNG for deterministic visual validation",
            });
        }
        if artifact.width == 0 || artifact.height == 0 {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "artifacts.dimensions",
                message: "screenshot artifact dimensions must be positive",
            });
        }
    }
}

fn target_matches_scope(scope: ScreenshotCaptureScope, target_ref: &str) -> bool {
    match scope {
        ScreenshotCaptureScope::FullApp => target_ref.starts_with("app://"),
        ScreenshotCaptureScope::Panel => target_ref.starts_with("panel://"),
        ScreenshotCaptureScope::Module => target_ref.starts_with("module://"),
    }
}

fn scope_supported_and_requested(
    capture: &ProductScreenshotCaptureV1,
    scope: ScreenshotCaptureScope,
) -> bool {
    capture.supported_scopes.contains(&scope)
        && capture
            .requests
            .iter()
            .any(|request| request.scope == scope)
}

fn ordered_trigger_kinds(
    capture: &ProductScreenshotCaptureV1,
) -> Vec<ScreenshotCaptureTriggerKind> {
    [
        ScreenshotCaptureTriggerKind::GovernedCoderCli,
        ScreenshotCaptureTriggerKind::GovernedValidatorCli,
        ScreenshotCaptureTriggerKind::DccApi,
        ScreenshotCaptureTriggerKind::LocalModelCli,
    ]
    .into_iter()
    .filter(|trigger_kind| {
        capture
            .requests
            .iter()
            .any(|request| request.trigger_kind == *trigger_kind)
    })
    .collect()
}

fn require_non_empty(
    errors: &mut Vec<ProductScreenshotCaptureValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(ProductScreenshotCaptureValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<ProductScreenshotCaptureValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(ProductScreenshotCaptureValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}

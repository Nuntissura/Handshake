use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScreenshotCaptureExecutionSurface {
    GovernedAdapterCli,
    GovernedAdapterApi,
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
    pub execution_surface: ScreenshotCaptureExecutionSurface,
    pub workdir_ref: String,
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
    pub screenshot_path: String,
    pub metadata_path: String,
    pub metadata_schema_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductScreenshotDurableReceiptV1 {
    pub receipt_id: String,
    pub request_id: String,
    pub scope: ScreenshotCaptureScope,
    pub receipt_ref: String,
    pub receipt_path: String,
    pub workdir_ref: String,
    pub execution_surface: ScreenshotCaptureExecutionSurface,
    pub records_screenshot_sha256: bool,
    pub records_metadata_sha256: bool,
    pub records_adapter_exit_status: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductScreenshotExecutionProofV1 {
    pub proof_id: String,
    pub request_id: String,
    pub adapter_ref: String,
    pub execution_surface: ScreenshotCaptureExecutionSurface,
    pub execution_path: String,
    pub command_or_api_ref: String,
    pub workdir_ref: String,
    pub metadata_ref: String,
    pub artifact_ref: String,
    pub receipt_ref: String,
    pub writes_screenshot_ref: String,
    pub writes_metadata_ref: String,
    pub writes_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductScreenshotAdapterCaptureV1 {
    pub png_bytes: Vec<u8>,
    pub adapter_exit_status: i32,
    pub captured_at_utc: String,
    pub command_or_api_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductScreenshotBrowserAdapterConfigV1 {
    pub source_url: String,
    pub adapter_script_path: String,
    pub node_binary: String,
    pub command_or_api_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductScreenshotMetadataV1 {
    pub schema_id: String,
    pub request_id: String,
    pub scope: ScreenshotCaptureScope,
    pub target_ref: String,
    pub width: u32,
    pub height: u32,
    pub captured_at_utc: String,
    pub capture_adapter_ref: String,
    pub command_or_api_ref: String,
    pub flight_recorder_ref: String,
    pub workdir_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductScreenshotExecutionReceiptV1 {
    pub schema_id: String,
    pub request_id: String,
    pub scope: ScreenshotCaptureScope,
    pub command_or_api_ref: String,
    pub artifact_ref: String,
    pub metadata_ref: String,
    pub receipt_ref: String,
    pub screenshot_path: String,
    pub metadata_path: String,
    pub receipt_path: String,
    pub screenshot_sha256: String,
    pub metadata_sha256: String,
    pub adapter_exit_status: i32,
    pub workdir_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductScreenshotExecutionResultV1 {
    pub artifact: ProductScreenshotArtifactV1,
    pub durable_receipt: ProductScreenshotDurableReceiptV1,
    pub proof: ProductScreenshotExecutionProofV1,
    pub metadata: ProductScreenshotMetadataV1,
    pub receipt: ProductScreenshotExecutionReceiptV1,
    pub screenshot_path: PathBuf,
    pub metadata_path: PathBuf,
    pub receipt_path: PathBuf,
}

#[derive(Debug)]
pub enum ProductScreenshotExecutionError {
    InvalidRequest(&'static str),
    InvalidPng(String),
    AdapterFailed {
        status_code: Option<i32>,
        stderr: String,
    },
    MissingAdapterOutput(PathBuf),
    Io(std::io::Error),
    Serialize(serde_json::Error),
}

impl From<std::io::Error> for ProductScreenshotExecutionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ProductScreenshotExecutionError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialize(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductScreenshotCaptureV1 {
    pub schema_id: String,
    pub capture_id: String,
    pub folded_stub_ids: Vec<String>,
    pub supported_scopes: Vec<ScreenshotCaptureScope>,
    pub requests: Vec<ProductScreenshotRequestV1>,
    pub artifacts: Vec<ProductScreenshotArtifactV1>,
    pub durable_receipts: Vec<ProductScreenshotDurableReceiptV1>,
    pub execution_proofs: Vec<ProductScreenshotExecutionProofV1>,
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
    pub durable_receipt_refs: Vec<String>,
    pub execution_proof_ids: Vec<String>,
    pub trigger_kinds: Vec<ScreenshotCaptureTriggerKind>,
    pub full_app_capture_available: bool,
    pub panel_capture_available: bool,
    pub module_capture_available: bool,
    pub metadata_complete: bool,
    pub real_execution_required: bool,
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
    require_vec(&mut errors, "durable_receipts", &capture.durable_receipts);
    require_vec(&mut errors, "execution_proofs", &capture.execution_proofs);
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
    validate_durable_receipts(&mut errors, capture);
    validate_execution_proofs(&mut errors, capture);

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
        durable_receipt_refs: capture
            .durable_receipts
            .iter()
            .map(|receipt| receipt.receipt_ref.clone())
            .collect(),
        execution_proof_ids: capture
            .execution_proofs
            .iter()
            .map(|proof| proof.proof_id.clone())
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
                && !artifact.screenshot_path.is_empty()
                && !artifact.metadata_path.is_empty()
                && !artifact.metadata_schema_id.is_empty()
        }),
        real_execution_required: true,
        mutates_authority: false,
    })
}

pub fn execute_product_screenshot_capture(
    request: &ProductScreenshotRequestV1,
    adapter_capture: ProductScreenshotAdapterCaptureV1,
    artifact_root: impl AsRef<Path>,
) -> Result<ProductScreenshotExecutionResultV1, ProductScreenshotExecutionError> {
    if request.request_id.trim().is_empty() {
        return Err(ProductScreenshotExecutionError::InvalidRequest(
            "request_id is required",
        ));
    }
    if !target_matches_scope(request.scope, &request.target_ref) {
        return Err(ProductScreenshotExecutionError::InvalidRequest(
            "target_ref must match screenshot scope",
        ));
    }
    if !uses_governed_capture_surface(
        request.execution_surface,
        &adapter_capture.command_or_api_ref,
    ) {
        return Err(ProductScreenshotExecutionError::InvalidRequest(
            "command_or_api_ref must use the governed capture surface",
        ));
    }

    let decoded =
        image::load_from_memory_with_format(&adapter_capture.png_bytes, image::ImageFormat::Png)
            .map_err(|err| ProductScreenshotExecutionError::InvalidPng(err.to_string()))?;
    let width = decoded.width();
    let height = decoded.height();
    if width == 0 || height == 0 {
        return Err(ProductScreenshotExecutionError::InvalidPng(
            "PNG dimensions must be positive".to_string(),
        ));
    }

    let artifact_root = artifact_root.as_ref();
    let metadata_dir = artifact_root.join("metadata");
    let receipt_dir = artifact_root.join("receipts");
    fs::create_dir_all(&metadata_dir)?;
    fs::create_dir_all(&receipt_dir)?;

    let file_stem = sanitize_artifact_segment(&request.request_id);
    let screenshot_path = artifact_root.join(format!("{file_stem}.png"));
    let metadata_path = metadata_dir.join(format!("{file_stem}.json"));
    let receipt_path = receipt_dir.join(format!("{file_stem}.json"));
    let artifact_id = format!("artifact.{file_stem}");
    let receipt_id = format!("receipt.{file_stem}");

    fs::write(&screenshot_path, &adapter_capture.png_bytes)?;
    let screenshot_sha256 = sha256_prefixed(&adapter_capture.png_bytes);

    let metadata = ProductScreenshotMetadataV1 {
        schema_id: "hsk.product_screenshot_metadata@1".to_string(),
        request_id: request.request_id.clone(),
        scope: request.scope,
        target_ref: request.target_ref.clone(),
        width,
        height,
        captured_at_utc: adapter_capture.captured_at_utc.clone(),
        capture_adapter_ref: request.capture_adapter_ref.clone(),
        command_or_api_ref: adapter_capture.command_or_api_ref.clone(),
        flight_recorder_ref: request.flight_recorder_ref.clone(),
        workdir_ref: request.workdir_ref.clone(),
    };
    let metadata_bytes = serde_json::to_vec_pretty(&metadata)?;
    fs::write(&metadata_path, &metadata_bytes)?;
    let metadata_sha256 = sha256_prefixed(&metadata_bytes);

    let artifact = ProductScreenshotArtifactV1 {
        artifact_id: artifact_id.clone(),
        request_id: request.request_id.clone(),
        screenshot_ref: format!("artifact://screenshots/{artifact_id}.png"),
        metadata_ref: format!("artifact://metadata/screenshots/{artifact_id}.json"),
        content_type: "image/png".to_string(),
        width,
        height,
        captured_at_utc: adapter_capture.captured_at_utc,
        retention_class: "visual-validation".to_string(),
        screenshot_path: screenshot_path.to_string_lossy().into_owned(),
        metadata_path: metadata_path.to_string_lossy().into_owned(),
        metadata_schema_id: "hsk.product_screenshot_metadata@1".to_string(),
    };
    let durable_receipt = ProductScreenshotDurableReceiptV1 {
        receipt_id: receipt_id.clone(),
        request_id: request.request_id.clone(),
        scope: request.scope,
        receipt_ref: format!("receipt://product-screenshot-capture/{receipt_id}"),
        receipt_path: receipt_path.to_string_lossy().into_owned(),
        workdir_ref: request.workdir_ref.clone(),
        execution_surface: request.execution_surface,
        records_screenshot_sha256: true,
        records_metadata_sha256: true,
        records_adapter_exit_status: true,
    };
    let proof = ProductScreenshotExecutionProofV1 {
        proof_id: format!("proof.{file_stem}"),
        request_id: request.request_id.clone(),
        adapter_ref: request.capture_adapter_ref.clone(),
        execution_surface: request.execution_surface,
        execution_path: format!("kernel://product-screenshot-capture/{}", request.request_id),
        command_or_api_ref: adapter_capture.command_or_api_ref.clone(),
        workdir_ref: request.workdir_ref.clone(),
        metadata_ref: artifact.metadata_ref.clone(),
        artifact_ref: artifact.screenshot_ref.clone(),
        receipt_ref: durable_receipt.receipt_ref.clone(),
        writes_screenshot_ref: artifact.screenshot_ref.clone(),
        writes_metadata_ref: artifact.metadata_ref.clone(),
        writes_receipt_ref: durable_receipt.receipt_ref.clone(),
    };
    let receipt = ProductScreenshotExecutionReceiptV1 {
        schema_id: "hsk.product_screenshot_execution_receipt@1".to_string(),
        request_id: request.request_id.clone(),
        scope: request.scope,
        command_or_api_ref: adapter_capture.command_or_api_ref,
        artifact_ref: artifact.screenshot_ref.clone(),
        metadata_ref: artifact.metadata_ref.clone(),
        receipt_ref: durable_receipt.receipt_ref.clone(),
        screenshot_path: artifact.screenshot_path.clone(),
        metadata_path: artifact.metadata_path.clone(),
        receipt_path: durable_receipt.receipt_path.clone(),
        screenshot_sha256,
        metadata_sha256,
        adapter_exit_status: adapter_capture.adapter_exit_status,
        workdir_ref: request.workdir_ref.clone(),
    };
    let receipt_bytes = serde_json::to_vec_pretty(&receipt)?;
    fs::write(&receipt_path, receipt_bytes)?;

    Ok(ProductScreenshotExecutionResultV1 {
        artifact,
        durable_receipt,
        proof,
        metadata,
        receipt,
        screenshot_path,
        metadata_path,
        receipt_path,
    })
}

pub fn capture_product_screenshot_from_browser_adapter(
    request: &ProductScreenshotRequestV1,
    adapter_config: ProductScreenshotBrowserAdapterConfigV1,
    artifact_root: impl AsRef<Path>,
) -> Result<ProductScreenshotExecutionResultV1, ProductScreenshotExecutionError> {
    if adapter_config.source_url.trim().is_empty() {
        return Err(ProductScreenshotExecutionError::InvalidRequest(
            "source_url is required",
        ));
    }
    if adapter_config.adapter_script_path.trim().is_empty() {
        return Err(ProductScreenshotExecutionError::InvalidRequest(
            "adapter_script_path is required",
        ));
    }
    if adapter_config.node_binary.trim().is_empty() {
        return Err(ProductScreenshotExecutionError::InvalidRequest(
            "node_binary is required",
        ));
    }

    let artifact_root = artifact_root.as_ref();
    let adapter_output_dir = artifact_root.join("adapter-output");
    fs::create_dir_all(&adapter_output_dir)?;
    let adapter_output_path = adapter_output_dir.join(format!(
        "{}.png",
        sanitize_artifact_segment(&request.request_id)
    ));

    let output = Command::new(adapter_config.node_binary.trim())
        .arg(adapter_config.adapter_script_path.trim())
        .arg("--scope")
        .arg(scope_cli_value(request.scope))
        .arg("--target-ref")
        .arg(request.target_ref.as_str())
        .arg("--source-url")
        .arg(adapter_config.source_url.as_str())
        .arg("--output")
        .arg(adapter_output_path.as_os_str())
        .arg("--width")
        .arg(request.width.to_string())
        .arg("--height")
        .arg(request.height.to_string())
        .output()?;

    if !output.status.success() {
        return Err(ProductScreenshotExecutionError::AdapterFailed {
            status_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    if !adapter_output_path.exists() {
        return Err(ProductScreenshotExecutionError::MissingAdapterOutput(
            adapter_output_path,
        ));
    }

    let png_bytes = fs::read(adapter_output_path)?;
    execute_product_screenshot_capture(
        request,
        ProductScreenshotAdapterCaptureV1 {
            png_bytes,
            adapter_exit_status: output.status.code().unwrap_or(0),
            captured_at_utc: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            command_or_api_ref: adapter_config.command_or_api_ref,
        },
        artifact_root,
    )
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
        require_non_empty(errors, "requests.workdir_ref", &request.workdir_ref);

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
        if request.workdir_ref != "repo-root://" {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "requests.workdir_ref",
                message: "screenshot capture must resolve execution from the repo root",
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
        require_non_empty(
            errors,
            "artifacts.screenshot_path",
            &artifact.screenshot_path,
        );
        require_non_empty(errors, "artifacts.metadata_path", &artifact.metadata_path);
        require_non_empty(
            errors,
            "artifacts.metadata_schema_id",
            &artifact.metadata_schema_id,
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
        if !artifact
            .screenshot_path
            .starts_with("../Handshake_Artifacts/handshake-product/screenshots/")
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "artifacts.screenshot_path",
                message: "screenshot images must be written outside the repo artifact root",
            });
        }
        if !artifact
            .metadata_path
            .starts_with("../Handshake_Artifacts/handshake-product/screenshots/metadata/")
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "artifacts.metadata_path",
                message: "screenshot metadata must be written outside the repo artifact root",
            });
        }
        if artifact.metadata_schema_id != "hsk.product_screenshot_metadata@1" {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "artifacts.metadata_schema_id",
                message: "screenshot metadata must use the governed metadata schema",
            });
        }
    }
}

fn validate_durable_receipts(
    errors: &mut Vec<ProductScreenshotCaptureValidationError>,
    capture: &ProductScreenshotCaptureV1,
) {
    let request_ids: HashSet<&str> = capture
        .requests
        .iter()
        .map(|request| request.request_id.as_str())
        .collect();
    let mut receipt_ids = HashSet::new();

    for receipt in &capture.durable_receipts {
        if !receipt_ids.insert(receipt.receipt_id.as_str()) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "durable_receipts.receipt_id",
                message: "screenshot durable receipt ids must be unique",
            });
        }
        require_non_empty(errors, "durable_receipts.receipt_id", &receipt.receipt_id);
        require_non_empty(errors, "durable_receipts.request_id", &receipt.request_id);
        require_non_empty(errors, "durable_receipts.receipt_ref", &receipt.receipt_ref);
        require_non_empty(
            errors,
            "durable_receipts.receipt_path",
            &receipt.receipt_path,
        );
        require_non_empty(errors, "durable_receipts.workdir_ref", &receipt.workdir_ref);

        if !request_ids.contains(receipt.request_id.as_str()) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "durable_receipts.request_id",
                message: "screenshot durable receipt must link an existing request",
            });
        }
        if !receipt
            .receipt_ref
            .starts_with("receipt://product-screenshot-capture/")
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "durable_receipts.receipt_ref",
                message: "screenshot durable receipt must use the capture receipt namespace",
            });
        }
        if !receipt
            .receipt_path
            .starts_with("../Handshake_Artifacts/handshake-product/screenshots/receipts/")
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "durable_receipts.receipt_path",
                message:
                    "screenshot durable receipts must be written outside the repo artifact root",
            });
        }
        if receipt.workdir_ref != "repo-root://" {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "durable_receipts.workdir_ref",
                message: "screenshot durable receipts must resolve execution from the repo root",
            });
        }
        if !(receipt.records_screenshot_sha256
            && receipt.records_metadata_sha256
            && receipt.records_adapter_exit_status)
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "durable_receipts.records",
                message:
                    "screenshot durable receipts must record artifact hashes and adapter status",
            });
        }
    }

    for request in &capture.requests {
        if !capture.durable_receipts.iter().any(|receipt| {
            receipt.request_id == request.request_id && receipt.scope == request.scope
        }) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "durable_receipts.request_id",
                message: "every screenshot request must have a durable receipt",
            });
        }
    }
}

fn validate_execution_proofs(
    errors: &mut Vec<ProductScreenshotCaptureValidationError>,
    capture: &ProductScreenshotCaptureV1,
) {
    let request_ids: HashSet<&str> = capture
        .requests
        .iter()
        .map(|request| request.request_id.as_str())
        .collect();
    let artifact_refs: HashSet<&str> = capture
        .artifacts
        .iter()
        .map(|artifact| artifact.screenshot_ref.as_str())
        .collect();
    let metadata_refs: HashSet<&str> = capture
        .artifacts
        .iter()
        .map(|artifact| artifact.metadata_ref.as_str())
        .collect();
    let receipt_refs: HashSet<&str> = capture
        .durable_receipts
        .iter()
        .map(|receipt| receipt.receipt_ref.as_str())
        .collect();
    let mut proof_ids = HashSet::new();

    for proof in &capture.execution_proofs {
        if !proof_ids.insert(proof.proof_id.as_str()) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.proof_id",
                message: "screenshot execution proof ids must be unique",
            });
        }
        require_non_empty(errors, "execution_proofs.proof_id", &proof.proof_id);
        require_non_empty(errors, "execution_proofs.request_id", &proof.request_id);
        require_non_empty(errors, "execution_proofs.adapter_ref", &proof.adapter_ref);
        require_non_empty(
            errors,
            "execution_proofs.execution_path",
            &proof.execution_path,
        );
        require_non_empty(
            errors,
            "execution_proofs.command_or_api_ref",
            &proof.command_or_api_ref,
        );
        require_non_empty(errors, "execution_proofs.workdir_ref", &proof.workdir_ref);
        require_non_empty(errors, "execution_proofs.metadata_ref", &proof.metadata_ref);
        require_non_empty(errors, "execution_proofs.artifact_ref", &proof.artifact_ref);
        require_non_empty(errors, "execution_proofs.receipt_ref", &proof.receipt_ref);
        require_non_empty(
            errors,
            "execution_proofs.writes_screenshot_ref",
            &proof.writes_screenshot_ref,
        );
        require_non_empty(
            errors,
            "execution_proofs.writes_metadata_ref",
            &proof.writes_metadata_ref,
        );
        require_non_empty(
            errors,
            "execution_proofs.writes_receipt_ref",
            &proof.writes_receipt_ref,
        );

        if !request_ids.contains(proof.request_id.as_str()) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.request_id",
                message: "screenshot execution proof must link an existing request",
            });
        }
        if !proof.adapter_ref.starts_with("capture-adapter://") {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.adapter_ref",
                message: "screenshot execution proof must use a governed capture adapter",
            });
        }
        if !proof
            .execution_path
            .starts_with("kernel://product-screenshot-capture/")
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.execution_path",
                message:
                    "screenshot execution proof must use a product-owned kernel execution path",
            });
        }
        if !(proof.command_or_api_ref.starts_with("cli://")
            || proof.command_or_api_ref.starts_with("api://"))
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.command_or_api_ref",
                message: "screenshot execution proof must cite a CLI or API execution surface",
            });
        }
        if !uses_governed_capture_surface(proof.execution_surface, &proof.command_or_api_ref) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.command_or_api_ref",
                message:
                    "screenshot execution must use the governed adapter CLI or API capture surface",
            });
        }
        if proof.workdir_ref != "repo-root://" {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.workdir_ref",
                message: "screenshot execution proof must resolve execution from the repo root",
            });
        }
        if !artifact_refs.contains(proof.artifact_ref.as_str()) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.artifact_ref",
                message: "screenshot execution proof must cite a captured screenshot artifact",
            });
        }
        if !metadata_refs.contains(proof.metadata_ref.as_str()) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.metadata_ref",
                message: "screenshot execution proof must cite captured metadata",
            });
        }
        if !receipt_refs.contains(proof.receipt_ref.as_str()) {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.receipt_ref",
                message: "screenshot execution proof must cite a durable capture receipt",
            });
        }
        if !proof
            .receipt_ref
            .starts_with("receipt://product-screenshot-capture/")
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.receipt_ref",
                message: "screenshot execution proof must emit a durable capture receipt",
            });
        }
        if proof.writes_screenshot_ref != proof.artifact_ref {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.writes_screenshot_ref",
                message: "screenshot execution proof must write the cited screenshot artifact",
            });
        }
        if proof.writes_metadata_ref != proof.metadata_ref {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.writes_metadata_ref",
                message: "screenshot execution proof must write the cited metadata artifact",
            });
        }
        if proof.writes_receipt_ref != proof.receipt_ref {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.writes_receipt_ref",
                message: "screenshot execution proof must write the cited durable receipt",
            });
        }
    }

    for request in &capture.requests {
        if !capture
            .execution_proofs
            .iter()
            .any(|proof| proof.request_id == request.request_id)
        {
            errors.push(ProductScreenshotCaptureValidationError {
                field: "execution_proofs.request_id",
                message: "every screenshot request must have execution proof",
            });
        }
    }
}

fn uses_governed_capture_surface(
    execution_surface: ScreenshotCaptureExecutionSurface,
    command_or_api_ref: &str,
) -> bool {
    match execution_surface {
        ScreenshotCaptureExecutionSurface::GovernedAdapterCli => {
            command_or_api_ref.starts_with("cli://handshake screenshot capture")
        }
        ScreenshotCaptureExecutionSurface::GovernedAdapterApi => {
            command_or_api_ref == "api://kernel.product_screenshot_capture.execute"
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

pub fn scope_cli_value(scope: ScreenshotCaptureScope) -> &'static str {
    match scope {
        ScreenshotCaptureScope::FullApp => "full-app",
        ScreenshotCaptureScope::Panel => "panel",
        ScreenshotCaptureScope::Module => "module",
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

fn sanitize_artifact_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect();
    sanitized.trim_matches('-').to_string()
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{}", hex::encode(hasher.finalize()))
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

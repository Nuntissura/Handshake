use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const FOLDED_VISUAL_DEBUGGING_LOOP_STUB_ID: &str = "WP-1-Visual-Debugging-Loop-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisualDebuggingTriggerKind {
    PostCommit,
    PostAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisualComparisonMode {
    PixelDiff,
    StructuralDom,
    /// MT-157: operator-adjudicated comparison. The loop cannot compute a
    /// verdict automatically; a manual review verdict must be recorded
    /// before the diff request can pass or fail.
    Manual,
}

impl VisualComparisonMode {
    pub fn as_token(self) -> &'static str {
        match self {
            VisualComparisonMode::PixelDiff => "pixel_diff",
            VisualComparisonMode::StructuralDom => "structural_dom",
            VisualComparisonMode::Manual => "manual",
        }
    }

    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "pixel_diff" => Some(VisualComparisonMode::PixelDiff),
            "structural_dom" => Some(VisualComparisonMode::StructuralDom),
            "manual" => Some(VisualComparisonMode::Manual),
            _ => None,
        }
    }
}

/// MT-157: outcome of a computed visual comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualDiffOutcome {
    Pass,
    Fail,
    /// Manual comparisons stay in this state until an operator verdict
    /// is recorded against the persisted diff request.
    ManualReviewRequired,
}

impl VisualDiffOutcome {
    pub fn as_token(self) -> &'static str {
        match self {
            VisualDiffOutcome::Pass => "pass",
            VisualDiffOutcome::Fail => "fail",
            VisualDiffOutcome::ManualReviewRequired => "manual_review_required",
        }
    }

    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "pass" => Some(VisualDiffOutcome::Pass),
            "fail" => Some(VisualDiffOutcome::Fail),
            "manual_review_required" => Some(VisualDiffOutcome::ManualReviewRequired),
            _ => None,
        }
    }
}

/// MT-157: computed comparison result fields (not request/config fields).
///
/// `units_compared`/`units_differing` are pixels (bytes of decoded payload)
/// for [`VisualComparisonMode::PixelDiff`] and DOM snapshot nodes for
/// [`VisualComparisonMode::StructuralDom`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualDiffComputationV1 {
    pub comparison_mode: VisualComparisonMode,
    pub units_compared: u64,
    pub units_differing: u64,
    pub mismatch_basis_points: u32,
    pub threshold_exceeded: bool,
    pub outcome: VisualDiffOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualDiffComputeError {
    pub message: String,
}

impl std::fmt::Display for VisualDiffComputeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "visual diff computation failed: {}",
            self.message
        )
    }
}

impl std::error::Error for VisualDiffComputeError {}

/// MT-157: compute a real comparison result from reference + candidate
/// payload bytes against the configured thresholds.
///
/// - `PixelDiff` compares the decoded payload byte-for-byte and reports the
///   differing fraction in basis points against `max_pixel_diff_basis_points`.
/// - `StructuralDom` parses both payloads as JSON DOM snapshots and reports
///   mismatched nodes against `structural_mismatch_limit`.
/// - `Manual` performs no automatic computation: the result is
///   [`VisualDiffOutcome::ManualReviewRequired`] until an operator verdict
///   is recorded.
pub fn compute_visual_comparison(
    mode: VisualComparisonMode,
    reference_bytes: &[u8],
    candidate_bytes: &[u8],
    threshold_config: &VisualDebuggingThresholdConfigV1,
) -> Result<VisualDiffComputationV1, VisualDiffComputeError> {
    match mode {
        VisualComparisonMode::Manual => Ok(VisualDiffComputationV1 {
            comparison_mode: mode,
            units_compared: 0,
            units_differing: 0,
            mismatch_basis_points: 0,
            threshold_exceeded: false,
            outcome: VisualDiffOutcome::ManualReviewRequired,
        }),
        VisualComparisonMode::PixelDiff => {
            if reference_bytes.is_empty() || candidate_bytes.is_empty() {
                return Err(VisualDiffComputeError {
                    message: "pixel comparison requires non-empty reference and candidate payloads"
                        .to_string(),
                });
            }
            let compared = reference_bytes.len().max(candidate_bytes.len()) as u64;
            let overlap_differing = reference_bytes
                .iter()
                .zip(candidate_bytes.iter())
                .filter(|(reference, candidate)| reference != candidate)
                .count() as u64;
            let length_differing = reference_bytes.len().abs_diff(candidate_bytes.len()) as u64;
            let differing = overlap_differing + length_differing;
            let mismatch_basis_points = mismatch_basis_points(differing, compared);
            let threshold_exceeded =
                mismatch_basis_points > threshold_config.max_pixel_diff_basis_points;
            Ok(VisualDiffComputationV1 {
                comparison_mode: mode,
                units_compared: compared,
                units_differing: differing,
                mismatch_basis_points,
                threshold_exceeded,
                outcome: if threshold_exceeded {
                    VisualDiffOutcome::Fail
                } else {
                    VisualDiffOutcome::Pass
                },
            })
        }
        VisualComparisonMode::StructuralDom => {
            let reference: serde_json::Value =
                serde_json::from_slice(reference_bytes).map_err(|err| VisualDiffComputeError {
                    message: format!("reference DOM snapshot is not valid JSON: {err}"),
                })?;
            let candidate: serde_json::Value =
                serde_json::from_slice(candidate_bytes).map_err(|err| VisualDiffComputeError {
                    message: format!("candidate DOM snapshot is not valid JSON: {err}"),
                })?;
            let (compared, mismatched) = structural_node_diff(&reference, &candidate);
            if compared == 0 {
                return Err(VisualDiffComputeError {
                    message: "structural comparison requires at least one DOM node".to_string(),
                });
            }
            let mismatch_basis_points = mismatch_basis_points(mismatched, compared);
            let threshold_exceeded =
                mismatched > u64::from(threshold_config.structural_mismatch_limit);
            Ok(VisualDiffComputationV1 {
                comparison_mode: mode,
                units_compared: compared,
                units_differing: mismatched,
                mismatch_basis_points,
                threshold_exceeded,
                outcome: if threshold_exceeded {
                    VisualDiffOutcome::Fail
                } else {
                    VisualDiffOutcome::Pass
                },
            })
        }
    }
}

fn mismatch_basis_points(differing: u64, compared: u64) -> u32 {
    if compared == 0 {
        return 0;
    }
    // Ceiling division so a single differing unit never rounds to zero.
    (differing.saturating_mul(10_000).div_ceil(compared)).min(10_000) as u32
}

fn structural_node_diff(
    reference: &serde_json::Value,
    candidate: &serde_json::Value,
) -> (u64, u64) {
    use serde_json::Value;
    match (reference, candidate) {
        (Value::Object(reference_map), Value::Object(candidate_map)) => {
            let mut compared = 1u64;
            let mut mismatched = 0u64;
            let mut keys: Vec<&String> = reference_map.keys().collect();
            for key in candidate_map.keys() {
                if !reference_map.contains_key(key) {
                    keys.push(key);
                }
            }
            for key in keys {
                match (reference_map.get(key), candidate_map.get(key)) {
                    (Some(reference_child), Some(candidate_child)) => {
                        let (child_compared, child_mismatched) =
                            structural_node_diff(reference_child, candidate_child);
                        compared += child_compared;
                        mismatched += child_mismatched;
                    }
                    (Some(only), None) | (None, Some(only)) => {
                        let missing = count_structural_nodes(only);
                        compared += missing;
                        mismatched += missing;
                    }
                    (None, None) => {}
                }
            }
            (compared, mismatched)
        }
        (Value::Array(reference_items), Value::Array(candidate_items)) => {
            let mut compared = 1u64;
            let mut mismatched = 0u64;
            let shared = reference_items.len().min(candidate_items.len());
            for index in 0..shared {
                let (child_compared, child_mismatched) =
                    structural_node_diff(&reference_items[index], &candidate_items[index]);
                compared += child_compared;
                mismatched += child_mismatched;
            }
            for extra in reference_items
                .iter()
                .skip(shared)
                .chain(candidate_items.iter().skip(shared))
            {
                let missing = count_structural_nodes(extra);
                compared += missing;
                mismatched += missing;
            }
            (compared, mismatched)
        }
        (reference_leaf, candidate_leaf)
            if std::mem::discriminant(reference_leaf) == std::mem::discriminant(candidate_leaf) =>
        {
            (1, u64::from(reference_leaf != candidate_leaf))
        }
        (reference_node, candidate_node) => {
            let nodes =
                count_structural_nodes(reference_node).max(count_structural_nodes(candidate_node));
            (nodes, nodes)
        }
    }
}

fn count_structural_nodes(value: &serde_json::Value) -> u64 {
    use serde_json::Value;
    match value {
        Value::Object(map) => 1 + map.values().map(count_structural_nodes).sum::<u64>(),
        Value::Array(items) => 1 + items.iter().map(count_structural_nodes).sum::<u64>(),
        _ => 1,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualDebuggingTriggerV1 {
    pub trigger_id: String,
    pub kind: VisualDebuggingTriggerKind,
    pub screenshot_request_ref: String,
    pub baseline_ref: String,
    pub capture_after_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualDebuggingThresholdConfigV1 {
    pub threshold_config_ref: String,
    pub max_pixel_diff_basis_points: u32,
    pub max_layout_shift_basis_points: u32,
    pub structural_mismatch_limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualDebugEvidenceArtifactV1 {
    pub evidence_id: String,
    pub wp_id: String,
    pub commit_ref: String,
    pub screenshot_ref: String,
    pub baseline_ref: String,
    pub visual_diff_artifact_ref: String,
    pub comparison_mode: VisualComparisonMode,
    pub mismatch_basis_points: u32,
    pub stored_in_artifact_system: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorSteeringV1 {
    pub enabled: bool,
    pub target_role: String,
    pub receipt_kind: String,
    pub code_diff_ref: String,
    pub visual_diff_ref: String,
    pub visual_evidence_required: bool,
    pub threshold_exceeded_sends_steer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualDebuggingLoopV1 {
    pub schema_id: String,
    pub loop_id: String,
    pub folded_stub_ids: Vec<String>,
    pub gui_bearing_wp_id: String,
    pub triggers: Vec<VisualDebuggingTriggerV1>,
    pub threshold_config: VisualDebuggingThresholdConfigV1,
    pub evidence_artifacts: Vec<VisualDebugEvidenceArtifactV1>,
    pub validator_steering: ValidatorSteeringV1,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualDebuggingLoopProjectionV1 {
    pub schema_id: String,
    pub loop_id: String,
    pub trigger_kinds: Vec<VisualDebuggingTriggerKind>,
    pub visual_diff_artifact_refs: Vec<String>,
    pub threshold_exceeded_evidence_ids: Vec<String>,
    pub threshold_config_ref: String,
    pub validator_steer_required: bool,
    pub mutates_gui_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualDebuggingLoopValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_visual_debugging_loop(
    loop_config: &VisualDebuggingLoopV1,
) -> Result<(), Vec<VisualDebuggingLoopValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &loop_config.schema_id);
    require_non_empty(&mut errors, "loop_id", &loop_config.loop_id);
    require_vec(&mut errors, "folded_stub_ids", &loop_config.folded_stub_ids);
    require_non_empty(
        &mut errors,
        "gui_bearing_wp_id",
        &loop_config.gui_bearing_wp_id,
    );
    require_vec(&mut errors, "triggers", &loop_config.triggers);
    require_vec(
        &mut errors,
        "evidence_artifacts",
        &loop_config.evidence_artifacts,
    );
    require_vec(
        &mut errors,
        "product_authority_refs",
        &loop_config.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &loop_config.folded_source_refs,
    );

    if !contains_exact(
        &loop_config.folded_stub_ids,
        FOLDED_VISUAL_DEBUGGING_LOOP_STUB_ID,
    ) {
        errors.push(VisualDebuggingLoopValidationError {
            field: "folded_stub_ids",
            message: "visual debugging loop must preserve the folded stub id",
        });
    }
    if !contains_text(
        &loop_config.folded_source_refs,
        FOLDED_VISUAL_DEBUGGING_LOOP_STUB_ID,
    ) {
        errors.push(VisualDebuggingLoopValidationError {
            field: "folded_source_refs",
            message: "visual debugging loop must preserve the folded source reference",
        });
    }

    validate_authority_refs(&mut errors, loop_config);
    validate_triggers(&mut errors, loop_config);
    validate_thresholds(&mut errors, &loop_config.threshold_config);
    validate_evidence(&mut errors, loop_config);
    validate_steering(&mut errors, loop_config);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_visual_debugging_loop(
    loop_config: &VisualDebuggingLoopV1,
) -> Result<VisualDebuggingLoopProjectionV1, Vec<VisualDebuggingLoopValidationError>> {
    validate_visual_debugging_loop(loop_config)?;

    Ok(VisualDebuggingLoopProjectionV1 {
        schema_id: "hsk.kernel.visual_debugging_loop_projection@1".to_string(),
        loop_id: loop_config.loop_id.clone(),
        trigger_kinds: ordered_trigger_kinds(loop_config),
        visual_diff_artifact_refs: loop_config
            .evidence_artifacts
            .iter()
            .map(|artifact| artifact.visual_diff_artifact_ref.clone())
            .collect(),
        threshold_exceeded_evidence_ids: threshold_exceeded_evidence_ids(loop_config),
        threshold_config_ref: loop_config.threshold_config.threshold_config_ref.clone(),
        validator_steer_required: validator_steer_required(loop_config),
        mutates_gui_authority: false,
    })
}

fn validate_authority_refs(
    errors: &mut Vec<VisualDebuggingLoopValidationError>,
    loop_config: &VisualDebuggingLoopV1,
) {
    for required_ref in [
        "kernel.product_screenshot_capture",
        "kernel.action_catalog",
        "artifact_store.visual_evidence",
        "validator.steering",
    ] {
        if !contains_exact(&loop_config.product_authority_refs, required_ref) {
            errors.push(VisualDebuggingLoopValidationError {
                field: "product_authority_refs",
                message: "visual debugging loop must cite screenshot capture, catalog, artifact store, and validator steering authorities",
            });
        }
    }
}

fn validate_triggers(
    errors: &mut Vec<VisualDebuggingLoopValidationError>,
    loop_config: &VisualDebuggingLoopV1,
) {
    let mut trigger_ids = HashSet::new();
    for trigger in &loop_config.triggers {
        if !trigger_ids.insert(trigger.trigger_id.as_str()) {
            errors.push(VisualDebuggingLoopValidationError {
                field: "triggers.trigger_id",
                message: "visual debugging trigger ids must be unique",
            });
        }
        require_non_empty(errors, "triggers.trigger_id", &trigger.trigger_id);
        require_non_empty(
            errors,
            "triggers.screenshot_request_ref",
            &trigger.screenshot_request_ref,
        );
        require_non_empty(errors, "triggers.baseline_ref", &trigger.baseline_ref);
        require_non_empty(
            errors,
            "triggers.capture_after_ref",
            &trigger.capture_after_ref,
        );
        if !trigger
            .screenshot_request_ref
            .starts_with("screenshot-request://")
        {
            errors.push(VisualDebuggingLoopValidationError {
                field: "triggers.screenshot_request_ref",
                message: "visual debugging triggers must bind screenshot request refs",
            });
        }
        if !trigger.baseline_ref.starts_with("artifact://baselines/") {
            errors.push(VisualDebuggingLoopValidationError {
                field: "triggers.baseline_ref",
                message: "visual debugging triggers must bind baseline artifact refs",
            });
        }
        if !trigger.capture_after_ref.starts_with("event://") {
            errors.push(VisualDebuggingLoopValidationError {
                field: "triggers.capture_after_ref",
                message:
                    "visual debugging triggers must bind post-action or post-commit event refs",
            });
        }
    }

    for required_kind in [
        VisualDebuggingTriggerKind::PostCommit,
        VisualDebuggingTriggerKind::PostAction,
    ] {
        if !loop_config
            .triggers
            .iter()
            .any(|trigger| trigger.kind == required_kind)
        {
            errors.push(VisualDebuggingLoopValidationError {
                field: "triggers.kind",
                message: "visual debugging loop must include post-commit and post-action triggers",
            });
        }
    }
}

fn validate_thresholds(
    errors: &mut Vec<VisualDebuggingLoopValidationError>,
    threshold: &VisualDebuggingThresholdConfigV1,
) {
    require_non_empty(
        errors,
        "threshold_config.threshold_config_ref",
        &threshold.threshold_config_ref,
    );
    if !threshold.threshold_config_ref.starts_with("packet://") {
        errors.push(VisualDebuggingLoopValidationError {
            field: "threshold_config.threshold_config_ref",
            message: "visual debugging thresholds must be configurable from the task packet or refinement",
        });
    }
    if threshold.max_pixel_diff_basis_points == 0 {
        errors.push(VisualDebuggingLoopValidationError {
            field: "threshold_config.max_pixel_diff_basis_points",
            message: "visual debugging pixel threshold must be positive",
        });
    }
    if threshold.max_layout_shift_basis_points == 0 {
        errors.push(VisualDebuggingLoopValidationError {
            field: "threshold_config.max_layout_shift_basis_points",
            message: "visual debugging layout threshold must be positive",
        });
    }
}

fn validate_evidence(
    errors: &mut Vec<VisualDebuggingLoopValidationError>,
    loop_config: &VisualDebuggingLoopV1,
) {
    let mut evidence_ids = HashSet::new();
    for evidence in &loop_config.evidence_artifacts {
        if !evidence_ids.insert(evidence.evidence_id.as_str()) {
            errors.push(VisualDebuggingLoopValidationError {
                field: "evidence_artifacts.evidence_id",
                message: "visual evidence ids must be unique",
            });
        }
        require_non_empty(
            errors,
            "evidence_artifacts.evidence_id",
            &evidence.evidence_id,
        );
        require_non_empty(errors, "evidence_artifacts.wp_id", &evidence.wp_id);
        require_non_empty(
            errors,
            "evidence_artifacts.commit_ref",
            &evidence.commit_ref,
        );
        require_non_empty(
            errors,
            "evidence_artifacts.screenshot_ref",
            &evidence.screenshot_ref,
        );
        require_non_empty(
            errors,
            "evidence_artifacts.baseline_ref",
            &evidence.baseline_ref,
        );
        require_non_empty(
            errors,
            "evidence_artifacts.visual_diff_artifact_ref",
            &evidence.visual_diff_artifact_ref,
        );
        if !evidence
            .screenshot_ref
            .starts_with("artifact://screenshots/")
        {
            errors.push(VisualDebuggingLoopValidationError {
                field: "evidence_artifacts.screenshot_ref",
                message: "visual evidence must reference screenshot artifacts",
            });
        }
        if !evidence.commit_ref.starts_with("git://commit/") {
            errors.push(VisualDebuggingLoopValidationError {
                field: "evidence_artifacts.commit_ref",
                message: "visual evidence must carry commit metadata",
            });
        }
        if evidence.wp_id != loop_config.gui_bearing_wp_id {
            errors.push(VisualDebuggingLoopValidationError {
                field: "evidence_artifacts.wp_id",
                message: "visual evidence must belong to the GUI-bearing work packet",
            });
        }
        if !evidence.baseline_ref.starts_with("artifact://baselines/") {
            errors.push(VisualDebuggingLoopValidationError {
                field: "evidence_artifacts.baseline_ref",
                message: "visual evidence must reference baseline artifacts",
            });
        }
        if !evidence
            .visual_diff_artifact_ref
            .starts_with("artifact://visual-diffs/")
        {
            errors.push(VisualDebuggingLoopValidationError {
                field: "evidence_artifacts.visual_diff_artifact_ref",
                message: "visual evidence must reference visual diff artifacts",
            });
        }
        if !evidence.stored_in_artifact_system {
            errors.push(VisualDebuggingLoopValidationError {
                field: "evidence_artifacts.stored_in_artifact_system",
                message: "visual evidence must be stored in the artifact system",
            });
        }
    }
}

fn validate_steering(
    errors: &mut Vec<VisualDebuggingLoopValidationError>,
    loop_config: &VisualDebuggingLoopV1,
) {
    let steering = &loop_config.validator_steering;

    if !steering.enabled {
        errors.push(VisualDebuggingLoopValidationError {
            field: "validator_steering.enabled",
            message: "visual debugging loop must enable validator steering",
        });
    }
    require_non_empty(
        errors,
        "validator_steering.target_role",
        &steering.target_role,
    );
    require_non_empty(
        errors,
        "validator_steering.receipt_kind",
        &steering.receipt_kind,
    );
    require_non_empty(
        errors,
        "validator_steering.code_diff_ref",
        &steering.code_diff_ref,
    );
    require_non_empty(
        errors,
        "validator_steering.visual_diff_ref",
        &steering.visual_diff_ref,
    );
    if steering.target_role != "VALIDATOR" {
        errors.push(VisualDebuggingLoopValidationError {
            field: "validator_steering.target_role",
            message: "visual debugging loop must steer validator review",
        });
    }
    if steering.receipt_kind != "STEER" {
        errors.push(VisualDebuggingLoopValidationError {
            field: "validator_steering.receipt_kind",
            message: "visual debugging loop must route threshold failures as STEER receipts",
        });
    }
    if !steering.code_diff_ref.starts_with("git://diff/") {
        errors.push(VisualDebuggingLoopValidationError {
            field: "validator_steering.code_diff_ref",
            message: "validator steering must carry a code diff ref",
        });
    }
    if !steering
        .visual_diff_ref
        .starts_with("artifact://visual-diffs/")
    {
        errors.push(VisualDebuggingLoopValidationError {
            field: "validator_steering.visual_diff_ref",
            message: "validator steering must carry a visual diff artifact ref",
        });
    }
    if !steering.visual_evidence_required {
        errors.push(VisualDebuggingLoopValidationError {
            field: "validator_steering.visual_evidence_required",
            message: "validator steering must include visual evidence",
        });
    }
    if threshold_exceeded(loop_config) && !steering.threshold_exceeded_sends_steer {
        errors.push(VisualDebuggingLoopValidationError {
            field: "validator_steering.threshold_exceeded_sends_steer",
            message: "threshold-exceeded visual mismatches must send STEER",
        });
    }
    if threshold_exceeded(loop_config)
        && !loop_config
            .evidence_artifacts
            .iter()
            .any(|evidence| evidence.visual_diff_artifact_ref == steering.visual_diff_ref)
    {
        errors.push(VisualDebuggingLoopValidationError {
            field: "validator_steering.visual_diff_ref",
            message: "validator steering visual diff must match stored visual evidence",
        });
    }
}

fn ordered_trigger_kinds(loop_config: &VisualDebuggingLoopV1) -> Vec<VisualDebuggingTriggerKind> {
    [
        VisualDebuggingTriggerKind::PostCommit,
        VisualDebuggingTriggerKind::PostAction,
    ]
    .into_iter()
    .filter(|kind| {
        loop_config
            .triggers
            .iter()
            .any(|trigger| trigger.kind == *kind)
    })
    .collect()
}

fn threshold_exceeded_evidence_ids(loop_config: &VisualDebuggingLoopV1) -> Vec<String> {
    loop_config
        .evidence_artifacts
        .iter()
        .filter(|artifact| {
            artifact.mismatch_basis_points
                > loop_config.threshold_config.max_pixel_diff_basis_points
        })
        .map(|artifact| artifact.evidence_id.clone())
        .collect()
}

fn threshold_exceeded(loop_config: &VisualDebuggingLoopV1) -> bool {
    !threshold_exceeded_evidence_ids(loop_config).is_empty()
}

fn validator_steer_required(loop_config: &VisualDebuggingLoopV1) -> bool {
    loop_config.validator_steering.enabled
        && loop_config.validator_steering.visual_evidence_required
        && loop_config
            .validator_steering
            .threshold_exceeded_sends_steer
        && threshold_exceeded(loop_config)
}

fn require_non_empty(
    errors: &mut Vec<VisualDebuggingLoopValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(VisualDebuggingLoopValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<VisualDebuggingLoopValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(VisualDebuggingLoopValidationError {
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

use serde::{Deserialize, Serialize};

use super::crdt::persistence::sha256_hex;
use std::collections::HashSet;

pub const CODER_HANDOFF_VALIDATION_REQUEST_SCHEMA_ID: &str =
    "hsk.kernel.coder_handoff_validation_request@1";
pub const CODER_HANDOFF_VALIDATION_REQUEST_PROJECTION_SCHEMA_ID: &str =
    "hsk.kernel.coder_handoff_validation_request_projection@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoderHandoffArtifactKind {
    SourceFile,
    TestFile,
    ProofHarnessFile,
    GeneratedProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoderHandoffTestOutcome {
    Passed,
    BlockedByKnownExternalDrift,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnownBlockerDisposition {
    NoneObserved,
    ExternalUnrelatedDrift,
    RequiresValidatorDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoderHandoffFailureState {
    MissingMicrotaskIdentity,
    MissingActorSession,
    ScopeOutsideAllowedPaths,
    MissingTouchedArtifacts,
    MissingTouchedActions,
    MissingReceipts,
    MissingTests,
    MissingEvidence,
    MissingKnownBlockerRecord,
    MissingRequestedReview,
    ManualStatusEditAttempt,
    ReviewRequestWouldMutateStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderHandoffActorV1 {
    pub actor_role: String,
    pub actor_session: String,
    pub work_profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimedMicrotaskScopeV1 {
    pub wp_id: String,
    pub mt_id: String,
    pub microtask_contract_ref: String,
    pub allowed_paths: Vec<String>,
    pub dependency_refs: Vec<String>,
    pub claim_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderTouchedArtifactV1 {
    pub path: String,
    pub kind: CoderHandoffArtifactKind,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderTouchedActionV1 {
    pub action_id: String,
    pub authority_effect: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderHandoffReceiptRefV1 {
    pub receipt_kind: String,
    pub receipt_ref: String,
    pub correlation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderHandoffTestEvidenceV1 {
    pub command: String,
    pub outcome: CoderHandoffTestOutcome,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderKnownBlockerV1 {
    pub blocker_id: String,
    pub disposition: KnownBlockerDisposition,
    pub summary: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestedValidationReviewV1 {
    pub target_role: String,
    pub target_session_ref: String,
    pub receipt_kind: String,
    pub named_verb: String,
    pub review_mode: String,
    pub microtask_json_schema_id: String,
    pub correlation_id: String,
    pub status_edit_allowed: bool,
    pub generated_without_model_status_edit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderHandoffResearchBasisV1 {
    pub source_ref: String,
    pub pattern_found: String,
    pub selected_reuse: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderHandoffValidationRequestContractV1 {
    pub schema_id: String,
    pub handoff_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub actor: CoderHandoffActorV1,
    pub claimed_scope: ClaimedMicrotaskScopeV1,
    pub touched_artifacts: Vec<CoderTouchedArtifactV1>,
    pub touched_actions: Vec<CoderTouchedActionV1>,
    pub receipt_refs: Vec<CoderHandoffReceiptRefV1>,
    pub tests: Vec<CoderHandoffTestEvidenceV1>,
    pub evidence_refs: Vec<String>,
    pub known_blockers: Vec<CoderKnownBlockerV1>,
    pub requested_review: RequestedValidationReviewV1,
    pub status_fields_mutated_by_model: bool,
    pub failure_states: Vec<CoderHandoffFailureState>,
    pub research_basis_refs: Vec<CoderHandoffResearchBasisV1>,
    pub product_authority_refs: Vec<String>,
}

pub type CoderHandoffContractV1 = CoderHandoffValidationRequestContractV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderHandoffValidationRequestProjectionV1 {
    pub schema_id: String,
    pub source_handoff_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub review_request_ready: bool,
    pub review_request_receipt_kind: String,
    pub target_role: String,
    pub microtask_json_refs: Vec<String>,
    pub file_targets: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub known_blocker_refs: Vec<String>,
    pub status_mutation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderHandoffValidationRequestError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_kernel002_coder_handoff_validation_request() -> CoderHandoffValidationRequestContractV1
{
    let wp_id = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";
    let mt_id = "MT-056";
    let correlation_id = "MT-056";

    CoderHandoffValidationRequestContractV1 {
        schema_id: CODER_HANDOFF_VALIDATION_REQUEST_SCHEMA_ID.to_string(),
        handoff_id: "kernel002-coder-handoff-validation-request-mt056".to_string(),
        wp_id: wp_id.to_string(),
        mt_id: mt_id.to_string(),
        actor: CoderHandoffActorV1 {
            actor_role: "CODER".to_string(),
            actor_session: "KERNEL_BUILDER-20260514-130219".to_string(),
            work_profile_id: "kernel-builder-elevated-coder".to_string(),
        },
        claimed_scope: ClaimedMicrotaskScopeV1 {
            wp_id: wp_id.to_string(),
            mt_id: mt_id.to_string(),
            microtask_contract_ref: format!(".GOV/task_packets/{wp_id}/{mt_id}.json"),
            allowed_paths: vec![
                "src/backend/handshake_core/src/locus/**".to_string(),
                "src/backend/handshake_core/src/kernel/**".to_string(),
                "src/backend/handshake_core/tests/**".to_string(),
            ],
            dependency_refs: vec!["MT-055".to_string()],
            claim_receipt_refs: vec![
                "mt-board://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-056/CLAIM"
                    .to_string(),
            ],
        },
        touched_artifacts: vec![
            artifact(
                "src/backend/handshake_core/src/kernel/coder_handoff_validation_request.rs",
                CoderHandoffArtifactKind::SourceFile,
            ),
            artifact(
                "src/backend/handshake_core/tests/kernel_coder_handoff_validation_request_tests.rs",
                CoderHandoffArtifactKind::TestFile,
            ),
            artifact(
                "src/backend/handshake_core/src/kernel/action_catalog.rs",
                CoderHandoffArtifactKind::SourceFile,
            ),
        ],
        touched_actions: vec![CoderTouchedActionV1 {
            action_id: "kernel.coder_handoff_validation_request.project".to_string(),
            authority_effect: "ProjectionOnly".to_string(),
            evidence_ref: "action-catalog://kernel.coder_handoff_validation_request.project"
                .to_string(),
        }],
        receipt_refs: vec![
            CoderHandoffReceiptRefV1 {
                receipt_kind: "STATUS".to_string(),
                receipt_ref: "receipt://MT-056/status".to_string(),
                correlation_id: correlation_id.to_string(),
            },
            CoderHandoffReceiptRefV1 {
                receipt_kind: "REVIEW_REQUEST".to_string(),
                receipt_ref: "receipt://MT-056/review-request/projection".to_string(),
                correlation_id: correlation_id.to_string(),
            },
        ],
        tests: vec![
            test_evidence(
                "cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test kernel_coder_handoff_validation_request_tests",
                CoderHandoffTestOutcome::Passed,
            ),
            test_evidence(
                "cargo test --manifest-path Cargo.toml --test coder_handoff_validation_request_harness",
                CoderHandoffTestOutcome::Passed,
            ),
            test_evidence(
                "cargo test --test micro_task_executor_tests --no-run",
                CoderHandoffTestOutcome::BlockedByKnownExternalDrift,
            ),
        ],
        evidence_refs: vec![
            "test://kernel_coder_handoff_validation_request_tests".to_string(),
            "test://coder_handoff_validation_request_harness".to_string(),
            "blocked-test://micro_task_executor_tests/closeout_badge-signature-drift"
                .to_string(),
        ],
        known_blockers: vec![CoderKnownBlockerV1 {
            blocker_id: "blocked-test-micro_task_executor_tests-closeout-badge".to_string(),
            disposition: KnownBlockerDisposition::ExternalUnrelatedDrift,
            summary: "Broad product compile remains blocked by unrelated closeout_badge and build_software_delivery_overlay_triage_row signature drift.".to_string(),
            evidence_ref: "blocked-test://micro_task_executor_tests/closeout_badge-signature-drift".to_string(),
        }],
        requested_review: RequestedValidationReviewV1 {
            target_role: "WP_VALIDATOR".to_string(),
            target_session_ref: "role-session://WP_VALIDATOR/current".to_string(),
            receipt_kind: "REVIEW_REQUEST".to_string(),
            named_verb: "MT_HANDOFF".to_string(),
            review_mode: "OVERLAP".to_string(),
            microtask_json_schema_id: "hsk.microtask_review_request@1".to_string(),
            correlation_id: correlation_id.to_string(),
            status_edit_allowed: false,
            generated_without_model_status_edit: true,
        },
        status_fields_mutated_by_model: false,
        failure_states: required_failure_states(),
        research_basis_refs: vec![
            research(
                "https://docs.github.com/en/rest/pulls/review-requests",
                "Review requests are explicit API resources that notify requested reviewers.",
                "Model coder handoff should generate a typed review request instead of relying on prose status.",
            ),
            research(
                "https://docs.gitlab.com/api/merge_request_approvals/",
                "Approval state distinguishes approvers from whether approval rules are satisfied.",
                "Record evidence and requested review separately from validator verdict truth.",
            ),
            research(
                "https://gerrit-review.googlesource.com/Documentation/config-submit-requirements.html",
                "Submit requirements bind review labels, vetoes, and non-uploader/self-approval constraints.",
                "Handoff records should keep actor/session, requested reviewer, and review rule posture explicit.",
            ),
            research(
                "https://kubernetes.io/docs/concepts/workloads/pods/pod-condition/",
                "Conditions carry type, status, reason, timestamps, message, and observedGeneration.",
                "Validation request projections should use typed status evidence fields rather than manual status edits.",
            ),
        ],
        product_authority_refs: vec![
            "kernel.action_catalog".to_string(),
            "kernel.local_model_microtask_loop".to_string(),
            "kernel.task_contract_lifecycle".to_string(),
            "kernel.generated_documentation_status_projection".to_string(),
            "kernel.direct_edit_guard".to_string(),
        ],
    }
}

pub fn validate_coder_handoff_validation_request(
    contract: &CoderHandoffValidationRequestContractV1,
) -> Result<(), Vec<CoderHandoffValidationRequestError>> {
    let mut errors = Vec::new();

    if contract.schema_id != CODER_HANDOFF_VALIDATION_REQUEST_SCHEMA_ID {
        errors.push(error(
            "schema_id",
            "coder handoff validation request schema id is required",
        ));
    }
    require_non_empty(&mut errors, "handoff_id", &contract.handoff_id);
    require_non_empty(&mut errors, "wp_id", &contract.wp_id);
    require_non_empty(&mut errors, "mt_id", &contract.mt_id);
    validate_actor(&mut errors, &contract.actor);
    validate_claimed_scope(&mut errors, contract);
    validate_touched_artifacts(&mut errors, contract);
    validate_touched_actions(&mut errors, &contract.touched_actions);
    validate_receipts(&mut errors, &contract.receipt_refs);
    validate_tests(&mut errors, &contract.tests);
    require_vec(&mut errors, "evidence_refs", &contract.evidence_refs);
    validate_known_blockers(&mut errors, &contract.known_blockers);
    validate_requested_review(&mut errors, &contract.requested_review);
    validate_status_boundary(&mut errors, contract);
    validate_failure_states(&mut errors, &contract.failure_states);
    validate_research_basis(&mut errors, &contract.research_basis_refs);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_coder_handoff_validation_request(
    contract: &CoderHandoffValidationRequestContractV1,
) -> Result<CoderHandoffValidationRequestProjectionV1, Vec<CoderHandoffValidationRequestError>> {
    validate_coder_handoff_validation_request(contract)?;

    Ok(CoderHandoffValidationRequestProjectionV1 {
        schema_id: CODER_HANDOFF_VALIDATION_REQUEST_PROJECTION_SCHEMA_ID.to_string(),
        source_handoff_id: contract.handoff_id.clone(),
        wp_id: contract.wp_id.clone(),
        mt_id: contract.mt_id.clone(),
        review_request_ready: true,
        review_request_receipt_kind: contract.requested_review.receipt_kind.clone(),
        target_role: contract.requested_review.target_role.clone(),
        microtask_json_refs: vec![contract.claimed_scope.microtask_contract_ref.clone()],
        file_targets: contract
            .touched_artifacts
            .iter()
            .map(|artifact| artifact.path.clone())
            .collect(),
        evidence_refs: contract.evidence_refs.clone(),
        known_blocker_refs: contract
            .known_blockers
            .iter()
            .map(|blocker| blocker.blocker_id.clone())
            .collect(),
        status_mutation_allowed: false,
    })
}

fn validate_actor(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    actor: &CoderHandoffActorV1,
) {
    if actor.actor_role != "CODER" {
        errors.push(error(
            "actor.actor_role",
            "handoff actor role must be CODER",
        ));
    }
    require_non_empty(errors, "actor.actor_session", &actor.actor_session);
    require_non_empty(errors, "actor.work_profile_id", &actor.work_profile_id);
}

fn validate_claimed_scope(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    contract: &CoderHandoffValidationRequestContractV1,
) {
    if contract.wp_id != contract.claimed_scope.wp_id {
        errors.push(error(
            "claimed_scope.wp_id",
            "claimed scope must match handoff wp_id",
        ));
    }
    if contract.mt_id != contract.claimed_scope.mt_id {
        errors.push(error(
            "claimed_scope.mt_id",
            "claimed scope must match handoff mt_id",
        ));
    }
    if contract.wp_id.is_empty() || contract.mt_id.is_empty() {
        errors.push(error(
            "mt_id",
            "work packet and microtask identity are required",
        ));
    }
    require_non_empty(
        errors,
        "claimed_scope.microtask_contract_ref",
        &contract.claimed_scope.microtask_contract_ref,
    );
    require_vec(
        errors,
        "claimed_scope.allowed_paths",
        &contract.claimed_scope.allowed_paths,
    );
    require_vec(
        errors,
        "claimed_scope.claim_receipt_refs",
        &contract.claimed_scope.claim_receipt_refs,
    );
}

fn validate_touched_artifacts(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    contract: &CoderHandoffValidationRequestContractV1,
) {
    require_vec(errors, "touched_artifacts", &contract.touched_artifacts);
    let allowed_paths = &contract.claimed_scope.allowed_paths;
    let mut seen = HashSet::new();

    for artifact in &contract.touched_artifacts {
        require_non_empty(errors, "touched_artifacts.path", &artifact.path);
        require_non_empty(
            errors,
            "touched_artifacts.source_hash",
            &artifact.source_hash,
        );
        if !is_sha256_digest(&artifact.source_hash) {
            errors.push(error(
                "touched_artifacts.source_hash",
                "touched artifact source hashes must be sha256 digests",
            ));
        }
        if !seen.insert(artifact.path.as_str()) {
            errors.push(error(
                "touched_artifacts.path",
                "touched artifact paths must be unique",
            ));
        }
        if !path_allowed_by_any(&artifact.path, allowed_paths) {
            errors.push(error(
                "touched_artifacts.path",
                "touched artifact must stay inside claimed MT allowed paths",
            ));
        }
    }
}

fn validate_touched_actions(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    actions: &[CoderTouchedActionV1],
) {
    require_vec(errors, "touched_actions", actions);
    if !actions
        .iter()
        .any(|action| action.action_id == "kernel.coder_handoff_validation_request.project")
    {
        errors.push(error(
            "touched_actions.action_id",
            "coder handoff projection action must be recorded",
        ));
    }
    for action in actions {
        require_non_empty(errors, "touched_actions.action_id", &action.action_id);
        require_non_empty(
            errors,
            "touched_actions.authority_effect",
            &action.authority_effect,
        );
        require_non_empty(errors, "touched_actions.evidence_ref", &action.evidence_ref);
    }
}

fn validate_receipts(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    receipts: &[CoderHandoffReceiptRefV1],
) {
    require_vec(errors, "receipt_refs", receipts);
    for required_kind in ["STATUS", "REVIEW_REQUEST"] {
        if !receipts
            .iter()
            .any(|receipt| receipt.receipt_kind == required_kind)
        {
            errors.push(error(
                "receipt_refs.receipt_kind",
                "handoff must record status and review request receipt refs",
            ));
        }
    }
}

fn validate_tests(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    tests: &[CoderHandoffTestEvidenceV1],
) {
    require_vec(errors, "tests", tests);
    if !tests
        .iter()
        .any(|test| test.outcome == CoderHandoffTestOutcome::Passed)
    {
        errors.push(error(
            "tests.outcome",
            "at least one passing focused test is required",
        ));
    }
    for test in tests {
        require_non_empty(errors, "tests.command", &test.command);
        require_non_empty(errors, "tests.evidence_ref", &test.evidence_ref);
        if test.outcome == CoderHandoffTestOutcome::Failed {
            errors.push(error(
                "tests.outcome",
                "failed tests cannot be hidden inside coder handoff",
            ));
        }
    }
}

fn validate_known_blockers(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    blockers: &[CoderKnownBlockerV1],
) {
    require_vec(errors, "known_blockers", blockers);
    for blocker in blockers {
        require_non_empty(errors, "known_blockers.blocker_id", &blocker.blocker_id);
        require_non_empty(errors, "known_blockers.summary", &blocker.summary);
        require_non_empty(errors, "known_blockers.evidence_ref", &blocker.evidence_ref);
    }
}

fn validate_requested_review(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    review: &RequestedValidationReviewV1,
) {
    if review.target_role != "WP_VALIDATOR" {
        errors.push(error(
            "requested_review.target_role",
            "MT handoff review target must be WP_VALIDATOR",
        ));
    }
    if review.receipt_kind != "REVIEW_REQUEST" {
        errors.push(error(
            "requested_review.receipt_kind",
            "MT handoff must generate a REVIEW_REQUEST",
        ));
    }
    if review.named_verb != "MT_HANDOFF" {
        errors.push(error(
            "requested_review.named_verb",
            "MT handoff should use the MT_HANDOFF named verb",
        ));
    }
    if review.review_mode != "OVERLAP" {
        errors.push(error(
            "requested_review.review_mode",
            "per-MT review requests must use bounded overlap mode",
        ));
    }
    if review.status_edit_allowed {
        errors.push(error(
            "requested_review.status_edit_allowed",
            "review request generation must not directly edit status fields",
        ));
    }
    if !review.generated_without_model_status_edit {
        errors.push(error(
            "requested_review.generated_without_model_status_edit",
            "review request must be generated without model status edits",
        ));
    }
    require_non_empty(
        errors,
        "requested_review.microtask_json_schema_id",
        &review.microtask_json_schema_id,
    );
    require_non_empty(
        errors,
        "requested_review.correlation_id",
        &review.correlation_id,
    );
}

fn validate_status_boundary(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    contract: &CoderHandoffValidationRequestContractV1,
) {
    if contract.status_fields_mutated_by_model {
        errors.push(error(
            "status_fields_mutated_by_model",
            "models must not edit status fields to perform handoff",
        ));
    }
}

fn validate_failure_states(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    failure_states: &[CoderHandoffFailureState],
) {
    for required in required_failure_states() {
        if !failure_states.contains(&required) {
            errors.push(error(
                "failure_states",
                "handoff failure states must cover identity, scope, proof, review, and status mutation hazards",
            ));
        }
    }
}

fn validate_research_basis(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    research_basis_refs: &[CoderHandoffResearchBasisV1],
) {
    require_vec(errors, "research_basis_refs", research_basis_refs);
    for required in [
        "docs.github.com/en/rest/pulls/review-requests",
        "docs.gitlab.com/api/merge_request_approvals",
        "gerrit-review.googlesource.com/Documentation/config-submit-requirements",
        "kubernetes.io/docs/concepts/workloads/pods/pod-condition",
    ] {
        if !research_basis_refs
            .iter()
            .any(|basis| basis.source_ref.contains(required))
        {
            errors.push(error(
                "research_basis_refs",
                "current review request, approval, submit-requirement, and typed-condition patterns must be recorded",
            ));
        }
    }
}

fn required_failure_states() -> Vec<CoderHandoffFailureState> {
    vec![
        CoderHandoffFailureState::MissingMicrotaskIdentity,
        CoderHandoffFailureState::MissingActorSession,
        CoderHandoffFailureState::ScopeOutsideAllowedPaths,
        CoderHandoffFailureState::MissingTouchedArtifacts,
        CoderHandoffFailureState::MissingTouchedActions,
        CoderHandoffFailureState::MissingReceipts,
        CoderHandoffFailureState::MissingTests,
        CoderHandoffFailureState::MissingEvidence,
        CoderHandoffFailureState::MissingKnownBlockerRecord,
        CoderHandoffFailureState::MissingRequestedReview,
        CoderHandoffFailureState::ManualStatusEditAttempt,
        CoderHandoffFailureState::ReviewRequestWouldMutateStatus,
    ]
}

fn path_allowed_by_any(path: &str, allowed_paths: &[String]) -> bool {
    allowed_paths
        .iter()
        .any(|allowed| path_allowed_by_pattern(path, allowed))
}

fn path_allowed_by_pattern(path: &str, pattern: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path.starts_with(prefix);
    }
    path == pattern
}

fn artifact(path: &str, kind: CoderHandoffArtifactKind) -> CoderTouchedArtifactV1 {
    CoderTouchedArtifactV1 {
        path: path.to_string(),
        kind,
        source_hash: source_hash("kernel002-mt056", &[path, artifact_kind_label(kind)]),
    }
}

fn source_hash(domain: &str, parts: &[&str]) -> String {
    format!(
        "sha256:{}",
        sha256_hex(format!("{domain}|{}", parts.join("|")).as_bytes())
    )
}

fn is_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.chars().all(|ch| ch.is_ascii_hexdigit()))
}

fn artifact_kind_label(kind: CoderHandoffArtifactKind) -> &'static str {
    match kind {
        CoderHandoffArtifactKind::SourceFile => "source-file",
        CoderHandoffArtifactKind::TestFile => "test-file",
        CoderHandoffArtifactKind::ProofHarnessFile => "proof-harness-file",
        CoderHandoffArtifactKind::GeneratedProjection => "generated-projection",
    }
}

fn test_evidence(command: &str, outcome: CoderHandoffTestOutcome) -> CoderHandoffTestEvidenceV1 {
    CoderHandoffTestEvidenceV1 {
        command: command.to_string(),
        outcome,
        evidence_ref: format!("test-output://{command}"),
    }
}

fn research(
    source_ref: &str,
    pattern_found: &str,
    selected_reuse: &str,
) -> CoderHandoffResearchBasisV1 {
    CoderHandoffResearchBasisV1 {
        source_ref: source_ref.to_string(),
        pattern_found: pattern_found.to_string(),
        selected_reuse: selected_reuse.to_string(),
    }
}

fn error(field: &'static str, message: &'static str) -> CoderHandoffValidationRequestError {
    CoderHandoffValidationRequestError { field, message }
}

fn require_non_empty(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(error(field, "value must not be empty"));
    }
}

fn require_vec<T>(
    errors: &mut Vec<CoderHandoffValidationRequestError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(error(field, "at least one value is required"));
    }
}

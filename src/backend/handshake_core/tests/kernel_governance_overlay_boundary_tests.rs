use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    governance_overlay_boundary::{
        kernel002_governance_overlay_boundary, project_governance_overlay_posture,
        validate_governance_overlay_boundary, validate_governance_overlay_transfer_request,
        GovernanceOverlayArtifactKind, GovernanceOverlayArtifactRefV1,
        GovernanceOverlayArtifactRole, GovernanceOverlayBoundaryValidationError,
        GovernanceOverlayTransferDirection, GovernanceOverlayTransferEffect,
        GovernanceOverlayTransferRequestV1,
    },
};

#[test]
fn governance_overlay_boundary_preserves_gov_artifacts_as_source_or_evidence_only() {
    let boundary = kernel002_governance_overlay_boundary();
    validate_governance_overlay_boundary(&boundary).expect("boundary must validate");

    assert!(boundary
        .imported_overlay_artifacts
        .iter()
        .all(|artifact| artifact.repo_relative_path.starts_with(".GOV/")));
    assert!(boundary
        .imported_overlay_artifacts
        .iter()
        .all(|artifact| artifact.role != GovernanceOverlayArtifactRole::RuntimeAuthority));
    assert!(boundary
        .product_runtime_authority_refs
        .contains(&"hsk.kernel.software_delivery_runtime_truth_record@1".to_string()));
    assert!(boundary
        .folded_source_refs
        .iter()
        .any(|source| source.contains("WP-1-Software-Delivery-Governance-Overlay-Boundary-v1")));
}

#[test]
fn overlay_validation_rejects_runtime_authority_and_non_gov_paths() {
    let mut boundary = kernel002_governance_overlay_boundary();
    boundary
        .imported_overlay_artifacts
        .push(GovernanceOverlayArtifactRefV1 {
            artifact_id: "bad-runtime-authority".to_string(),
            repo_relative_path: ".GOV/roles_shared/records/TASK_BOARD.md".to_string(),
            artifact_kind: GovernanceOverlayArtifactKind::TaskPacket,
            role: GovernanceOverlayArtifactRole::RuntimeAuthority,
            provenance_hash: "sha256:bad".to_string(),
            source_wp_id: "WP-1-Software-Delivery-Governance-Overlay-Boundary-v1".to_string(),
        });
    boundary
        .imported_overlay_artifacts
        .push(GovernanceOverlayArtifactRefV1 {
            artifact_id: "bad-path".to_string(),
            repo_relative_path: "src/backend/runtime.rs".to_string(),
            artifact_kind: GovernanceOverlayArtifactKind::GovernanceCheckReport,
            role: GovernanceOverlayArtifactRole::Evidence,
            provenance_hash: "sha256:bad-path".to_string(),
            source_wp_id: "WP-1-Software-Delivery-Governance-Overlay-Boundary-v1".to_string(),
        });

    let errors = validate_governance_overlay_boundary(&boundary)
        .expect_err("bad overlay boundary must fail");

    assert!(errors.iter().any(|error| matches!(
        error,
        GovernanceOverlayBoundaryValidationError::ImportedOverlayClaimsRuntimeAuthority { .. }
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        GovernanceOverlayBoundaryValidationError::ImportedOverlayPathOutsideGov { .. }
    )));
}

#[test]
fn import_export_requests_cannot_bypass_gates_or_mutate_runtime_truth() {
    let boundary = kernel002_governance_overlay_boundary();
    let mut request = sample_transfer_request(GovernanceOverlayTransferEffect::BypassGate);
    request.gate_refs.clear();

    let errors = validate_governance_overlay_transfer_request(&boundary, &request)
        .expect_err("gate bypass request must fail");

    assert!(errors.iter().any(|error| matches!(
        error,
        GovernanceOverlayBoundaryValidationError::TransferBypassesGate { .. }
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        GovernanceOverlayBoundaryValidationError::MissingGateRef { .. }
    )));

    let runtime_mutation =
        sample_transfer_request(GovernanceOverlayTransferEffect::MutateRuntimeTruth);
    let errors = validate_governance_overlay_transfer_request(&boundary, &runtime_mutation)
        .expect_err("runtime mutation request must fail");
    assert!(errors.iter().any(|error| matches!(
        error,
        GovernanceOverlayBoundaryValidationError::TransferMutatesRuntimeTruth { .. }
    )));
}

#[test]
fn overlay_projection_explains_imported_posture_without_elevating_authority() {
    let boundary = kernel002_governance_overlay_boundary();
    let request = sample_transfer_request(GovernanceOverlayTransferEffect::PreserveAsOverlay);
    let projection = project_governance_overlay_posture(&boundary, &request)
        .expect("valid overlay request should project");

    assert_eq!(projection.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(projection.runtime_truth_unchanged);
    assert_eq!(
        projection.operational_authority_refs,
        boundary.product_runtime_authority_refs
    );
    assert!(projection
        .dcc_preview_fields
        .contains(&"overlay_role".to_string()));
}

#[test]
fn kernel_action_catalog_exposes_governance_overlay_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.governance_overlay.project")
        .expect("governance overlay projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "overlay_source_only"));
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "gate_required"));
}

fn sample_transfer_request(
    requested_effect: GovernanceOverlayTransferEffect,
) -> GovernanceOverlayTransferRequestV1 {
    GovernanceOverlayTransferRequestV1 {
        transfer_id: "overlay-transfer-001".to_string(),
        direction: GovernanceOverlayTransferDirection::Import,
        governed_action_id: "kernel.governance_overlay.project".to_string(),
        artifact_ids: vec!["overlay-task-packet-stub".to_string()],
        requested_effect,
        gate_refs: vec!["promotion-gate-kernel002".to_string()],
        projection_target_id: "dcc.governance-overlay".to_string(),
    }
}

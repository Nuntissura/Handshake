use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::action_envelope::AuthorityEffect;
use super::crdt::persistence::sha256_hex;

const FOLDED_GOVERNANCE_OVERLAY_STUB: &str =
    "WP-1-Software-Delivery-Governance-Overlay-Boundary-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GovernanceOverlayArtifactKind {
    TaskPacket,
    ReceiptLedger,
    RoleProtocol,
    GovernanceCheckReport,
    GovernancePackExport,
    GovernanceScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GovernanceOverlayArtifactRole {
    SourceMaterial,
    Evidence,
    RuntimeAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GovernanceOverlayTransferDirection {
    Import,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GovernanceOverlayTransferEffect {
    PreserveAsOverlay,
    CreateEvidenceRecord,
    MutateRuntimeTruth,
    BypassGate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceOverlayArtifactRefV1 {
    pub artifact_id: String,
    pub repo_relative_path: String,
    pub artifact_kind: GovernanceOverlayArtifactKind,
    pub role: GovernanceOverlayArtifactRole,
    pub provenance_hash: String,
    pub source_wp_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceOverlayBoundaryV1 {
    pub schema_id: String,
    pub boundary_id: String,
    pub imported_overlay_artifacts: Vec<GovernanceOverlayArtifactRefV1>,
    pub product_runtime_authority_refs: Vec<String>,
    pub import_export_gate_ids: Vec<String>,
    pub dcc_projection_rules: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceOverlayTransferRequestV1 {
    pub transfer_id: String,
    pub direction: GovernanceOverlayTransferDirection,
    pub governed_action_id: String,
    pub artifact_ids: Vec<String>,
    pub requested_effect: GovernanceOverlayTransferEffect,
    pub gate_refs: Vec<String>,
    pub projection_target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceOverlayProjectionV1 {
    pub schema_id: String,
    pub transfer_id: String,
    pub authority_effect: AuthorityEffect,
    pub runtime_truth_unchanged: bool,
    pub imported_overlay_artifacts: Vec<GovernanceOverlayArtifactRefV1>,
    pub operational_authority_refs: Vec<String>,
    pub gate_refs: Vec<String>,
    pub dcc_preview_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceOverlayBoundaryValidationError {
    MissingBoundaryField {
        field: &'static str,
    },
    MissingArtifactField {
        artifact_id: String,
        field: &'static str,
    },
    DuplicateArtifactId {
        artifact_id: String,
    },
    ImportedOverlayPathOutsideGov {
        artifact_id: String,
        repo_relative_path: String,
    },
    ImportedOverlayClaimsRuntimeAuthority {
        artifact_id: String,
    },
    InvalidArtifactProvenanceHash {
        artifact_id: String,
    },
    MissingTransferField {
        transfer_id: String,
        field: &'static str,
    },
    UnknownTransferArtifact {
        transfer_id: String,
        artifact_id: String,
    },
    MissingGateRef {
        transfer_id: String,
    },
    UnknownGateRef {
        transfer_id: String,
        gate_ref: String,
    },
    TransferBypassesGate {
        transfer_id: String,
    },
    TransferMutatesRuntimeTruth {
        transfer_id: String,
    },
}

pub fn kernel002_governance_overlay_boundary() -> GovernanceOverlayBoundaryV1 {
    GovernanceOverlayBoundaryV1 {
        schema_id: "hsk.kernel.governance_overlay_boundary@1".to_string(),
        boundary_id: "kernel002-governance-overlay-boundary-v1".to_string(),
        imported_overlay_artifacts: vec![
            overlay_artifact(
                "overlay-task-packet-stub",
                ".GOV/task_packets/stubs/WP-1-Software-Delivery-Governance-Overlay-Boundary-v1.md",
                GovernanceOverlayArtifactKind::TaskPacket,
                GovernanceOverlayArtifactRole::SourceMaterial,
            ),
            overlay_artifact(
                "overlay-stub-contract",
                ".GOV/task_packets/stubs/WP-1-Software-Delivery-Governance-Overlay-Boundary-v1.contract.json",
                GovernanceOverlayArtifactKind::GovernancePackExport,
                GovernanceOverlayArtifactRole::Evidence,
            ),
            overlay_artifact(
                "overlay-receipt-ledger",
                ".GOV/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-002/RECEIPTS.jsonl",
                GovernanceOverlayArtifactKind::ReceiptLedger,
                GovernanceOverlayArtifactRole::Evidence,
            ),
        ],
        product_runtime_authority_refs: vec![
            "hsk.kernel.software_delivery_runtime_truth_record@1".to_string(),
            "hsk.kernel.workflow_transition_registry@1".to_string(),
            "hsk.event_ledger.stream@1".to_string(),
        ],
        import_export_gate_ids: vec![
            "promotion-gate-kernel002".to_string(),
            "governance-pack-import-gate".to_string(),
            "governance-pack-export-gate".to_string(),
        ],
        dcc_projection_rules: vec![
            "show_overlay_role".to_string(),
            "show_product_authority_refs".to_string(),
            "show_gate_refs".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Software-Delivery-Governance-Overlay-Boundary-v1.contract.json".to_string(),
            ".GOV/task_packets/stubs/WP-1-Software-Delivery-Governance-Overlay-Boundary-v1.md".to_string(),
        ],
    }
}

pub fn validate_governance_overlay_boundary(
    boundary: &GovernanceOverlayBoundaryV1,
) -> Result<(), Vec<GovernanceOverlayBoundaryValidationError>> {
    let mut errors = Vec::new();

    require_boundary_field(&mut errors, "schema_id", &boundary.schema_id);
    require_boundary_field(&mut errors, "boundary_id", &boundary.boundary_id);
    require_boundary_vec(
        &mut errors,
        "imported_overlay_artifacts",
        &boundary.imported_overlay_artifacts,
    );
    require_boundary_vec(
        &mut errors,
        "product_runtime_authority_refs",
        &boundary.product_runtime_authority_refs,
    );
    require_boundary_vec(
        &mut errors,
        "import_export_gate_ids",
        &boundary.import_export_gate_ids,
    );
    require_boundary_vec(
        &mut errors,
        "dcc_projection_rules",
        &boundary.dcc_projection_rules,
    );

    if !boundary
        .folded_source_refs
        .iter()
        .any(|source| source.contains(FOLDED_GOVERNANCE_OVERLAY_STUB))
    {
        errors.push(
            GovernanceOverlayBoundaryValidationError::MissingBoundaryField {
                field: "folded_source_refs",
            },
        );
    }

    let mut artifact_ids = HashSet::new();
    for artifact in &boundary.imported_overlay_artifacts {
        if !artifact_ids.insert(artifact.artifact_id.clone()) {
            errors.push(
                GovernanceOverlayBoundaryValidationError::DuplicateArtifactId {
                    artifact_id: artifact.artifact_id.clone(),
                },
            );
        }
        validate_overlay_artifact(artifact, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_governance_overlay_transfer_request(
    boundary: &GovernanceOverlayBoundaryV1,
    request: &GovernanceOverlayTransferRequestV1,
) -> Result<(), Vec<GovernanceOverlayBoundaryValidationError>> {
    let mut errors = Vec::new();

    require_transfer_field(&mut errors, request, "transfer_id", &request.transfer_id);
    require_transfer_field(
        &mut errors,
        request,
        "governed_action_id",
        &request.governed_action_id,
    );
    require_transfer_field(
        &mut errors,
        request,
        "projection_target_id",
        &request.projection_target_id,
    );
    if request.artifact_ids.is_empty() {
        errors.push(
            GovernanceOverlayBoundaryValidationError::MissingTransferField {
                transfer_id: request.transfer_id.clone(),
                field: "artifact_ids",
            },
        );
    }
    if request.gate_refs.is_empty() {
        errors.push(GovernanceOverlayBoundaryValidationError::MissingGateRef {
            transfer_id: request.transfer_id.clone(),
        });
    }

    match request.requested_effect {
        GovernanceOverlayTransferEffect::BypassGate => {
            errors.push(
                GovernanceOverlayBoundaryValidationError::TransferBypassesGate {
                    transfer_id: request.transfer_id.clone(),
                },
            );
        }
        GovernanceOverlayTransferEffect::MutateRuntimeTruth => {
            errors.push(
                GovernanceOverlayBoundaryValidationError::TransferMutatesRuntimeTruth {
                    transfer_id: request.transfer_id.clone(),
                },
            );
        }
        GovernanceOverlayTransferEffect::PreserveAsOverlay
        | GovernanceOverlayTransferEffect::CreateEvidenceRecord => {}
    }

    for artifact_id in &request.artifact_ids {
        if !boundary
            .imported_overlay_artifacts
            .iter()
            .any(|artifact| &artifact.artifact_id == artifact_id)
        {
            errors.push(
                GovernanceOverlayBoundaryValidationError::UnknownTransferArtifact {
                    transfer_id: request.transfer_id.clone(),
                    artifact_id: artifact_id.clone(),
                },
            );
        }
    }

    for gate_ref in &request.gate_refs {
        if !boundary.import_export_gate_ids.contains(gate_ref) {
            errors.push(GovernanceOverlayBoundaryValidationError::UnknownGateRef {
                transfer_id: request.transfer_id.clone(),
                gate_ref: gate_ref.clone(),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_governance_overlay_posture(
    boundary: &GovernanceOverlayBoundaryV1,
    request: &GovernanceOverlayTransferRequestV1,
) -> Result<GovernanceOverlayProjectionV1, Vec<GovernanceOverlayBoundaryValidationError>> {
    validate_governance_overlay_boundary(boundary)?;
    validate_governance_overlay_transfer_request(boundary, request)?;

    let requested_artifacts = boundary
        .imported_overlay_artifacts
        .iter()
        .filter(|artifact| request.artifact_ids.contains(&artifact.artifact_id))
        .cloned()
        .collect();

    Ok(GovernanceOverlayProjectionV1 {
        schema_id: "hsk.kernel.governance_overlay_projection@1".to_string(),
        transfer_id: request.transfer_id.clone(),
        authority_effect: AuthorityEffect::ProjectionOnly,
        runtime_truth_unchanged: true,
        imported_overlay_artifacts: requested_artifacts,
        operational_authority_refs: boundary.product_runtime_authority_refs.clone(),
        gate_refs: request.gate_refs.clone(),
        dcc_preview_fields: vec![
            "overlay_artifact_id".to_string(),
            "overlay_role".to_string(),
            "repo_relative_path".to_string(),
            "gate_refs".to_string(),
            "product_runtime_authority_refs".to_string(),
        ],
    })
}

fn overlay_artifact(
    artifact_id: &str,
    repo_relative_path: &str,
    artifact_kind: GovernanceOverlayArtifactKind,
    role: GovernanceOverlayArtifactRole,
) -> GovernanceOverlayArtifactRefV1 {
    GovernanceOverlayArtifactRefV1 {
        artifact_id: artifact_id.to_string(),
        repo_relative_path: repo_relative_path.to_string(),
        artifact_kind,
        role,
        provenance_hash: overlay_provenance_hash(repo_relative_path, artifact_kind, role),
        source_wp_id: FOLDED_GOVERNANCE_OVERLAY_STUB.to_string(),
    }
}

fn overlay_provenance_hash(
    repo_relative_path: &str,
    artifact_kind: GovernanceOverlayArtifactKind,
    role: GovernanceOverlayArtifactRole,
) -> String {
    format!(
        "sha256:{}",
        sha256_hex(
            format!(
                "kernel002-governance-overlay|{repo_relative_path}|{}|{}",
                artifact_kind_label(artifact_kind),
                artifact_role_label(role)
            )
            .as_bytes()
        )
    )
}

fn validate_overlay_artifact(
    artifact: &GovernanceOverlayArtifactRefV1,
    errors: &mut Vec<GovernanceOverlayBoundaryValidationError>,
) {
    require_artifact_field(errors, artifact, "artifact_id", &artifact.artifact_id);
    require_artifact_field(
        errors,
        artifact,
        "repo_relative_path",
        &artifact.repo_relative_path,
    );
    require_artifact_field(
        errors,
        artifact,
        "provenance_hash",
        &artifact.provenance_hash,
    );
    if !is_sha256_digest(&artifact.provenance_hash) {
        errors.push(
            GovernanceOverlayBoundaryValidationError::InvalidArtifactProvenanceHash {
                artifact_id: artifact.artifact_id.clone(),
            },
        );
    }
    require_artifact_field(errors, artifact, "source_wp_id", &artifact.source_wp_id);

    let normalized_path = artifact.repo_relative_path.replace('\\', "/");
    if !normalized_path.starts_with(".GOV/") {
        errors.push(
            GovernanceOverlayBoundaryValidationError::ImportedOverlayPathOutsideGov {
                artifact_id: artifact.artifact_id.clone(),
                repo_relative_path: artifact.repo_relative_path.clone(),
            },
        );
    }

    if artifact.role == GovernanceOverlayArtifactRole::RuntimeAuthority {
        errors.push(
            GovernanceOverlayBoundaryValidationError::ImportedOverlayClaimsRuntimeAuthority {
                artifact_id: artifact.artifact_id.clone(),
            },
        );
    }
}

fn require_boundary_field(
    errors: &mut Vec<GovernanceOverlayBoundaryValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(GovernanceOverlayBoundaryValidationError::MissingBoundaryField { field });
    }
}

fn require_boundary_vec<T>(
    errors: &mut Vec<GovernanceOverlayBoundaryValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(GovernanceOverlayBoundaryValidationError::MissingBoundaryField { field });
    }
}

fn require_artifact_field(
    errors: &mut Vec<GovernanceOverlayBoundaryValidationError>,
    artifact: &GovernanceOverlayArtifactRefV1,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(
            GovernanceOverlayBoundaryValidationError::MissingArtifactField {
                artifact_id: artifact.artifact_id.clone(),
                field,
            },
        );
    }
}

fn require_transfer_field(
    errors: &mut Vec<GovernanceOverlayBoundaryValidationError>,
    request: &GovernanceOverlayTransferRequestV1,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(
            GovernanceOverlayBoundaryValidationError::MissingTransferField {
                transfer_id: request.transfer_id.clone(),
                field,
            },
        );
    }
}

fn is_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.chars().all(|ch| ch.is_ascii_hexdigit()))
}

fn artifact_kind_label(kind: GovernanceOverlayArtifactKind) -> &'static str {
    match kind {
        GovernanceOverlayArtifactKind::TaskPacket => "task-packet",
        GovernanceOverlayArtifactKind::ReceiptLedger => "receipt-ledger",
        GovernanceOverlayArtifactKind::RoleProtocol => "role-protocol",
        GovernanceOverlayArtifactKind::GovernanceCheckReport => "governance-check-report",
        GovernanceOverlayArtifactKind::GovernancePackExport => "governance-pack-export",
        GovernanceOverlayArtifactKind::GovernanceScript => "governance-script",
    }
}

fn artifact_role_label(role: GovernanceOverlayArtifactRole) -> &'static str {
    match role {
        GovernanceOverlayArtifactRole::SourceMaterial => "source-material",
        GovernanceOverlayArtifactRole::Evidence => "evidence",
        GovernanceOverlayArtifactRole::RuntimeAuthority => "runtime-authority",
    }
}

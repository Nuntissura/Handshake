use std::collections::HashSet;

use super::action_envelope::AuthorityEffect;

pub const MECHANICAL_CONTRACT_GENERATION_SCHEMA_ID: &str =
    "hsk.kernel.mechanical_contract_generation@1";
pub const WORK_PACKET_STUB_CONTRACT_SCHEMA_ID: &str = "hsk.work_packet_stub_contract@1";
pub const WORK_PACKET_CONTRACT_SCHEMA_ID: &str = "hsk.work_packet_contract@1";
pub const MICRO_TASK_CONTRACT_SCHEMA_ID: &str = "hsk.microtask_contract@1";

const WP_ID: &str = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MechanicalContractOperationKind {
    StubToWorkPacketPromotion,
    WorkPacketToMicrotaskExtraction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreservedContractField {
    OperatorIntent,
    SourceHashes,
    FoldedDetails,
    Dependencies,
    Constraints,
    AcceptanceCriteria,
    Verification,
    StatusProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeneratedContractArtifactKind {
    WorkPacketContract,
    WorkPacketProjection,
    RefinementContract,
    MicrotaskContract,
    MicrotaskProjection,
    TaskBoardProjection,
    TraceabilityProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MechanicalContractFailureState {
    MissingOperatorIntent,
    SourceHashMismatch,
    FoldedDetailLoss,
    DependencyLoss,
    ConstraintLoss,
    AcceptanceCriteriaLoss,
    VerificationLoss,
    StatusProvenanceLoss,
    MissingSourceContractId,
    ProjectionHashDrift,
    NonDeterministicOutput,
    DirectMutationBypass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicContractCommandV1 {
    pub command_id: String,
    pub command_line: String,
    pub script_ref: String,
    pub dry_run_supported: bool,
    pub repair_mode_supported: bool,
    pub output_schema_id: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFoldMapEntryV1 {
    pub source_ref: String,
    pub source_contract_id: String,
    pub destination_field: String,
    pub preservation_rule: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPreservationManifestEntryV1 {
    pub field: PreservedContractField,
    pub source_path: String,
    pub target_path: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedContractArtifactRefV1 {
    pub artifact_id: String,
    pub kind: GeneratedContractArtifactKind,
    pub path_template: String,
    pub source_contract_id: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MechanicalContractOperationV1 {
    pub operation_id: String,
    pub kind: MechanicalContractOperationKind,
    pub action_id: String,
    pub transition_action_ids: Vec<String>,
    pub source_schema_id: &'static str,
    pub target_schema_id: &'static str,
    pub command: DeterministicContractCommandV1,
    pub required_preserved_fields: Vec<PreservedContractField>,
    pub source_fold_map: Vec<SourceFoldMapEntryV1>,
    pub field_preservation_manifest: Vec<FieldPreservationManifestEntryV1>,
    pub generated_artifacts: Vec<GeneratedContractArtifactRefV1>,
    pub receipt_events: Vec<String>,
    pub validation_hooks: Vec<String>,
    pub status_provenance_fields: Vec<String>,
    pub authority_effect: AuthorityEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MechanicalContractGenerationV1 {
    pub schema_id: &'static str,
    pub wp_id: String,
    pub operations: Vec<MechanicalContractOperationV1>,
    pub provenance_fields: Vec<String>,
    pub failure_states: Vec<MechanicalContractFailureState>,
    pub direct_mutation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MechanicalContractGenerationValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

impl MechanicalContractGenerationV1 {
    pub fn operation(
        &self,
        kind: MechanicalContractOperationKind,
    ) -> Option<&MechanicalContractOperationV1> {
        self.operations
            .iter()
            .find(|operation| operation.kind == kind)
    }
}

pub fn build_kernel002_mechanical_contract_generation() -> MechanicalContractGenerationV1 {
    MechanicalContractGenerationV1 {
        schema_id: MECHANICAL_CONTRACT_GENERATION_SCHEMA_ID,
        wp_id: WP_ID.to_string(),
        operations: vec![stub_promotion_operation(), microtask_extraction_operation()],
        provenance_fields: vec![
            "source_contract_id".to_string(),
            "source_file".to_string(),
            "source_fold_map".to_string(),
            "field_preservation_manifest".to_string(),
            "source_hash".to_string(),
            "projection_hash".to_string(),
            "generated_at_utc".to_string(),
            "generator".to_string(),
            "receipt_event".to_string(),
        ],
        failure_states: required_failure_states().to_vec(),
        direct_mutation_allowed: false,
    }
}

pub fn validate_mechanical_contract_generation(
    generation: &MechanicalContractGenerationV1,
) -> Result<(), Vec<MechanicalContractGenerationValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "wp_id", &generation.wp_id);
    require_vec(&mut errors, "operations", &generation.operations);
    require_vec(
        &mut errors,
        "provenance_fields",
        &generation.provenance_fields,
    );
    require_failure_states(&mut errors, &generation.failure_states);

    if generation.schema_id != MECHANICAL_CONTRACT_GENERATION_SCHEMA_ID {
        errors.push(error(
            "schema_id",
            "mechanical contract generation schema id is required",
        ));
    }
    if generation.direct_mutation_allowed {
        errors.push(error(
            "direct_mutation_allowed",
            "mechanical generation must not bypass registered actions",
        ));
    }
    for required in [
        "source_contract_id",
        "source_fold_map",
        "field_preservation_manifest",
        "source_hash",
        "projection_hash",
        "generated_at_utc",
        "generator",
    ] {
        if !generation
            .provenance_fields
            .iter()
            .any(|field| field == required)
        {
            errors.push(error(
                "provenance_fields",
                "required provenance field is missing",
            ));
        }
    }

    validate_operation_set(generation, &mut errors);
    for operation in &generation.operations {
        validate_operation(operation, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_operation_set(
    generation: &MechanicalContractGenerationV1,
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
) {
    for kind in [
        MechanicalContractOperationKind::StubToWorkPacketPromotion,
        MechanicalContractOperationKind::WorkPacketToMicrotaskExtraction,
    ] {
        if generation.operation(kind).is_none() {
            errors.push(error(
                "operations",
                "required mechanical contract operation is missing",
            ));
        }
    }

    let mut ids = HashSet::new();
    for operation in &generation.operations {
        if !ids.insert(operation.operation_id.as_str()) {
            errors.push(error(
                "operations.operation_id",
                "operation ids must be unique",
            ));
        }
    }
}

fn validate_operation(
    operation: &MechanicalContractOperationV1,
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
) {
    require_non_empty(errors, "operations.operation_id", &operation.operation_id);
    require_non_empty(errors, "operations.action_id", &operation.action_id);
    require_vec(
        errors,
        "operations.transition_action_ids",
        &operation.transition_action_ids,
    );
    require_vec(
        errors,
        "operations.required_preserved_fields",
        &operation.required_preserved_fields,
    );
    require_vec(
        errors,
        "operations.source_fold_map",
        &operation.source_fold_map,
    );
    require_vec(
        errors,
        "operations.field_preservation_manifest",
        &operation.field_preservation_manifest,
    );
    require_vec(
        errors,
        "operations.generated_artifacts",
        &operation.generated_artifacts,
    );
    require_vec(
        errors,
        "operations.receipt_events",
        &operation.receipt_events,
    );
    require_vec(
        errors,
        "operations.validation_hooks",
        &operation.validation_hooks,
    );
    require_vec(
        errors,
        "operations.status_provenance_fields",
        &operation.status_provenance_fields,
    );

    validate_command(&operation.command, errors);
    validate_source_fold_map(&operation.source_fold_map, errors);
    validate_field_preservation_manifest(operation, errors);
    validate_artifacts(&operation.generated_artifacts, errors);

    for required in required_preserved_fields() {
        if !operation.required_preserved_fields.contains(&required) {
            errors.push(error(
                "operations.required_preserved_fields",
                "required preserved contract detail is missing",
            ));
        }
    }
    for field in [
        "lifecycle.status",
        "markdown_projection.source_file",
        "markdown_projection.source_hash",
        "markdown_projection.projection_hash",
        "markdown_projection.generated_at_utc",
        "markdown_projection.generator",
        "receipt_event",
    ] {
        if !operation
            .status_provenance_fields
            .iter()
            .any(|value| value == field)
        {
            errors.push(error(
                "operations.status_provenance_fields",
                "required status provenance field is missing",
            ));
        }
    }

    match operation.kind {
        MechanicalContractOperationKind::StubToWorkPacketPromotion => {
            if operation.source_schema_id != WORK_PACKET_STUB_CONTRACT_SCHEMA_ID
                || operation.target_schema_id != WORK_PACKET_CONTRACT_SCHEMA_ID
            {
                errors.push(error(
                    "operations.schema_id",
                    "stub promotion must convert stub contract to work packet contract",
                ));
            }
            if operation.action_id != "kernel.work_packet_contract.activate" {
                errors.push(error(
                    "operations.action_id",
                    "stub promotion must activate a work packet contract",
                ));
            }
            require_artifact_kind(
                errors,
                &operation.generated_artifacts,
                GeneratedContractArtifactKind::WorkPacketContract,
            );
            require_artifact_kind(
                errors,
                &operation.generated_artifacts,
                GeneratedContractArtifactKind::WorkPacketProjection,
            );
        }
        MechanicalContractOperationKind::WorkPacketToMicrotaskExtraction => {
            if operation.source_schema_id != WORK_PACKET_CONTRACT_SCHEMA_ID
                || operation.target_schema_id != MICRO_TASK_CONTRACT_SCHEMA_ID
            {
                errors.push(error(
                    "operations.schema_id",
                    "microtask extraction must convert work packet contract to microtask contracts",
                ));
            }
            if operation.action_id != "kernel.microtask_contract.extract" {
                errors.push(error(
                    "operations.action_id",
                    "microtask extraction action id is required",
                ));
            }
            require_artifact_kind(
                errors,
                &operation.generated_artifacts,
                GeneratedContractArtifactKind::MicrotaskContract,
            );
            require_artifact_kind(
                errors,
                &operation.generated_artifacts,
                GeneratedContractArtifactKind::MicrotaskProjection,
            );
            require_artifact_kind(
                errors,
                &operation.generated_artifacts,
                GeneratedContractArtifactKind::TaskBoardProjection,
            );
        }
    }
}

fn validate_command(
    command: &DeterministicContractCommandV1,
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
) {
    require_non_empty(errors, "operations.command.command_id", &command.command_id);
    require_non_empty(
        errors,
        "operations.command.command_line",
        &command.command_line,
    );
    require_non_empty(errors, "operations.command.script_ref", &command.script_ref);
    if !command.dry_run_supported {
        errors.push(error(
            "operations.command.dry_run_supported",
            "mechanical generation command must support dry-run verification",
        ));
    }
    if command.output_schema_id.trim().is_empty() {
        errors.push(error(
            "operations.command.output_schema_id",
            "command output schema id is required",
        ));
    }
}

fn validate_artifacts(
    artifacts: &[GeneratedContractArtifactRefV1],
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
) {
    for artifact in artifacts {
        require_non_empty(
            errors,
            "operations.generated_artifacts.artifact_id",
            &artifact.artifact_id,
        );
        require_non_empty(
            errors,
            "operations.generated_artifacts.path_template",
            &artifact.path_template,
        );
        require_non_empty(
            errors,
            "operations.generated_artifacts.source_contract_id",
            &artifact.source_contract_id,
        );
        if artifact.source_hash.len() < 16 {
            errors.push(error(
                "operations.generated_artifacts.source_hash",
                "generated artifacts must record a source hash",
            ));
        }
    }
}

fn validate_source_fold_map(
    fold_map: &[SourceFoldMapEntryV1],
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
) {
    for entry in fold_map {
        require_non_empty(
            errors,
            "operations.source_fold_map.source_ref",
            &entry.source_ref,
        );
        require_non_empty(
            errors,
            "operations.source_fold_map.source_contract_id",
            &entry.source_contract_id,
        );
        require_non_empty(
            errors,
            "operations.source_fold_map.destination_field",
            &entry.destination_field,
        );
        require_non_empty(
            errors,
            "operations.source_fold_map.preservation_rule",
            &entry.preservation_rule,
        );
    }
}

fn validate_field_preservation_manifest(
    operation: &MechanicalContractOperationV1,
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
) {
    for required in required_preserved_fields() {
        if !operation
            .field_preservation_manifest
            .iter()
            .any(|entry| entry.field == required && entry.required)
        {
            errors.push(error(
                "operations.field_preservation_manifest",
                "preservation manifest must cover every required field",
            ));
        }
    }
    for entry in &operation.field_preservation_manifest {
        require_non_empty(
            errors,
            "operations.field_preservation_manifest.source_path",
            &entry.source_path,
        );
        require_non_empty(
            errors,
            "operations.field_preservation_manifest.target_path",
            &entry.target_path,
        );
        if !entry.required {
            errors.push(error(
                "operations.field_preservation_manifest.required",
                "preserved fields must be required",
            ));
        }
    }
}

fn require_artifact_kind(
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
    artifacts: &[GeneratedContractArtifactRefV1],
    kind: GeneratedContractArtifactKind,
) {
    if !artifacts.iter().any(|artifact| artifact.kind == kind) {
        errors.push(error(
            "operations.generated_artifacts",
            "required generated artifact kind is missing",
        ));
    }
}

fn stub_promotion_operation() -> MechanicalContractOperationV1 {
    MechanicalContractOperationV1 {
        operation_id: "mechanical-stub-to-work-packet-promotion".to_string(),
        kind: MechanicalContractOperationKind::StubToWorkPacketPromotion,
        action_id: "kernel.work_packet_contract.activate".to_string(),
        transition_action_ids: vec![
            "kernel.stub_contract.prepare_promotion".to_string(),
            "kernel.work_packet_contract.activate".to_string(),
        ],
        source_schema_id: WORK_PACKET_STUB_CONTRACT_SCHEMA_ID,
        target_schema_id: WORK_PACKET_CONTRACT_SCHEMA_ID,
        command: DeterministicContractCommandV1 {
            command_id: "task-packet-stub-contracts-check".to_string(),
            command_line: "just task-packet-stub-contracts --check".to_string(),
            script_ref: ".GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs"
                .to_string(),
            dry_run_supported: true,
            repair_mode_supported: true,
            output_schema_id: WORK_PACKET_STUB_CONTRACT_SCHEMA_ID,
        },
        required_preserved_fields: required_preserved_fields().to_vec(),
        source_fold_map: vec![
            fold_map(
                ".GOV/task_packets/stubs/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1.contract.json#draft_scope.intent",
                "hsk.work_packet_stub_contract@1:WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
                "packet.json#scope.summary",
                "PRESERVE_OPERATOR_INTENT_VERBATIM",
            ),
            fold_map(
                ".GOV/task_packets/stubs/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1.contract.json#draft_scope.scope_sketch",
                "hsk.work_packet_stub_contract@1:WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
                "packet.json#scope.implementation_reality_notes",
                "PRESERVE_FOLDED_DETAIL_WITH_RESET_OVERRIDES",
            ),
            fold_map(
                ".GOV/task_packets/stubs/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1.contract.json#draft_scope.acceptance_criteria",
                "hsk.work_packet_stub_contract@1:WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
                "packet.json#scope.acceptance_criteria",
                "PRESERVE_ACCEPTANCE_CRITERIA",
            ),
        ],
        field_preservation_manifest: preservation_manifest("stub_contract", "packet_contract"),
        generated_artifacts: vec![
            artifact(
                "artifact-active-packet-json",
                GeneratedContractArtifactKind::WorkPacketContract,
                ".GOV/task_packets/{wp_id}/packet.json",
                "hsk.work_packet_stub_contract@1:WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
                "c9668ed8a5f42d21",
            ),
            artifact(
                "artifact-active-packet-md",
                GeneratedContractArtifactKind::WorkPacketProjection,
                ".GOV/task_packets/{wp_id}/packet.md",
                "hsk.work_packet_contract@1:{wp_id}",
                "edb5123ab8260de7",
            ),
            artifact(
                "artifact-active-refinement-json",
                GeneratedContractArtifactKind::RefinementContract,
                ".GOV/task_packets/{wp_id}/refinement.json",
                "hsk.work_packet_contract@1:{wp_id}",
                "7ce22fe78f8cbfd3",
            ),
        ],
        receipt_events: vec!["STATUS".to_string(), "SPEC_CONFIRMATION".to_string()],
        validation_hooks: vec![
            "stub_promotion_preserves_operator_intent".to_string(),
            "stub_promotion_preserves_source_hashes".to_string(),
            "stub_promotion_imports_folded_details".to_string(),
            "stub_promotion_preserves_status_provenance".to_string(),
        ],
        status_provenance_fields: required_status_provenance_fields().to_vec(),
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
    }
}

fn microtask_extraction_operation() -> MechanicalContractOperationV1 {
    MechanicalContractOperationV1 {
        operation_id: "mechanical-work-packet-to-microtask-extraction".to_string(),
        kind: MechanicalContractOperationKind::WorkPacketToMicrotaskExtraction,
        action_id: "kernel.microtask_contract.extract".to_string(),
        transition_action_ids: vec!["kernel.microtask_contract.extract".to_string()],
        source_schema_id: WORK_PACKET_CONTRACT_SCHEMA_ID,
        target_schema_id: MICRO_TASK_CONTRACT_SCHEMA_ID,
        command: DeterministicContractCommandV1 {
            command_id: "wp-contract-import-dry-run".to_string(),
            command_line: format!("just wp-contract-import {WP_ID} --dry-run --no-repair"),
            script_ref: ".GOV/roles_shared/scripts/wp/wp-contract-import.mjs".to_string(),
            dry_run_supported: true,
            repair_mode_supported: true,
            output_schema_id: "hsk.wp_contract_import_result@1",
        },
        required_preserved_fields: required_preserved_fields().to_vec(),
        source_fold_map: vec![
            fold_map(
                "packet.json#microtasks.declared_ids",
                "hsk.work_packet_contract@1:{wp_id}",
                "MT-*.json#mt_id",
                "PRESERVE_ONE_TO_ONE_MICROTASK_IDENTITY",
            ),
            fold_map(
                "packet.json#scope.allowed_paths",
                "hsk.work_packet_contract@1:{wp_id}",
                "MT-*.json#scope.allowed_paths",
                "PRESERVE_ALLOWED_PATH_CONSTRAINTS",
            ),
            fold_map(
                "packet.json#scope.acceptance_criteria",
                "hsk.work_packet_contract@1:{wp_id}",
                "MT-*.json#scope.acceptance_criteria",
                "PRESERVE_ACCEPTANCE_AND_VERIFICATION_DETAIL",
            ),
        ],
        field_preservation_manifest: preservation_manifest(
            "work_packet_contract",
            "microtask_contract",
        ),
        generated_artifacts: vec![
            artifact(
                "artifact-microtask-json-glob",
                GeneratedContractArtifactKind::MicrotaskContract,
                ".GOV/task_packets/{wp_id}/MT-*.json",
                "hsk.work_packet_contract@1:{wp_id}",
                "0e92f69f5fc9d65c",
            ),
            artifact(
                "artifact-microtask-md-glob",
                GeneratedContractArtifactKind::MicrotaskProjection,
                ".GOV/task_packets/{wp_id}/MT-*.md",
                "hsk.work_packet_contract@1:{wp_id}",
                "12f3f18ec8773b2c",
            ),
            artifact(
                "artifact-task-board-row",
                GeneratedContractArtifactKind::TaskBoardProjection,
                ".GOV/task_packets/TASK_BOARD.md#{wp_id}",
                "hsk.work_packet_contract@1:{wp_id}",
                "15955b89678c621a",
            ),
            artifact(
                "artifact-traceability-row",
                GeneratedContractArtifactKind::TraceabilityProjection,
                ".GOV/task_packets/WP_TRACEABILITY_REGISTRY.md#{wp_id}",
                "hsk.work_packet_contract@1:{wp_id}",
                "744309d8eba07f2e",
            ),
        ],
        receipt_events: vec!["STATUS".to_string(), "CLAIM".to_string()],
        validation_hooks: vec![
            "microtask_extraction_preserves_source_hashes".to_string(),
            "microtask_extraction_preserves_dependencies".to_string(),
            "microtask_extraction_preserves_acceptance_criteria".to_string(),
            "microtask_extraction_preserves_verification".to_string(),
            "microtask_extraction_preserves_status_provenance".to_string(),
        ],
        status_provenance_fields: required_status_provenance_fields().to_vec(),
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
    }
}

fn artifact(
    artifact_id: &str,
    kind: GeneratedContractArtifactKind,
    path_template: &str,
    source_contract_id: &str,
    source_hash: &str,
) -> GeneratedContractArtifactRefV1 {
    GeneratedContractArtifactRefV1 {
        artifact_id: artifact_id.to_string(),
        kind,
        path_template: path_template.to_string(),
        source_contract_id: source_contract_id.to_string(),
        source_hash: source_hash.to_string(),
    }
}

fn fold_map(
    source_ref: &str,
    source_contract_id: &str,
    destination_field: &str,
    preservation_rule: &str,
) -> SourceFoldMapEntryV1 {
    SourceFoldMapEntryV1 {
        source_ref: source_ref.to_string(),
        source_contract_id: source_contract_id.to_string(),
        destination_field: destination_field.to_string(),
        preservation_rule: preservation_rule.to_string(),
    }
}

fn preservation_manifest(
    source_root: &str,
    target_root: &str,
) -> Vec<FieldPreservationManifestEntryV1> {
    required_preserved_fields()
        .iter()
        .map(|field| {
            let segment = match field {
                PreservedContractField::OperatorIntent => "operator_intent",
                PreservedContractField::SourceHashes => "source_hashes",
                PreservedContractField::FoldedDetails => "folded_details",
                PreservedContractField::Dependencies => "dependencies",
                PreservedContractField::Constraints => "constraints",
                PreservedContractField::AcceptanceCriteria => "acceptance_criteria",
                PreservedContractField::Verification => "verification",
                PreservedContractField::StatusProvenance => "status_provenance",
            };
            FieldPreservationManifestEntryV1 {
                field: *field,
                source_path: format!("{source_root}.{segment}"),
                target_path: format!("{target_root}.{segment}"),
                required: true,
            }
        })
        .collect()
}

fn required_preserved_fields() -> [PreservedContractField; 8] {
    [
        PreservedContractField::OperatorIntent,
        PreservedContractField::SourceHashes,
        PreservedContractField::FoldedDetails,
        PreservedContractField::Dependencies,
        PreservedContractField::Constraints,
        PreservedContractField::AcceptanceCriteria,
        PreservedContractField::Verification,
        PreservedContractField::StatusProvenance,
    ]
}

fn required_status_provenance_fields() -> Vec<String> {
    [
        "lifecycle.status",
        "markdown_projection.source_file",
        "markdown_projection.source_hash",
        "markdown_projection.projection_hash",
        "markdown_projection.generated_at_utc",
        "markdown_projection.generator",
        "receipt_event",
    ]
    .iter()
    .map(|field| (*field).to_string())
    .collect()
}

fn required_failure_states() -> [MechanicalContractFailureState; 12] {
    [
        MechanicalContractFailureState::MissingOperatorIntent,
        MechanicalContractFailureState::SourceHashMismatch,
        MechanicalContractFailureState::FoldedDetailLoss,
        MechanicalContractFailureState::DependencyLoss,
        MechanicalContractFailureState::ConstraintLoss,
        MechanicalContractFailureState::AcceptanceCriteriaLoss,
        MechanicalContractFailureState::VerificationLoss,
        MechanicalContractFailureState::StatusProvenanceLoss,
        MechanicalContractFailureState::MissingSourceContractId,
        MechanicalContractFailureState::ProjectionHashDrift,
        MechanicalContractFailureState::NonDeterministicOutput,
        MechanicalContractFailureState::DirectMutationBypass,
    ]
}

fn require_failure_states(
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
    states: &[MechanicalContractFailureState],
) {
    require_vec(errors, "failure_states", states);
    for state in required_failure_states() {
        if !states.contains(&state) {
            errors.push(error(
                "failure_states",
                "required mechanical generation failure state is missing",
            ));
        }
    }
}

fn require_non_empty(
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(error(field, "value must not be empty"));
    }
}

fn require_vec<T>(
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(error(field, "at least one value is required"));
    }
}

fn error(
    field: &'static str,
    message: &'static str,
) -> MechanicalContractGenerationValidationError {
    MechanicalContractGenerationValidationError { field, message }
}

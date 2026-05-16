use serde::{Deserialize, Serialize};

use super::crdt::persistence::sha256_hex;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use super::action_envelope::AuthorityEffect;

pub const MECHANICAL_CONTRACT_GENERATION_SCHEMA_ID: &str =
    "hsk.kernel.mechanical_contract_generation@1";
pub const WORK_PACKET_STUB_CONTRACT_SCHEMA_ID: &str = "hsk.work_packet_stub_contract@1";
pub const WORK_PACKET_CONTRACT_SCHEMA_ID: &str = "hsk.work_packet_contract@1";
pub const MICRO_TASK_CONTRACT_SCHEMA_ID: &str = "hsk.microtask_contract@1";

const WP_ID: &str = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MechanicalContractOperationKind {
    StubToWorkPacketPromotion,
    WorkPacketToMicrotaskExtraction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeneratedContractArtifactKind {
    WorkPacketContract,
    WorkPacketProjection,
    RefinementContract,
    MicrotaskContract,
    MicrotaskProjection,
    TaskBoardProjection,
    TraceabilityProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicContractCommandV1 {
    pub command_id: String,
    pub command_line: String,
    pub script_ref: String,
    pub dry_run_supported: bool,
    pub repair_mode_supported: bool,
    pub output_schema_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFoldMapEntryV1 {
    pub source_ref: String,
    pub source_contract_id: String,
    pub destination_field: String,
    pub preservation_rule: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldPreservationManifestEntryV1 {
    pub field: PreservedContractField,
    pub source_path: String,
    pub target_path: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedContractArtifactRefV1 {
    pub artifact_id: String,
    pub kind: GeneratedContractArtifactKind,
    pub path_template: String,
    pub source_contract_id: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableCommandReceiptV1 {
    pub receipt_id: String,
    pub command_line: String,
    pub script_ref: String,
    pub workdir_ref: String,
    pub script_resolution: String,
    pub receipt_ref: String,
    pub current_candidate_receipt_schema_id: String,
    pub candidate_receipt_ref: String,
    pub candidate_sha_ref: String,
    pub expected_exit_code: i32,
    pub records_actual_exit_code: bool,
    pub artifact_refs: Vec<String>,
    pub projection_refs: Vec<String>,
    pub failure_blocker_policy: String,
    pub blocker_evidence_refs: Vec<String>,
    pub blocks_activation_on_failure: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentCandidateCommandReceiptInputV1 {
    pub command_line: String,
    pub workdir: String,
    pub candidate_sha: String,
    pub expected_exit_code: i32,
    pub actual_exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub blocker_refs: Vec<String>,
    pub projection_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentCandidateCommandReceiptV1 {
    pub schema_id: String,
    pub receipt_id: String,
    pub command_line: String,
    pub workdir: String,
    pub candidate_sha: String,
    pub expected_exit_code: i32,
    pub actual_exit_code: i32,
    pub stdout_ref: String,
    pub stderr_ref: String,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub blocker_refs: Vec<String>,
    pub projection_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentCandidateCommandReceiptWriteResultV1 {
    pub receipt: CurrentCandidateCommandReceiptV1,
    pub receipt_path: PathBuf,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
    pub blocker_path: Option<PathBuf>,
}

#[derive(Debug)]
pub enum CurrentCandidateCommandReceiptError {
    InvalidInput(&'static str),
    MissingBlockerRefsForNonzeroExit,
    Io(std::io::Error),
    Serialize(serde_json::Error),
}

impl From<std::io::Error> for CurrentCandidateCommandReceiptError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for CurrentCandidateCommandReceiptError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialize(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanicalContractOperationV1 {
    pub operation_id: String,
    pub kind: MechanicalContractOperationKind,
    pub action_id: String,
    pub transition_action_ids: Vec<String>,
    pub source_schema_id: String,
    pub target_schema_id: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanicalContractGenerationV1 {
    pub schema_id: String,
    pub wp_id: String,
    pub operations: Vec<MechanicalContractOperationV1>,
    pub durable_command_receipts: Vec<DurableCommandReceiptV1>,
    pub provenance_fields: Vec<String>,
    pub failure_states: Vec<MechanicalContractFailureState>,
    pub direct_mutation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        schema_id: MECHANICAL_CONTRACT_GENERATION_SCHEMA_ID.to_string(),
        wp_id: WP_ID.to_string(),
        operations: vec![stub_promotion_operation(), microtask_extraction_operation()],
        durable_command_receipts: durable_command_receipts(),
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

pub fn write_current_candidate_command_receipt(
    input: CurrentCandidateCommandReceiptInputV1,
    artifact_root: impl AsRef<Path>,
) -> Result<CurrentCandidateCommandReceiptWriteResultV1, CurrentCandidateCommandReceiptError> {
    if input.command_line.trim().is_empty() {
        return Err(CurrentCandidateCommandReceiptError::InvalidInput(
            "command_line is required",
        ));
    }
    if input.workdir.trim().is_empty() {
        return Err(CurrentCandidateCommandReceiptError::InvalidInput(
            "workdir is required",
        ));
    }
    if input.candidate_sha.trim().is_empty() {
        return Err(CurrentCandidateCommandReceiptError::InvalidInput(
            "candidate_sha is required",
        ));
    }
    if input.actual_exit_code != input.expected_exit_code && input.blocker_refs.is_empty() {
        return Err(CurrentCandidateCommandReceiptError::MissingBlockerRefsForNonzeroExit);
    }

    let artifact_root = artifact_root.as_ref();
    fs::create_dir_all(artifact_root)?;
    let slug = command_receipt_slug(&input.command_line);
    let stdout_path = artifact_root.join(format!("{slug}.stdout.txt"));
    let stderr_path = artifact_root.join(format!("{slug}.stderr.txt"));
    let receipt_path = artifact_root.join(format!("{slug}.json"));
    fs::write(&stdout_path, input.stdout.as_bytes())?;
    fs::write(&stderr_path, input.stderr.as_bytes())?;

    let blocker_path = if input.actual_exit_code != input.expected_exit_code {
        let path = artifact_root.join(format!("{slug}.blockers.json"));
        let blockers = serde_json::json!({
            "schema_id": "hsk.current_candidate_command_blockers@1",
            "command_line": &input.command_line,
            "workdir": &input.workdir,
            "candidate_sha": &input.candidate_sha,
            "expected_exit_code": input.expected_exit_code,
            "actual_exit_code": input.actual_exit_code,
            "blocker_refs": &input.blocker_refs,
        });
        fs::write(&path, serde_json::to_vec_pretty(&blockers)?)?;
        Some(path)
    } else {
        None
    };

    let stdout_ref = format!("artifact://command-receipts/{slug}.stdout.txt");
    let stderr_ref = format!("artifact://command-receipts/{slug}.stderr.txt");
    let receipt_ref = format!("artifact://command-receipts/{slug}.json");
    let mut artifact_refs = vec![receipt_ref, stdout_ref.clone(), stderr_ref.clone()];
    if blocker_path.is_some() {
        artifact_refs.push(format!("artifact://command-receipts/{slug}.blockers.json"));
    }

    let receipt = CurrentCandidateCommandReceiptV1 {
        schema_id: "hsk.current_candidate_command_receipt@1".to_string(),
        receipt_id: format!("receipt.current-candidate.{slug}"),
        command_line: input.command_line,
        workdir: input.workdir,
        candidate_sha: input.candidate_sha,
        expected_exit_code: input.expected_exit_code,
        actual_exit_code: input.actual_exit_code,
        stdout_ref,
        stderr_ref,
        stdout_sha256: format!("sha256:{}", sha256_hex(input.stdout.as_bytes())),
        stderr_sha256: format!("sha256:{}", sha256_hex(input.stderr.as_bytes())),
        blocker_refs: input.blocker_refs,
        projection_refs: input.projection_refs,
        artifact_refs,
    };
    fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt)?)?;

    Ok(CurrentCandidateCommandReceiptWriteResultV1 {
        receipt,
        receipt_path,
        stdout_path,
        stderr_path,
        blocker_path,
    })
}

pub fn command_receipt_slug(command_line: &str) -> String {
    let normalized = command_line
        .trim()
        .trim_start_matches("just ")
        .replace("--", "")
        .replace([' ', '/', '\\', ':'], "-");
    normalized
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn validate_mechanical_contract_generation(
    generation: &MechanicalContractGenerationV1,
) -> Result<(), Vec<MechanicalContractGenerationValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "wp_id", &generation.wp_id);
    require_vec(&mut errors, "operations", &generation.operations);
    require_vec(
        &mut errors,
        "durable_command_receipts",
        &generation.durable_command_receipts,
    );
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
    validate_durable_command_receipts(&generation.durable_command_receipts, &mut errors);

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
            require_artifact_kind(
                errors,
                &operation.generated_artifacts,
                GeneratedContractArtifactKind::TraceabilityProjection,
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
        if !is_sha256_digest(&artifact.source_hash) {
            errors.push(error(
                "operations.generated_artifacts.source_hash",
                "generated artifacts must record a sha256 source hash",
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
        source_schema_id: WORK_PACKET_STUB_CONTRACT_SCHEMA_ID.to_string(),
        target_schema_id: WORK_PACKET_CONTRACT_SCHEMA_ID.to_string(),
        command: DeterministicContractCommandV1 {
            command_id: "task-packet-stub-contracts-check".to_string(),
            command_line: "just task-packet-stub-contracts --all".to_string(),
            script_ref: ".GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs"
                .to_string(),
            dry_run_supported: true,
            repair_mode_supported: true,
            output_schema_id: WORK_PACKET_STUB_CONTRACT_SCHEMA_ID.to_string(),
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
            ),
            artifact(
                "artifact-active-packet-md",
                GeneratedContractArtifactKind::WorkPacketProjection,
                ".GOV/task_packets/{wp_id}/packet.md",
                "hsk.work_packet_contract@1:{wp_id}",
            ),
            artifact(
                "artifact-active-refinement-json",
                GeneratedContractArtifactKind::RefinementContract,
                ".GOV/task_packets/{wp_id}/refinement.json",
                "hsk.work_packet_contract@1:{wp_id}",
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
        source_schema_id: WORK_PACKET_CONTRACT_SCHEMA_ID.to_string(),
        target_schema_id: MICRO_TASK_CONTRACT_SCHEMA_ID.to_string(),
        command: DeterministicContractCommandV1 {
            command_id: "wp-contract-import-dry-run".to_string(),
            command_line: format!("just wp-contract-import {WP_ID} --dry-run --no-repair"),
            script_ref: ".GOV/roles_shared/scripts/wp/wp-contract-import.mjs".to_string(),
            dry_run_supported: true,
            repair_mode_supported: true,
            output_schema_id: "hsk.wp_contract_import_result@1".to_string(),
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
            ),
            artifact(
                "artifact-microtask-md-glob",
                GeneratedContractArtifactKind::MicrotaskProjection,
                ".GOV/task_packets/{wp_id}/MT-*.md",
                "hsk.work_packet_contract@1:{wp_id}",
            ),
            artifact(
                "artifact-task-board-row",
                GeneratedContractArtifactKind::TaskBoardProjection,
                ".GOV/roles_shared/records/TASK_BOARD.md#{wp_id}",
                "hsk.work_packet_contract@1:{wp_id}",
            ),
            artifact(
                "artifact-traceability-row",
                GeneratedContractArtifactKind::TraceabilityProjection,
                ".GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md#{wp_id}",
                "hsk.work_packet_contract@1:{wp_id}",
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

fn durable_command_receipts() -> Vec<DurableCommandReceiptV1> {
    vec![
        durable_command_receipt(
            "receipt-task-packet-stub-contracts-all",
            "just task-packet-stub-contracts --all",
            ".GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs",
        ),
        durable_command_receipt(
            "receipt-build-order-sync",
            "just build-order-sync",
            ".GOV/roles_shared/scripts/build-order-sync.mjs",
        ),
        durable_command_receipt(
            "receipt-gov-check",
            "just gov-check",
            ".GOV/roles_shared/checks/gov-check.mjs",
        ),
    ]
}

fn durable_command_receipt(
    receipt_id: &str,
    command_line: &str,
    script_ref: &str,
) -> DurableCommandReceiptV1 {
    let receipt_slug = receipt_id.strip_prefix("receipt-").unwrap_or(receipt_id);
    DurableCommandReceiptV1 {
        receipt_id: receipt_id.to_string(),
        command_line: command_line.to_string(),
        script_ref: script_ref.to_string(),
        workdir_ref: "repo-root://".to_string(),
        script_resolution: "resolve-script-ref-from-workdir".to_string(),
        receipt_ref: format!("receipt://mechanical-contract-generation/{receipt_id}"),
        current_candidate_receipt_schema_id: "hsk.kernel.current_candidate_command_receipt@1"
            .to_string(),
        candidate_receipt_ref: format!("artifact://command-receipts/{receipt_slug}.json"),
        candidate_sha_ref: "git://candidate/HEAD".to_string(),
        expected_exit_code: 0,
        records_actual_exit_code: true,
        artifact_refs: vec![
            format!("artifact://command-receipts/{receipt_slug}.stdout.txt"),
            format!("artifact://command-receipts/{receipt_slug}.stderr.txt"),
            format!("artifact://command-receipts/{receipt_slug}.json"),
        ],
        projection_refs: vec![
            ".GOV/task_packets/stubs".to_string(),
            ".GOV/roles_shared/records/BUILD_ORDER.md".to_string(),
            ".GOV/roles_shared/records/TASK_BOARD.md".to_string(),
            ".GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md".to_string(),
        ],
        failure_blocker_policy:
            "nonzero-exit-requires-current-candidate-blocker-summary-and-stdout-stderr-artifacts"
                .to_string(),
        blocker_evidence_refs: vec![
            format!("artifact://command-receipts/{receipt_slug}.stdout.txt"),
            format!("artifact://command-receipts/{receipt_slug}.stderr.txt"),
            format!("artifact://command-receipts/{receipt_slug}.blockers.json"),
        ],
        blocks_activation_on_failure: true,
    }
}

fn validate_durable_command_receipts(
    receipts: &[DurableCommandReceiptV1],
    errors: &mut Vec<MechanicalContractGenerationValidationError>,
) {
    let mut ids = HashSet::new();
    for receipt in receipts {
        if !ids.insert(receipt.receipt_id.as_str()) {
            errors.push(error(
                "durable_command_receipts.receipt_id",
                "durable command receipt ids must be unique",
            ));
        }
        require_non_empty(
            errors,
            "durable_command_receipts.receipt_id",
            &receipt.receipt_id,
        );
        require_non_empty(
            errors,
            "durable_command_receipts.command_line",
            &receipt.command_line,
        );
        require_non_empty(
            errors,
            "durable_command_receipts.script_ref",
            &receipt.script_ref,
        );
        require_non_empty(
            errors,
            "durable_command_receipts.workdir_ref",
            &receipt.workdir_ref,
        );
        require_non_empty(
            errors,
            "durable_command_receipts.script_resolution",
            &receipt.script_resolution,
        );
        require_non_empty(
            errors,
            "durable_command_receipts.receipt_ref",
            &receipt.receipt_ref,
        );
        require_non_empty(
            errors,
            "durable_command_receipts.current_candidate_receipt_schema_id",
            &receipt.current_candidate_receipt_schema_id,
        );
        require_non_empty(
            errors,
            "durable_command_receipts.candidate_receipt_ref",
            &receipt.candidate_receipt_ref,
        );
        require_non_empty(
            errors,
            "durable_command_receipts.candidate_sha_ref",
            &receipt.candidate_sha_ref,
        );
        require_vec(
            errors,
            "durable_command_receipts.artifact_refs",
            &receipt.artifact_refs,
        );
        require_vec(
            errors,
            "durable_command_receipts.projection_refs",
            &receipt.projection_refs,
        );
        require_non_empty(
            errors,
            "durable_command_receipts.failure_blocker_policy",
            &receipt.failure_blocker_policy,
        );
        require_vec(
            errors,
            "durable_command_receipts.blocker_evidence_refs",
            &receipt.blocker_evidence_refs,
        );
        if !receipt
            .receipt_ref
            .starts_with("receipt://mechanical-contract-generation/")
        {
            errors.push(error(
                "durable_command_receipts.receipt_ref",
                "durable command receipt ref must use the mechanical contract receipt namespace",
            ));
        }
        if receipt.workdir_ref != "repo-root://" {
            errors.push(error(
                "durable_command_receipts.workdir_ref",
                "durable command receipts must resolve from the current candidate repo workdir",
            ));
        }
        if receipt.script_resolution != "resolve-script-ref-from-workdir" {
            errors.push(error(
                "durable_command_receipts.script_resolution",
                "durable command receipts must declare workdir-relative script resolution",
            ));
        }
        if !receipt.blocks_activation_on_failure {
            errors.push(error(
                "durable_command_receipts.blocks_activation_on_failure",
                "required mechanical command receipts must block activation on failure",
            ));
        }
        if receipt.current_candidate_receipt_schema_id
            != "hsk.kernel.current_candidate_command_receipt@1"
        {
            errors.push(error(
                "durable_command_receipts.current_candidate_receipt_schema_id",
                "durable command receipts must declare the current-candidate command receipt schema",
            ));
        }
        if !receipt
            .candidate_receipt_ref
            .starts_with("artifact://command-receipts/")
        {
            errors.push(error(
                "durable_command_receipts.candidate_receipt_ref",
                "candidate command receipt must cite a command-receipt artifact",
            ));
        }
        if !receipt
            .candidate_sha_ref
            .starts_with("git://candidate/HEAD")
        {
            errors.push(error(
                "durable_command_receipts.candidate_sha_ref",
                "candidate command receipt must bind to the current candidate HEAD",
            ));
        }
        if receipt.expected_exit_code != 0 {
            errors.push(error(
                "durable_command_receipts.expected_exit_code",
                "acceptance command receipt must record a passing exit code",
            ));
        }
        if !receipt.records_actual_exit_code {
            errors.push(error(
                "durable_command_receipts.records_actual_exit_code",
                "candidate command receipts must record the actual exit code",
            ));
        }
        if !receipt
            .artifact_refs
            .iter()
            .any(|artifact_ref| artifact_ref.starts_with("artifact://command-receipts/"))
        {
            errors.push(error(
                "durable_command_receipts.artifact_refs",
                "durable command receipt must cite stdout/stderr/json artifacts",
            ));
        }
        if receipt.projection_refs.is_empty() {
            errors.push(error(
                "durable_command_receipts.projection_refs",
                "durable command receipt must cite affected governance projections",
            ));
        }
        if !receipt
            .failure_blocker_policy
            .contains("nonzero-exit-requires")
        {
            errors.push(error(
                "durable_command_receipts.failure_blocker_policy",
                "durable command receipt must define concrete blocker evidence for nonzero exits",
            ));
        }
        if !receipt
            .blocker_evidence_refs
            .iter()
            .any(|evidence_ref| evidence_ref.ends_with(".blockers.json"))
        {
            errors.push(error(
                "durable_command_receipts.blocker_evidence_refs",
                "durable command receipt must cite a blocker summary artifact for nonzero exits",
            ));
        }
    }
    for required in [
        "just task-packet-stub-contracts --all",
        "just build-order-sync",
        "just gov-check",
    ] {
        if !receipts
            .iter()
            .any(|receipt| receipt.command_line == required)
        {
            errors.push(error(
                "durable_command_receipts.command_line",
                "required exact durable command receipt is missing",
            ));
        }
    }
}

fn artifact(
    artifact_id: &str,
    kind: GeneratedContractArtifactKind,
    path_template: &str,
    source_contract_id: &str,
) -> GeneratedContractArtifactRefV1 {
    GeneratedContractArtifactRefV1 {
        artifact_id: artifact_id.to_string(),
        kind,
        path_template: path_template.to_string(),
        source_contract_id: source_contract_id.to_string(),
        source_hash: source_hash(
            "mechanical-contract-generation",
            &[
                artifact_id,
                generated_artifact_kind_label(kind),
                path_template,
                source_contract_id,
            ],
        ),
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

fn generated_artifact_kind_label(kind: GeneratedContractArtifactKind) -> &'static str {
    match kind {
        GeneratedContractArtifactKind::WorkPacketContract => "work-packet-contract",
        GeneratedContractArtifactKind::WorkPacketProjection => "work-packet-projection",
        GeneratedContractArtifactKind::RefinementContract => "refinement-contract",
        GeneratedContractArtifactKind::MicrotaskContract => "microtask-contract",
        GeneratedContractArtifactKind::MicrotaskProjection => "microtask-projection",
        GeneratedContractArtifactKind::TaskBoardProjection => "task-board-projection",
        GeneratedContractArtifactKind::TraceabilityProjection => "traceability-projection",
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

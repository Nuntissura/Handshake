use serde::{Deserialize, Serialize};

use std::collections::HashSet;

pub const WORK_PACKET_FULL_DETAIL_AUTHORITY_SCHEMA_ID: &str =
    "hsk.kernel.work_packet_full_detail_authority@1";
pub const MICROTASK_SOURCE_PLAN_SCHEMA_ID: &str = "hsk.microtask_source_plan@1";
pub const WORK_PACKET_CONTRACT_SCHEMA_ID: &str = "hsk.work_packet_contract@1";
pub const MICRO_TASK_CONTRACT_SCHEMA_ID: &str = "hsk.microtask_contract@1";

const WP_ID: &str = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkPacketFullDetailSection {
    Purpose,
    ArchitectureContext,
    OwnedSurfaces,
    RequiredBehavior,
    DataContracts,
    ScopeEdges,
    SourceImports,
    AcceptanceCriteria,
    VerificationPlan,
    RiskControls,
    NonGoals,
    ImplementationRealityNotes,
    NoContextExecutionNotes,
    MicrotaskSourcePlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MicrotaskPlanSourceField {
    MtId,
    Title,
    Summary,
    DependsOn,
    AllowedPaths,
    ForbiddenPaths,
    AcceptanceCriteria,
    ProofTargets,
    RiskIfMissed,
    CodeSurfaces,
    ExpectedTests,
    SourceRefs,
    HandoffRules,
    RedTeamProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeneratedArtifactKind {
    MicrotaskJsonContract,
    MarkdownProjection,
    TaskBoardRow,
    TraceabilityRow,
    DccWorkView,
    MirrorDoc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkPacketAuthorityFailureState {
    MissingFullDetail,
    MissingSourcePlan,
    HiddenChatContextDependency,
    ManualSidecarAuthority,
    SourceHashMismatch,
    ProjectionDrift,
    RoundTripLoss,
    InsufficientMicrotaskCoverage,
    NoContextExecutionBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkPacketFullDetailAuthoritySurfaceV1 {
    pub authority_file: String,
    pub contract_schema_id: String,
    pub required_sections: Vec<WorkPacketFullDetailSection>,
    pub source_refs: Vec<String>,
    pub can_execute_without_microtask_files: bool,
    pub no_context_model_profile: String,
    pub prohibited_dependency_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicrotaskSourcePlanV1 {
    pub schema_id: String,
    pub source_packet_ref: String,
    pub contract_glob: String,
    pub declared_microtask_count: usize,
    pub declared_ids: Vec<String>,
    pub source_fields: Vec<MicrotaskPlanSourceField>,
    pub generated_artifacts: Vec<GeneratedArtifactKind>,
    pub one_to_one_round_trip_required: bool,
    pub extraction_action_id: String,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegenerationContractV1 {
    pub generator: String,
    pub dry_run_command: String,
    pub output_contract_glob: String,
    pub output_projection_glob: String,
    pub required_provenance_fields: Vec<String>,
    pub validation_hooks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkPacketFullDetailAuthorityV1 {
    pub schema_id: String,
    pub wp_id: String,
    pub parent_contract_schema_id: String,
    pub microtask_contract_schema_id: String,
    pub packet_contract_ref: String,
    pub refinement_contract_ref: String,
    pub full_detail_authority: WorkPacketFullDetailAuthoritySurfaceV1,
    pub microtask_source_plan: MicrotaskSourcePlanV1,
    pub regeneration_contract: RegenerationContractV1,
    pub authority_rules: Vec<String>,
    pub validation_hooks: Vec<String>,
    pub failure_states: Vec<WorkPacketAuthorityFailureState>,
    pub hidden_chat_context_required: bool,
    pub manual_sidecar_authority_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkPacketFullDetailAuthorityValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_kernel002_work_packet_full_detail_authority() -> WorkPacketFullDetailAuthorityV1 {
    WorkPacketFullDetailAuthorityV1 {
        schema_id: WORK_PACKET_FULL_DETAIL_AUTHORITY_SCHEMA_ID.to_string(),
        wp_id: WP_ID.to_string(),
        parent_contract_schema_id: WORK_PACKET_CONTRACT_SCHEMA_ID.to_string(),
        microtask_contract_schema_id: MICRO_TASK_CONTRACT_SCHEMA_ID.to_string(),
        packet_contract_ref: packet_ref("packet.json"),
        refinement_contract_ref: packet_ref("refinement.json"),
        full_detail_authority: WorkPacketFullDetailAuthoritySurfaceV1 {
            authority_file: packet_ref("packet.json"),
            contract_schema_id: WORK_PACKET_CONTRACT_SCHEMA_ID.to_string(),
            required_sections: required_full_detail_sections().to_vec(),
            source_refs: vec![
                "packet.json#scope.summary".to_string(),
                "packet.json#scope.why".to_string(),
                "packet.json#scope.implementation_reality_notes".to_string(),
                "packet.json#scope.spec_anchors".to_string(),
                "packet.json#scope.allowed_paths".to_string(),
                "packet.json#scope.forbidden_paths".to_string(),
                "packet.json#scope.acceptance_criteria".to_string(),
                "packet.json#scope.implementation_reality_notes".to_string(),
                "packet.json#red_team".to_string(),
                "packet.json#workflow".to_string(),
                "packet.json#source_control".to_string(),
                "packet.json#microtasks.declared_ids".to_string(),
            ],
            can_execute_without_microtask_files: true,
            no_context_model_profile: "NO_CONTEXT_STRONG_MODEL".to_string(),
            prohibited_dependency_refs: vec![
                "hidden_chat_context".to_string(),
                "manual_sidecar_status".to_string(),
                "stale_markdown_projection".to_string(),
            ],
        },
        microtask_source_plan: MicrotaskSourcePlanV1 {
            schema_id: MICROTASK_SOURCE_PLAN_SCHEMA_ID.to_string(),
            source_packet_ref: packet_ref("packet.json"),
            contract_glob: packet_ref("MT-*.json"),
            declared_microtask_count: 61,
            declared_ids: required_microtask_ids(),
            source_fields: required_source_fields().to_vec(),
            generated_artifacts: required_generated_artifacts().to_vec(),
            one_to_one_round_trip_required: true,
            extraction_action_id: "kernel.microtask_contract.extract".to_string(),
            source_refs: vec![
                "packet.json#microtasks.declared_ids".to_string(),
                "packet.json#scope.allowed_paths".to_string(),
                "packet.json#scope.forbidden_paths".to_string(),
                "packet.json#scope.acceptance_criteria".to_string(),
                "packet.json#scope.implementation_reality_notes".to_string(),
                "packet.json#scope.spec_anchors".to_string(),
                "packet.json#red_team".to_string(),
            ],
        },
        regeneration_contract: RegenerationContractV1 {
            generator: "wp-contract-import.mjs".to_string(),
            dry_run_command: format!("just wp-contract-import {WP_ID} --dry-run --no-repair"),
            output_contract_glob: packet_ref("MT-*.json"),
            output_projection_glob: packet_ref("MT-*.md"),
            required_provenance_fields: vec![
                "source_file".to_string(),
                "source_hash".to_string(),
                "projection_hash".to_string(),
                "generated_at_utc".to_string(),
                "generator".to_string(),
            ],
            validation_hooks: vec![
                "work_packet_no_context_execution".to_string(),
                "microtask_source_plan_one_to_one".to_string(),
                "projection_source_hash_match".to_string(),
                "projection_hash_match".to_string(),
                "round_trip_generation_no_loss".to_string(),
                "manual_sidecar_authority_denied".to_string(),
            ],
        },
        authority_rules: vec![
            "packet_json_primary_authority".to_string(),
            "generated_markdown_projection_only".to_string(),
            "microtask_files_regenerated_from_packet".to_string(),
            "hidden_chat_context_denied".to_string(),
            "manual_sidecar_authority_denied".to_string(),
            "source_hash_required_before_claim".to_string(),
        ],
        validation_hooks: vec![
            "work_packet_no_context_execution".to_string(),
            "work_packet_source_plan_one_to_one".to_string(),
            "work_packet_projection_provenance".to_string(),
            "work_packet_no_sidecar_authority".to_string(),
            "work_packet_round_trip_loss".to_string(),
        ],
        failure_states: required_failure_states().to_vec(),
        hidden_chat_context_required: false,
        manual_sidecar_authority_allowed: false,
    }
}

pub fn validate_work_packet_full_detail_authority(
    authority: &WorkPacketFullDetailAuthorityV1,
) -> Result<(), Vec<WorkPacketFullDetailAuthorityValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "wp_id", &authority.wp_id);
    require_non_empty(
        &mut errors,
        "packet_contract_ref",
        &authority.packet_contract_ref,
    );
    require_non_empty(
        &mut errors,
        "refinement_contract_ref",
        &authority.refinement_contract_ref,
    );
    require_vec(&mut errors, "authority_rules", &authority.authority_rules);
    require_vec(&mut errors, "validation_hooks", &authority.validation_hooks);
    require_failure_states(&mut errors, &authority.failure_states);

    if authority.schema_id != WORK_PACKET_FULL_DETAIL_AUTHORITY_SCHEMA_ID {
        errors.push(error(
            "schema_id",
            "work packet full-detail authority schema id is required",
        ));
    }
    if authority.parent_contract_schema_id != WORK_PACKET_CONTRACT_SCHEMA_ID {
        errors.push(error(
            "parent_contract_schema_id",
            "parent work-packet contract schema id is required",
        ));
    }
    if authority.microtask_contract_schema_id != MICRO_TASK_CONTRACT_SCHEMA_ID {
        errors.push(error(
            "microtask_contract_schema_id",
            "microtask contract schema id is required",
        ));
    }
    if authority.hidden_chat_context_required {
        errors.push(error(
            "hidden_chat_context_required",
            "work packet execution must not depend on hidden chat context",
        ));
    }
    if authority.manual_sidecar_authority_allowed {
        errors.push(error(
            "manual_sidecar_authority_allowed",
            "manual sidecars cannot be authority for packet execution",
        ));
    }

    validate_full_detail_surface(&mut errors, &authority.full_detail_authority);
    validate_microtask_source_plan(&mut errors, &authority.microtask_source_plan);
    validate_regeneration_contract(&mut errors, &authority.regeneration_contract);

    for required_rule in required_authority_rules() {
        if !authority
            .authority_rules
            .iter()
            .any(|rule| rule == required_rule)
        {
            errors.push(error(
                "authority_rules",
                "work packet authority rule is missing",
            ));
        }
    }
    for required_hook in required_validation_hooks() {
        if !authority
            .validation_hooks
            .iter()
            .any(|hook| hook == required_hook)
        {
            errors.push(error(
                "validation_hooks",
                "work packet validation hook is missing",
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_full_detail_surface(
    errors: &mut Vec<WorkPacketFullDetailAuthorityValidationError>,
    surface: &WorkPacketFullDetailAuthoritySurfaceV1,
) {
    require_non_empty(
        errors,
        "full_detail_authority.authority_file",
        &surface.authority_file,
    );
    require_vec(
        errors,
        "full_detail_authority.required_sections",
        &surface.required_sections,
    );
    require_vec(
        errors,
        "full_detail_authority.source_refs",
        &surface.source_refs,
    );
    require_non_empty(
        errors,
        "full_detail_authority.no_context_model_profile",
        &surface.no_context_model_profile,
    );
    if surface.contract_schema_id != WORK_PACKET_CONTRACT_SCHEMA_ID {
        errors.push(error(
            "full_detail_authority.contract_schema_id",
            "full-detail authority must be the work packet contract",
        ));
    }
    if !surface.can_execute_without_microtask_files {
        errors.push(error(
            "full_detail_authority.can_execute_without_microtask_files",
            "a no-context strong model must be able to execute from the packet alone",
        ));
    }
    for required_section in required_full_detail_sections() {
        if !surface.required_sections.contains(&required_section) {
            errors.push(error(
                "full_detail_authority.required_sections",
                "required full-detail section is missing",
            ));
        }
    }
    for forbidden in [
        "hidden_chat_context",
        "manual_sidecar_status",
        "stale_markdown_projection",
    ] {
        if !surface
            .prohibited_dependency_refs
            .iter()
            .any(|value| value == forbidden)
        {
            errors.push(error(
                "full_detail_authority.prohibited_dependency_refs",
                "required prohibited dependency is missing",
            ));
        }
    }
}

fn validate_microtask_source_plan(
    errors: &mut Vec<WorkPacketFullDetailAuthorityValidationError>,
    plan: &MicrotaskSourcePlanV1,
) {
    require_non_empty(
        errors,
        "microtask_source_plan.source_packet_ref",
        &plan.source_packet_ref,
    );
    require_non_empty(
        errors,
        "microtask_source_plan.contract_glob",
        &plan.contract_glob,
    );
    require_vec(
        errors,
        "microtask_source_plan.declared_ids",
        &plan.declared_ids,
    );
    require_vec(
        errors,
        "microtask_source_plan.source_fields",
        &plan.source_fields,
    );
    require_vec(
        errors,
        "microtask_source_plan.generated_artifacts",
        &plan.generated_artifacts,
    );
    require_vec(
        errors,
        "microtask_source_plan.source_refs",
        &plan.source_refs,
    );
    require_non_empty(
        errors,
        "microtask_source_plan.extraction_action_id",
        &plan.extraction_action_id,
    );

    if plan.schema_id != MICROTASK_SOURCE_PLAN_SCHEMA_ID {
        errors.push(error(
            "microtask_source_plan.schema_id",
            "microtask source plan schema id is required",
        ));
    }
    if plan.declared_microtask_count != 61 || plan.declared_ids.len() != 61 {
        errors.push(error(
            "microtask_source_plan.declared_ids",
            "source plan must declare exactly 61 microtasks",
        ));
    }
    if plan.declared_microtask_count != plan.declared_ids.len() {
        errors.push(error(
            "microtask_source_plan.declared_microtask_count",
            "declared count must match declared ids",
        ));
    }
    if plan.declared_ids != required_microtask_ids() {
        errors.push(error(
            "microtask_source_plan.declared_ids",
            "declared microtask ids must round-trip MT-001 through MT-061",
        ));
    }
    if has_duplicates(&plan.declared_ids) {
        errors.push(error(
            "microtask_source_plan.declared_ids",
            "declared microtask ids must be unique",
        ));
    }
    if !plan.one_to_one_round_trip_required {
        errors.push(error(
            "microtask_source_plan.one_to_one_round_trip_required",
            "MT extraction must be a one-to-one round trip from packet source plan",
        ));
    }
    for required_field in required_source_fields() {
        if !plan.source_fields.contains(&required_field) {
            errors.push(error(
                "microtask_source_plan.source_fields",
                "required microtask source field is missing",
            ));
        }
    }
    for required_artifact in required_generated_artifacts() {
        if !plan.generated_artifacts.contains(&required_artifact) {
            errors.push(error(
                "microtask_source_plan.generated_artifacts",
                "required generated artifact kind is missing",
            ));
        }
    }
}

fn validate_regeneration_contract(
    errors: &mut Vec<WorkPacketFullDetailAuthorityValidationError>,
    contract: &RegenerationContractV1,
) {
    require_non_empty(
        errors,
        "regeneration_contract.generator",
        &contract.generator,
    );
    require_non_empty(
        errors,
        "regeneration_contract.dry_run_command",
        &contract.dry_run_command,
    );
    require_non_empty(
        errors,
        "regeneration_contract.output_contract_glob",
        &contract.output_contract_glob,
    );
    require_non_empty(
        errors,
        "regeneration_contract.output_projection_glob",
        &contract.output_projection_glob,
    );
    require_vec(
        errors,
        "regeneration_contract.required_provenance_fields",
        &contract.required_provenance_fields,
    );
    require_vec(
        errors,
        "regeneration_contract.validation_hooks",
        &contract.validation_hooks,
    );
    for field in [
        "source_file",
        "source_hash",
        "projection_hash",
        "generated_at_utc",
        "generator",
    ] {
        if !contract
            .required_provenance_fields
            .iter()
            .any(|value| value == field)
        {
            errors.push(error(
                "regeneration_contract.required_provenance_fields",
                "required projection provenance field is missing",
            ));
        }
    }
    for hook in [
        "work_packet_no_context_execution",
        "microtask_source_plan_one_to_one",
        "round_trip_generation_no_loss",
    ] {
        if !contract.validation_hooks.iter().any(|value| value == hook) {
            errors.push(error(
                "regeneration_contract.validation_hooks",
                "required regeneration validation hook is missing",
            ));
        }
    }
}

fn require_failure_states(
    errors: &mut Vec<WorkPacketFullDetailAuthorityValidationError>,
    states: &[WorkPacketAuthorityFailureState],
) {
    require_vec(errors, "failure_states", states);
    for state in required_failure_states() {
        if !states.contains(&state) {
            errors.push(error(
                "failure_states",
                "required work packet authority failure state is missing",
            ));
        }
    }
}

fn packet_ref(file: &str) -> String {
    format!(".GOV/task_packets/{WP_ID}/{file}")
}

fn required_microtask_ids() -> Vec<String> {
    (1..=61).map(|index| format!("MT-{index:03}")).collect()
}

fn required_full_detail_sections() -> [WorkPacketFullDetailSection; 14] {
    [
        WorkPacketFullDetailSection::Purpose,
        WorkPacketFullDetailSection::ArchitectureContext,
        WorkPacketFullDetailSection::OwnedSurfaces,
        WorkPacketFullDetailSection::RequiredBehavior,
        WorkPacketFullDetailSection::DataContracts,
        WorkPacketFullDetailSection::ScopeEdges,
        WorkPacketFullDetailSection::SourceImports,
        WorkPacketFullDetailSection::AcceptanceCriteria,
        WorkPacketFullDetailSection::VerificationPlan,
        WorkPacketFullDetailSection::RiskControls,
        WorkPacketFullDetailSection::NonGoals,
        WorkPacketFullDetailSection::ImplementationRealityNotes,
        WorkPacketFullDetailSection::NoContextExecutionNotes,
        WorkPacketFullDetailSection::MicrotaskSourcePlan,
    ]
}

fn required_source_fields() -> [MicrotaskPlanSourceField; 14] {
    [
        MicrotaskPlanSourceField::MtId,
        MicrotaskPlanSourceField::Title,
        MicrotaskPlanSourceField::Summary,
        MicrotaskPlanSourceField::DependsOn,
        MicrotaskPlanSourceField::AllowedPaths,
        MicrotaskPlanSourceField::ForbiddenPaths,
        MicrotaskPlanSourceField::AcceptanceCriteria,
        MicrotaskPlanSourceField::ProofTargets,
        MicrotaskPlanSourceField::RiskIfMissed,
        MicrotaskPlanSourceField::CodeSurfaces,
        MicrotaskPlanSourceField::ExpectedTests,
        MicrotaskPlanSourceField::SourceRefs,
        MicrotaskPlanSourceField::HandoffRules,
        MicrotaskPlanSourceField::RedTeamProfile,
    ]
}

fn required_generated_artifacts() -> [GeneratedArtifactKind; 6] {
    [
        GeneratedArtifactKind::MicrotaskJsonContract,
        GeneratedArtifactKind::MarkdownProjection,
        GeneratedArtifactKind::TaskBoardRow,
        GeneratedArtifactKind::TraceabilityRow,
        GeneratedArtifactKind::DccWorkView,
        GeneratedArtifactKind::MirrorDoc,
    ]
}

fn required_failure_states() -> [WorkPacketAuthorityFailureState; 9] {
    [
        WorkPacketAuthorityFailureState::MissingFullDetail,
        WorkPacketAuthorityFailureState::MissingSourcePlan,
        WorkPacketAuthorityFailureState::HiddenChatContextDependency,
        WorkPacketAuthorityFailureState::ManualSidecarAuthority,
        WorkPacketAuthorityFailureState::SourceHashMismatch,
        WorkPacketAuthorityFailureState::ProjectionDrift,
        WorkPacketAuthorityFailureState::RoundTripLoss,
        WorkPacketAuthorityFailureState::InsufficientMicrotaskCoverage,
        WorkPacketAuthorityFailureState::NoContextExecutionBlocked,
    ]
}

fn required_authority_rules() -> [&'static str; 6] {
    [
        "packet_json_primary_authority",
        "generated_markdown_projection_only",
        "microtask_files_regenerated_from_packet",
        "hidden_chat_context_denied",
        "manual_sidecar_authority_denied",
        "source_hash_required_before_claim",
    ]
}

fn required_validation_hooks() -> [&'static str; 5] {
    [
        "work_packet_no_context_execution",
        "work_packet_source_plan_one_to_one",
        "work_packet_projection_provenance",
        "work_packet_no_sidecar_authority",
        "work_packet_round_trip_loss",
    ]
}

fn has_duplicates(values: &[String]) -> bool {
    let mut seen = HashSet::new();
    values.iter().any(|value| !seen.insert(value))
}

fn require_non_empty(
    errors: &mut Vec<WorkPacketFullDetailAuthorityValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(error(field, "value must not be empty"));
    }
}

fn require_vec<T>(
    errors: &mut Vec<WorkPacketFullDetailAuthorityValidationError>,
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
) -> WorkPacketFullDetailAuthorityValidationError {
    WorkPacketFullDetailAuthorityValidationError { field, message }
}

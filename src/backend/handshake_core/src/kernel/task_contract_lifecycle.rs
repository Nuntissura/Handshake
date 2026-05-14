use std::collections::HashSet;

pub const TASK_CONTRACT_LIFECYCLE_SCHEMA_ID: &str = "hsk.kernel.task_contract_lifecycle@1";
pub const STUB_CONTRACT_SCHEMA_ID: &str = "hsk.work_packet_stub_contract@1";
pub const WORK_PACKET_CONTRACT_SCHEMA_ID: &str = "hsk.work_packet_contract@1";
pub const MICRO_TASK_CONTRACT_SCHEMA_ID: &str = "hsk.microtask_contract@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContractKind {
    Stub,
    WorkPacket,
    MicroTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContractLifecycleState {
    StubInactive,
    StubReadyForPromotion,
    WorkPacketActive,
    WorkPacketBlocked,
    WorkPacketValidated,
    MicroTaskGenerated,
    MicroTaskClaimable,
    MicroTaskClaimed,
    MicroTaskValidationRequested,
    MicroTaskInReview,
    MicroTaskRepairRequired,
    MicroTaskCleared,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContractAuthorityRule {
    MachineContractAuthority,
    GeneratedProjectionOnly,
    DirectStatusEditsDenied,
    ProvenanceHashRequired,
    PromotionGateRequired,
    ReceiptBackedLifecycle,
    ValidationHookRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContractFailureState {
    MissingRequiredField,
    SourceHashMismatch,
    ProjectionDrift,
    InvalidTransition,
    ReceiptMissing,
    ValidationFailed,
    AuthorityBoundaryViolation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceHashV1 {
    pub hash_kind: String,
    pub hash_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractSourceImportV1 {
    pub import_id: String,
    pub source_ref: String,
    pub source_contract_id: String,
    pub required: bool,
    pub provenance_hash: ProvenanceHashV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractLifecycleTransitionV1 {
    pub transition_id: String,
    pub from: ContractLifecycleState,
    pub to: ContractLifecycleState,
    pub action_id: String,
    pub receipt_event: String,
    pub projection_hook_id: String,
    pub validation_hook_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractSchemaDefinitionV1 {
    pub schema_id: &'static str,
    pub contract_kind: ContractKind,
    pub contract_id: String,
    pub lifecycle_state: ContractLifecycleState,
    pub authority_rules: Vec<ContractAuthorityRule>,
    pub required_fields: Vec<String>,
    pub provenance_hashes: Vec<ProvenanceHashV1>,
    pub source_import_refs: Vec<String>,
    pub lifecycle_transition_refs: Vec<String>,
    pub receipt_events: Vec<String>,
    pub projection_hooks: Vec<String>,
    pub validation_hooks: Vec<String>,
    pub failure_states: Vec<ContractFailureState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskContractLifecycleV1 {
    pub schema_id: &'static str,
    pub lifecycle_id: String,
    pub stub_contract: ContractSchemaDefinitionV1,
    pub work_packet_contract: ContractSchemaDefinitionV1,
    pub micro_task_contracts: Vec<ContractSchemaDefinitionV1>,
    pub source_imports: Vec<ContractSourceImportV1>,
    pub transitions: Vec<ContractLifecycleTransitionV1>,
    pub receipt_events: Vec<String>,
    pub projection_hooks: Vec<String>,
    pub validation_hooks: Vec<String>,
    pub failure_states: Vec<ContractFailureState>,
    pub markdown_authority_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskContractLifecycleValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_kernel002_task_contract_lifecycle() -> TaskContractLifecycleV1 {
    let receipt_events = vec![
        "CODER_INTENT".to_string(),
        "REVIEW_REQUEST".to_string(),
        "CODER_HANDOFF".to_string(),
        "REPAIR".to_string(),
        "VALIDATOR_QUERY".to_string(),
        "VALIDATOR_RESPONSE".to_string(),
        "VALIDATOR_REVIEW".to_string(),
        "REVIEW_RESPONSE".to_string(),
        "SPEC_CONFIRMATION".to_string(),
        "STATUS".to_string(),
        "CLAIM".to_string(),
        "BLOCKER".to_string(),
    ];
    let projection_hooks = vec![
        "task_board_projection".to_string(),
        "markdown_projection".to_string(),
        "dcc_work_view".to_string(),
        "traceability_registry_projection".to_string(),
        "failure_projection".to_string(),
    ];
    let validation_hooks = vec![
        "required_fields_present".to_string(),
        "source_hash_match".to_string(),
        "transition_allowed".to_string(),
        "projection_hash_match".to_string(),
        "receipt_event_recorded".to_string(),
        "mt_contract_generation".to_string(),
        "claim_lease_available".to_string(),
        "failure_state_recorded".to_string(),
    ];
    let failure_states = vec![
        ContractFailureState::MissingRequiredField,
        ContractFailureState::SourceHashMismatch,
        ContractFailureState::ProjectionDrift,
        ContractFailureState::InvalidTransition,
        ContractFailureState::ReceiptMissing,
        ContractFailureState::ValidationFailed,
        ContractFailureState::AuthorityBoundaryViolation,
    ];
    let transitions = transitions();

    TaskContractLifecycleV1 {
        schema_id: TASK_CONTRACT_LIFECYCLE_SCHEMA_ID,
        lifecycle_id: "kernel002-task-contract-lifecycle-mt051".to_string(),
        stub_contract: contract_schema(
            STUB_CONTRACT_SCHEMA_ID,
            ContractKind::Stub,
            "stub-contract-v1",
            ContractLifecycleState::StubInactive,
            &[
                "stub_id",
                "contract_id",
                "schema_version",
                "contract_authority",
                "artifact_policy",
                "execution_authority",
                "intent_summary",
                "source_hash",
                "source_files",
                "promotion_target",
                "markdown_projection",
                "build_order",
                "spec_trace",
                "session_policy",
                "draft_scope",
                "activation_contract",
                "red_team",
                "authority_rules",
                "lifecycle",
            ],
            &["source-import-stub-template"],
            &["transition-stub-promote-ready", "transition-wp-activate"],
            &receipt_events,
            &projection_hooks,
            &validation_hooks,
            &failure_states,
        ),
        work_packet_contract: contract_schema(
            WORK_PACKET_CONTRACT_SCHEMA_ID,
            ContractKind::WorkPacket,
            "work-packet-contract-v1",
            ContractLifecycleState::WorkPacketActive,
            &[
                "wp_id",
                "contract_id",
                "schema_version",
                "contract_authority",
                "artifact_policy",
                "base_wp_id",
                "created_at_utc",
                "updated_at_utc",
                "source_control",
                "workflow",
                "authority_files",
                "markdown_projection",
                "scope",
                "acceptance_criteria",
                "mt_source_plan",
                "source_imports",
                "proof_targets",
                "refinement",
                "microtasks",
                "role_profiles",
                "red_team",
                "lifecycle",
            ],
            &[
                "source-import-packet-contract",
                "source-import-stub-template",
            ],
            &["transition-wp-activate", "transition-mt-extract"],
            &receipt_events,
            &projection_hooks,
            &validation_hooks,
            &failure_states,
        ),
        micro_task_contracts: vec![contract_schema(
            MICRO_TASK_CONTRACT_SCHEMA_ID,
            ContractKind::MicroTask,
            "microtask-contract-v1",
            ContractLifecycleState::MicroTaskGenerated,
            &[
                "mt_id",
                "wp_id",
                "contract_id",
                "schema_version",
                "contract_authority",
                "artifact_policy",
                "created_at_utc",
                "updated_at_utc",
                "authority_files",
                "markdown_projection",
                "scope",
                "dependencies",
                "allowed_paths",
                "proof_targets",
                "handoff",
                "red_team",
                "lifecycle",
            ],
            &[
                "source-import-mt051-contract",
                "source-import-packet-contract",
            ],
            &[
                "transition-mt-extract",
                "transition-mt-publish",
                "transition-mt-claim",
                "transition-mt-review-request",
                "transition-mt-review-start",
                "transition-mt-repair-required",
                "transition-mt-clear",
            ],
            &receipt_events,
            &projection_hooks,
            &validation_hooks,
            &failure_states,
        )],
        source_imports: source_imports(),
        transitions,
        receipt_events,
        projection_hooks,
        validation_hooks,
        failure_states,
        markdown_authority_allowed: false,
    }
}

pub fn validate_task_contract_lifecycle(
    lifecycle: &TaskContractLifecycleV1,
) -> Result<(), Vec<TaskContractLifecycleValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "lifecycle_id", &lifecycle.lifecycle_id);
    require_vec(&mut errors, "source_imports", &lifecycle.source_imports);
    require_vec(&mut errors, "transitions", &lifecycle.transitions);
    require_vec(&mut errors, "receipt_events", &lifecycle.receipt_events);
    require_vec(&mut errors, "projection_hooks", &lifecycle.projection_hooks);
    require_vec(&mut errors, "validation_hooks", &lifecycle.validation_hooks);
    require_vec(&mut errors, "failure_states", &lifecycle.failure_states);

    if lifecycle.schema_id != TASK_CONTRACT_LIFECYCLE_SCHEMA_ID {
        errors.push(error(
            "schema_id",
            "task contract lifecycle schema id is required",
        ));
    }
    if lifecycle.markdown_authority_allowed {
        errors.push(error(
            "markdown_authority_allowed",
            "markdown is a projection only and cannot be contract authority",
        ));
    }

    validate_source_imports(&mut errors, &lifecycle.source_imports);
    validate_transitions(lifecycle, &mut errors);
    validate_failure_states(&mut errors, "failure_states", &lifecycle.failure_states);

    validate_contract(
        &mut errors,
        "stub_contract",
        &lifecycle.stub_contract,
        ContractKind::Stub,
        STUB_CONTRACT_SCHEMA_ID,
        &required_stub_fields(),
        lifecycle,
    );
    validate_contract(
        &mut errors,
        "work_packet_contract",
        &lifecycle.work_packet_contract,
        ContractKind::WorkPacket,
        WORK_PACKET_CONTRACT_SCHEMA_ID,
        &required_work_packet_fields(),
        lifecycle,
    );
    if lifecycle.micro_task_contracts.is_empty() {
        errors.push(error(
            "micro_task_contracts",
            "at least one microtask contract schema is required",
        ));
    }
    for contract in &lifecycle.micro_task_contracts {
        validate_contract(
            &mut errors,
            "micro_task_contracts",
            contract,
            ContractKind::MicroTask,
            MICRO_TASK_CONTRACT_SCHEMA_ID,
            &required_microtask_fields(),
            lifecycle,
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_contract(
    errors: &mut Vec<TaskContractLifecycleValidationError>,
    field_prefix: &'static str,
    contract: &ContractSchemaDefinitionV1,
    expected_kind: ContractKind,
    expected_schema_id: &'static str,
    required_fields: &[&str],
    lifecycle: &TaskContractLifecycleV1,
) {
    require_non_empty(errors, field_prefix, &contract.contract_id);
    require_vec(
        errors,
        authority_rule_field(field_prefix),
        &contract.authority_rules,
    );
    require_vec(
        errors,
        required_fields_field(field_prefix),
        &contract.required_fields,
    );
    require_vec(
        errors,
        provenance_hashes_field(field_prefix),
        &contract.provenance_hashes,
    );
    require_vec(
        errors,
        source_import_refs_field(field_prefix),
        &contract.source_import_refs,
    );
    require_vec(
        errors,
        transition_refs_field(field_prefix),
        &contract.lifecycle_transition_refs,
    );
    require_vec(
        errors,
        receipt_events_field(field_prefix),
        &contract.receipt_events,
    );
    require_vec(
        errors,
        projection_hooks_field(field_prefix),
        &contract.projection_hooks,
    );
    require_vec(
        errors,
        validation_hooks_field(field_prefix),
        &contract.validation_hooks,
    );
    require_vec(
        errors,
        failure_states_field(field_prefix),
        &contract.failure_states,
    );

    if contract.schema_id != expected_schema_id {
        errors.push(error(
            schema_field(field_prefix),
            "unexpected contract schema id",
        ));
    }
    if contract.contract_kind != expected_kind {
        errors.push(error(
            contract_kind_field(field_prefix),
            "unexpected contract kind",
        ));
    }
    for required_rule in required_authority_rules() {
        if !contract.authority_rules.contains(&required_rule) {
            errors.push(error(
                authority_rule_field(field_prefix),
                "contract is missing required authority rule",
            ));
        }
    }
    for required_field in required_fields {
        if !contract
            .required_fields
            .iter()
            .any(|field| field == required_field)
        {
            errors.push(error(
                required_fields_field(field_prefix),
                "contract is missing required field declaration",
            ));
        }
    }
    for hash in &contract.provenance_hashes {
        validate_hash(errors, provenance_hashes_field(field_prefix), hash);
    }
    validate_failure_states(
        errors,
        failure_states_field(field_prefix),
        &contract.failure_states,
    );

    let source_import_ids: HashSet<&str> = lifecycle
        .source_imports
        .iter()
        .map(|source| source.import_id.as_str())
        .collect();
    for source_ref in &contract.source_import_refs {
        if !source_import_ids.contains(source_ref.as_str()) {
            errors.push(error(
                source_import_refs_field(field_prefix),
                "contract references an unknown source import",
            ));
        }
    }

    let transition_ids: HashSet<&str> = lifecycle
        .transitions
        .iter()
        .map(|transition| transition.transition_id.as_str())
        .collect();
    for transition_ref in &contract.lifecycle_transition_refs {
        if !transition_ids.contains(transition_ref.as_str()) {
            errors.push(error(
                transition_refs_field(field_prefix),
                "contract references an unknown lifecycle transition",
            ));
        }
    }
}

fn validate_source_imports(
    errors: &mut Vec<TaskContractLifecycleValidationError>,
    imports: &[ContractSourceImportV1],
) {
    let mut import_ids = HashSet::new();
    for import in imports {
        if !import_ids.insert(import.import_id.as_str()) {
            errors.push(error(
                "source_imports.import_id",
                "source import ids must be unique",
            ));
        }
        require_non_empty(errors, "source_imports.import_id", &import.import_id);
        require_non_empty(errors, "source_imports.source_ref", &import.source_ref);
        require_non_empty(
            errors,
            "source_imports.source_contract_id",
            &import.source_contract_id,
        );
        if !import.required {
            errors.push(error(
                "source_imports.required",
                "contract lifecycle source imports must fail closed by default",
            ));
        }
        validate_hash(
            errors,
            "source_imports.provenance_hash",
            &import.provenance_hash,
        );
    }
}

fn validate_transitions(
    lifecycle: &TaskContractLifecycleV1,
    errors: &mut Vec<TaskContractLifecycleValidationError>,
) {
    for transition in &lifecycle.transitions {
        require_non_empty(
            errors,
            "transitions.transition_id",
            &transition.transition_id,
        );
        require_non_empty(errors, "transitions.action_id", &transition.action_id);
        require_non_empty(
            errors,
            "transitions.receipt_event",
            &transition.receipt_event,
        );
        require_non_empty(
            errors,
            "transitions.projection_hook_id",
            &transition.projection_hook_id,
        );
        require_non_empty(
            errors,
            "transitions.validation_hook_id",
            &transition.validation_hook_id,
        );
        if transition.from == transition.to {
            errors.push(error(
                "transitions",
                "lifecycle transition must move to a different state",
            ));
        }
        if !lifecycle
            .receipt_events
            .iter()
            .any(|event| event == &transition.receipt_event)
        {
            errors.push(error(
                "transitions.receipt_event",
                "transition receipt event must be registered",
            ));
        }
        if !lifecycle
            .projection_hooks
            .iter()
            .any(|hook| hook == &transition.projection_hook_id)
        {
            errors.push(error(
                "transitions.projection_hook_id",
                "transition projection hook must be registered",
            ));
        }
        if !lifecycle
            .validation_hooks
            .iter()
            .any(|hook| hook == &transition.validation_hook_id)
        {
            errors.push(error(
                "transitions.validation_hook_id",
                "transition validation hook must be registered",
            ));
        }
    }

    for (from, to, action_id) in required_transitions() {
        if !lifecycle.transitions.iter().any(|transition| {
            transition.from == from && transition.to == to && transition.action_id == action_id
        }) {
            errors.push(error(
                "transitions",
                "required contract lifecycle transition is missing",
            ));
        }
    }
}

fn validate_failure_states(
    errors: &mut Vec<TaskContractLifecycleValidationError>,
    field: &'static str,
    states: &[ContractFailureState],
) {
    for required in [
        ContractFailureState::MissingRequiredField,
        ContractFailureState::SourceHashMismatch,
        ContractFailureState::ProjectionDrift,
        ContractFailureState::InvalidTransition,
        ContractFailureState::ReceiptMissing,
        ContractFailureState::ValidationFailed,
        ContractFailureState::AuthorityBoundaryViolation,
    ] {
        if !states.contains(&required) {
            errors.push(error(
                field,
                "contract lifecycle is missing required failure state",
            ));
        }
    }
}

fn source_imports() -> Vec<ContractSourceImportV1> {
    vec![
        source_import(
            "source-import-stub-template",
            ".GOV/templates/TASK_PACKET_STUB_TEMPLATE.md",
            "template.task_packet_stub",
            "7a4e3b5130202b5c",
        ),
        source_import(
            "source-import-packet-contract",
            ".GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json",
            "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
            "744309d8eba07f2e",
        ),
        source_import(
            "source-import-mt051-contract",
            ".GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-051.json",
            "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-051",
            "d5ee0b452f6e9ee5",
        ),
    ]
}

fn source_import(
    import_id: &str,
    source_ref: &str,
    source_contract_id: &str,
    hash_value: &str,
) -> ContractSourceImportV1 {
    ContractSourceImportV1 {
        import_id: import_id.to_string(),
        source_ref: source_ref.to_string(),
        source_contract_id: source_contract_id.to_string(),
        required: true,
        provenance_hash: ProvenanceHashV1 {
            hash_kind: "blake3-16".to_string(),
            hash_value: hash_value.to_string(),
        },
    }
}

fn transitions() -> Vec<ContractLifecycleTransitionV1> {
    vec![
        transition(
            "transition-stub-promote-ready",
            ContractLifecycleState::StubInactive,
            ContractLifecycleState::StubReadyForPromotion,
            "kernel.stub_contract.prepare_promotion",
            "STATUS",
            "markdown_projection",
            "required_fields_present",
        ),
        transition(
            "transition-wp-activate",
            ContractLifecycleState::StubReadyForPromotion,
            ContractLifecycleState::WorkPacketActive,
            "kernel.work_packet_contract.activate",
            "STATUS",
            "task_board_projection",
            "source_hash_match",
        ),
        transition(
            "transition-mt-extract",
            ContractLifecycleState::WorkPacketActive,
            ContractLifecycleState::MicroTaskGenerated,
            "kernel.microtask_contract.extract",
            "STATUS",
            "markdown_projection",
            "mt_contract_generation",
        ),
        transition(
            "transition-mt-publish",
            ContractLifecycleState::MicroTaskGenerated,
            ContractLifecycleState::MicroTaskClaimable,
            "kernel.microtask_contract.publish",
            "STATUS",
            "task_board_projection",
            "projection_hash_match",
        ),
        transition(
            "transition-mt-claim",
            ContractLifecycleState::MicroTaskClaimable,
            ContractLifecycleState::MicroTaskClaimed,
            "kernel.microtask_contract.claim",
            "CLAIM",
            "dcc_work_view",
            "claim_lease_available",
        ),
        transition(
            "transition-mt-review-request",
            ContractLifecycleState::MicroTaskClaimed,
            ContractLifecycleState::MicroTaskValidationRequested,
            "kernel.microtask_contract.request_review",
            "CODER_HANDOFF",
            "traceability_registry_projection",
            "receipt_event_recorded",
        ),
        transition(
            "transition-mt-review-start",
            ContractLifecycleState::MicroTaskValidationRequested,
            ContractLifecycleState::MicroTaskInReview,
            "kernel.validator_contract.review_start",
            "REVIEW_REQUEST",
            "traceability_registry_projection",
            "receipt_event_recorded",
        ),
        transition(
            "transition-mt-repair-required",
            ContractLifecycleState::MicroTaskInReview,
            ContractLifecycleState::MicroTaskRepairRequired,
            "kernel.validator_contract.request_repair",
            "VALIDATOR_REVIEW",
            "failure_projection",
            "failure_state_recorded",
        ),
        transition(
            "transition-mt-clear",
            ContractLifecycleState::MicroTaskInReview,
            ContractLifecycleState::MicroTaskCleared,
            "kernel.validator_contract.clear",
            "REVIEW_RESPONSE",
            "traceability_registry_projection",
            "receipt_event_recorded",
        ),
        transition(
            "transition-fail-closed",
            ContractLifecycleState::WorkPacketActive,
            ContractLifecycleState::Failed,
            "kernel.contract_lifecycle.fail_closed",
            "BLOCKER",
            "failure_projection",
            "failure_state_recorded",
        ),
    ]
}

fn transition(
    transition_id: &str,
    from: ContractLifecycleState,
    to: ContractLifecycleState,
    action_id: &str,
    receipt_event: &str,
    projection_hook_id: &str,
    validation_hook_id: &str,
) -> ContractLifecycleTransitionV1 {
    ContractLifecycleTransitionV1 {
        transition_id: transition_id.to_string(),
        from,
        to,
        action_id: action_id.to_string(),
        receipt_event: receipt_event.to_string(),
        projection_hook_id: projection_hook_id.to_string(),
        validation_hook_id: validation_hook_id.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn contract_schema(
    schema_id: &'static str,
    contract_kind: ContractKind,
    contract_id: &str,
    lifecycle_state: ContractLifecycleState,
    required_fields: &[&str],
    source_import_refs: &[&str],
    lifecycle_transition_refs: &[&str],
    receipt_events: &[String],
    projection_hooks: &[String],
    validation_hooks: &[String],
    failure_states: &[ContractFailureState],
) -> ContractSchemaDefinitionV1 {
    ContractSchemaDefinitionV1 {
        schema_id,
        contract_kind,
        contract_id: contract_id.to_string(),
        lifecycle_state,
        authority_rules: required_authority_rules().to_vec(),
        required_fields: required_fields
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
        provenance_hashes: vec![ProvenanceHashV1 {
            hash_kind: "blake3-16".to_string(),
            hash_value: "744309d8eba07f2e".to_string(),
        }],
        source_import_refs: source_import_refs
            .iter()
            .map(|source| (*source).to_string())
            .collect(),
        lifecycle_transition_refs: lifecycle_transition_refs
            .iter()
            .map(|transition| (*transition).to_string())
            .collect(),
        receipt_events: receipt_events.to_vec(),
        projection_hooks: projection_hooks.to_vec(),
        validation_hooks: validation_hooks.to_vec(),
        failure_states: failure_states.to_vec(),
    }
}

fn required_authority_rules() -> [ContractAuthorityRule; 7] {
    [
        ContractAuthorityRule::MachineContractAuthority,
        ContractAuthorityRule::GeneratedProjectionOnly,
        ContractAuthorityRule::DirectStatusEditsDenied,
        ContractAuthorityRule::ProvenanceHashRequired,
        ContractAuthorityRule::PromotionGateRequired,
        ContractAuthorityRule::ReceiptBackedLifecycle,
        ContractAuthorityRule::ValidationHookRequired,
    ]
}

fn required_stub_fields() -> [&'static str; 19] {
    [
        "stub_id",
        "contract_id",
        "schema_version",
        "contract_authority",
        "artifact_policy",
        "execution_authority",
        "intent_summary",
        "source_hash",
        "source_files",
        "promotion_target",
        "markdown_projection",
        "build_order",
        "spec_trace",
        "session_policy",
        "draft_scope",
        "activation_contract",
        "red_team",
        "authority_rules",
        "lifecycle",
    ]
}

fn required_work_packet_fields() -> [&'static str; 22] {
    [
        "wp_id",
        "contract_id",
        "schema_version",
        "contract_authority",
        "artifact_policy",
        "base_wp_id",
        "created_at_utc",
        "updated_at_utc",
        "source_control",
        "workflow",
        "authority_files",
        "markdown_projection",
        "scope",
        "acceptance_criteria",
        "mt_source_plan",
        "source_imports",
        "proof_targets",
        "refinement",
        "microtasks",
        "role_profiles",
        "red_team",
        "lifecycle",
    ]
}

fn required_microtask_fields() -> [&'static str; 17] {
    [
        "mt_id",
        "wp_id",
        "contract_id",
        "schema_version",
        "contract_authority",
        "artifact_policy",
        "created_at_utc",
        "updated_at_utc",
        "authority_files",
        "markdown_projection",
        "scope",
        "dependencies",
        "allowed_paths",
        "proof_targets",
        "handoff",
        "red_team",
        "lifecycle",
    ]
}

fn required_transitions() -> [(ContractLifecycleState, ContractLifecycleState, &'static str); 7] {
    [
        (
            ContractLifecycleState::StubInactive,
            ContractLifecycleState::StubReadyForPromotion,
            "kernel.stub_contract.prepare_promotion",
        ),
        (
            ContractLifecycleState::StubReadyForPromotion,
            ContractLifecycleState::WorkPacketActive,
            "kernel.work_packet_contract.activate",
        ),
        (
            ContractLifecycleState::WorkPacketActive,
            ContractLifecycleState::MicroTaskGenerated,
            "kernel.microtask_contract.extract",
        ),
        (
            ContractLifecycleState::MicroTaskGenerated,
            ContractLifecycleState::MicroTaskClaimable,
            "kernel.microtask_contract.publish",
        ),
        (
            ContractLifecycleState::MicroTaskClaimable,
            ContractLifecycleState::MicroTaskClaimed,
            "kernel.microtask_contract.claim",
        ),
        (
            ContractLifecycleState::MicroTaskClaimed,
            ContractLifecycleState::MicroTaskValidationRequested,
            "kernel.microtask_contract.request_review",
        ),
        (
            ContractLifecycleState::MicroTaskValidationRequested,
            ContractLifecycleState::MicroTaskInReview,
            "kernel.validator_contract.review_start",
        ),
    ]
}

fn validate_hash(
    errors: &mut Vec<TaskContractLifecycleValidationError>,
    field: &'static str,
    hash: &ProvenanceHashV1,
) {
    require_non_empty(errors, field, &hash.hash_kind);
    require_non_empty(errors, field, &hash.hash_value);
    if hash.hash_value.len() < 16 {
        errors.push(error(
            field,
            "provenance hash must include at least a 16-character digest",
        ));
    }
}

fn schema_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.schema_id",
        "work_packet_contract" => "work_packet_contract.schema_id",
        "micro_task_contracts" => "micro_task_contracts.schema_id",
        _ => "contract.schema_id",
    }
}

fn contract_kind_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.contract_kind",
        "work_packet_contract" => "work_packet_contract.contract_kind",
        "micro_task_contracts" => "micro_task_contracts.contract_kind",
        _ => "contract.contract_kind",
    }
}

fn authority_rule_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.authority_rules",
        "work_packet_contract" => "work_packet_contract.authority_rules",
        "micro_task_contracts" => "micro_task_contracts.authority_rules",
        _ => "contract.authority_rules",
    }
}

fn required_fields_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.required_fields",
        "work_packet_contract" => "work_packet_contract.required_fields",
        "micro_task_contracts" => "micro_task_contracts.required_fields",
        _ => "contract.required_fields",
    }
}

fn provenance_hashes_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.provenance_hashes",
        "work_packet_contract" => "work_packet_contract.provenance_hashes",
        "micro_task_contracts" => "micro_task_contracts.provenance_hashes",
        _ => "contract.provenance_hashes",
    }
}

fn source_import_refs_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.source_import_refs",
        "work_packet_contract" => "work_packet_contract.source_import_refs",
        "micro_task_contracts" => "micro_task_contracts.source_import_refs",
        _ => "contract.source_import_refs",
    }
}

fn transition_refs_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.lifecycle_transition_refs",
        "work_packet_contract" => "work_packet_contract.lifecycle_transition_refs",
        "micro_task_contracts" => "micro_task_contracts.lifecycle_transition_refs",
        _ => "contract.lifecycle_transition_refs",
    }
}

fn receipt_events_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.receipt_events",
        "work_packet_contract" => "work_packet_contract.receipt_events",
        "micro_task_contracts" => "micro_task_contracts.receipt_events",
        _ => "contract.receipt_events",
    }
}

fn projection_hooks_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.projection_hooks",
        "work_packet_contract" => "work_packet_contract.projection_hooks",
        "micro_task_contracts" => "micro_task_contracts.projection_hooks",
        _ => "contract.projection_hooks",
    }
}

fn validation_hooks_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.validation_hooks",
        "work_packet_contract" => "work_packet_contract.validation_hooks",
        "micro_task_contracts" => "micro_task_contracts.validation_hooks",
        _ => "contract.validation_hooks",
    }
}

fn failure_states_field(field_prefix: &'static str) -> &'static str {
    match field_prefix {
        "stub_contract" => "stub_contract.failure_states",
        "work_packet_contract" => "work_packet_contract.failure_states",
        "micro_task_contracts" => "micro_task_contracts.failure_states",
        _ => "contract.failure_states",
    }
}

fn require_non_empty(
    errors: &mut Vec<TaskContractLifecycleValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(error(field, "value must not be empty"));
    }
}

fn require_vec<T>(
    errors: &mut Vec<TaskContractLifecycleValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(error(field, "at least one value is required"));
    }
}

fn error(field: &'static str, message: &'static str) -> TaskContractLifecycleValidationError {
    TaskContractLifecycleValidationError { field, message }
}

use serde::{Deserialize, Serialize};

use super::crdt::persistence::sha256_hex;
use std::collections::{HashMap, HashSet};

pub const GENERATED_DOCUMENTATION_STATUS_PROJECTION_SCHEMA_ID: &str =
    "hsk.kernel.generated_documentation_status_projection@1";
pub const GENERATED_DOCUMENTATION_STATUS_PROJECTION_RESULT_SCHEMA_ID: &str =
    "hsk.kernel.generated_documentation_status_projection_result@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeneratedStatusSourceKind {
    Contract,
    ReceiptLedger,
    RuntimeState,
    ValidationOutput,
    PacketProse,
    MailboxChronology,
    MarkdownFreshness,
}

impl GeneratedStatusSourceKind {
    fn is_authoritative(self) -> bool {
        matches!(
            self,
            GeneratedStatusSourceKind::Contract
                | GeneratedStatusSourceKind::ReceiptLedger
                | GeneratedStatusSourceKind::RuntimeState
                | GeneratedStatusSourceKind::ValidationOutput
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeneratedStatusTargetKind {
    PacketStatus,
    MicroTaskStatus,
    TaskBoardRow,
    TraceabilityRow,
    DccWorkView,
    MirrorDoc,
    OperatorSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManualStatusEditDisposition {
    DenyDirectEdit,
    CaptureAsAdvisoryNormalization,
    AcceptAsAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeneratedStatusFailureState {
    ManualStatusEditAttempt,
    NonAuthoritativeSourceUsed,
    StaleGeneratedDocument,
    MissingReceipt,
    MissingRuntimeState,
    MissingValidationOutput,
    ProjectionHashDrift,
    DirectTaskBoardMutation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedStatusSourceRefV1 {
    pub source_id: String,
    pub kind: GeneratedStatusSourceKind,
    pub source_ref: String,
    pub schema_id: String,
    pub source_hash: String,
    pub authoritative: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedStatusTargetV1 {
    pub target_id: String,
    pub kind: GeneratedStatusTargetKind,
    pub target_ref: String,
    pub output_schema_id: String,
    pub source_ids: Vec<String>,
    pub generation_hook_id: String,
    pub projection_hash_ref: String,
    pub generated_from_machine_authority: bool,
    pub authority_mutation: bool,
    pub manual_edit_disposition: ManualStatusEditDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedStatusRegenerationContractV1 {
    pub deterministic_regeneration: bool,
    pub machine_readable_authority_only: bool,
    pub generated_docs_are_authority: bool,
    pub manual_status_edits_are_authority: bool,
    pub direct_edit_denial_action_id: String,
    pub advisory_capture_action_id: String,
    pub advisory_normalize_action_id: String,
    pub denied_source_kinds: Vec<GeneratedStatusSourceKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedStatusResearchBasisV1 {
    pub source_ref: String,
    pub pattern_found: String,
    pub selected_reuse: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDocumentationStatusProjectionV1 {
    pub schema_id: String,
    pub contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub sources: Vec<GeneratedStatusSourceRefV1>,
    pub targets: Vec<GeneratedStatusTargetV1>,
    pub regeneration_contract: GeneratedStatusRegenerationContractV1,
    pub product_authority_refs: Vec<String>,
    pub failure_states: Vec<GeneratedStatusFailureState>,
    pub research_basis_refs: Vec<GeneratedStatusResearchBasisV1>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDocumentationStatusProjectionResultV1 {
    pub schema_id: String,
    pub contract_id: String,
    pub wp_id: String,
    pub source_kinds: Vec<GeneratedStatusSourceKind>,
    pub target_kinds: Vec<GeneratedStatusTargetKind>,
    pub target_refs: Vec<String>,
    pub source_lineage_refs: Vec<String>,
    pub allowed_advisory_action_ids: Vec<String>,
    pub direct_edit_denial_action_id: String,
    pub generated_docs_are_authority: bool,
    pub manual_status_edits_are_authority: bool,
    pub mutates_task_board: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDocumentationStatusProjectionValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_kernel002_generated_documentation_status_projection(
) -> GeneratedDocumentationStatusProjectionV1 {
    let source_ids = vec![
        "wp-contract-kernel002".to_string(),
        "mt-contract-mt055".to_string(),
        "receipt-ledger-mt055".to_string(),
        "runtime-state-kernel002".to_string(),
        "validation-output-mt055".to_string(),
    ];

    GeneratedDocumentationStatusProjectionV1 {
        schema_id: GENERATED_DOCUMENTATION_STATUS_PROJECTION_SCHEMA_ID.to_string(),
        contract_id: "kernel002-generated-documentation-status-projection-mt055".to_string(),
        wp_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
        mt_id: "MT-055".to_string(),
        sources: vec![
            source(
                "wp-contract-kernel002",
                GeneratedStatusSourceKind::Contract,
                ".GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json",
                "hsk.work_packet_contract@1",
            ),
            source(
                "mt-contract-mt055",
                GeneratedStatusSourceKind::Contract,
                ".GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-055.json",
                "hsk.microtask_contract@1",
            ),
            source(
                "receipt-ledger-mt055",
                GeneratedStatusSourceKind::ReceiptLedger,
                "receipt-ledger://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-055",
                "hsk.role_receipt@1",
            ),
            source(
                "runtime-state-kernel002",
                GeneratedStatusSourceKind::RuntimeState,
                "runtime-state://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
                "hsk.kernel.software_delivery_runtime_truth_record@1",
            ),
            source(
                "validation-output-mt055",
                GeneratedStatusSourceKind::ValidationOutput,
                "validation-output://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-055",
                "hsk.validation_output@1",
            ),
        ],
        targets: vec![
            target(
                "packet-status-projection",
                GeneratedStatusTargetKind::PacketStatus,
                ".GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.md#status",
                "hsk.generated_packet_status_projection@1",
                &source_ids,
                ManualStatusEditDisposition::DenyDirectEdit,
            ),
            target(
                "microtask-status-projection",
                GeneratedStatusTargetKind::MicroTaskStatus,
                ".GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-055.md#status",
                "hsk.generated_microtask_status_projection@1",
                &source_ids,
                ManualStatusEditDisposition::DenyDirectEdit,
            ),
            target(
                "task-board-row-projection",
                GeneratedStatusTargetKind::TaskBoardRow,
                ".GOV/roles_shared/records/TASK_BOARD.md#WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
                "hsk.generated_task_board_row_projection@1",
                &source_ids,
                ManualStatusEditDisposition::DenyDirectEdit,
            ),
            target(
                "traceability-row-projection",
                GeneratedStatusTargetKind::TraceabilityRow,
                ".GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md#WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
                "hsk.generated_traceability_row_projection@1",
                &source_ids,
                ManualStatusEditDisposition::DenyDirectEdit,
            ),
            target(
                "dcc-work-view-projection",
                GeneratedStatusTargetKind::DccWorkView,
                "dcc://work-view/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
                "hsk.generated_dcc_work_view_projection@1",
                &source_ids,
                ManualStatusEditDisposition::DenyDirectEdit,
            ),
            target(
                "mirror-doc-projection",
                GeneratedStatusTargetKind::MirrorDoc,
                ".GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/generated-mirrors",
                "hsk.generated_markdown_mirror_projection@1",
                &source_ids,
                ManualStatusEditDisposition::CaptureAsAdvisoryNormalization,
            ),
            target(
                "operator-summary-projection",
                GeneratedStatusTargetKind::OperatorSummary,
                "operator-summary://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
                "hsk.generated_operator_summary_projection@1",
                &source_ids,
                ManualStatusEditDisposition::CaptureAsAdvisoryNormalization,
            ),
        ],
        regeneration_contract: GeneratedStatusRegenerationContractV1 {
            deterministic_regeneration: true,
            machine_readable_authority_only: true,
            generated_docs_are_authority: false,
            manual_status_edits_are_authority: false,
            direct_edit_denial_action_id: "kernel.direct_edit.deny".to_string(),
            advisory_capture_action_id: "kernel.mirror_advisory.capture".to_string(),
            advisory_normalize_action_id: "kernel.mirror_advisory.normalize".to_string(),
            denied_source_kinds: vec![
                GeneratedStatusSourceKind::PacketProse,
                GeneratedStatusSourceKind::MailboxChronology,
                GeneratedStatusSourceKind::MarkdownFreshness,
            ],
        },
        product_authority_refs: required_authority_refs()
            .iter()
            .map(|authority_ref| (*authority_ref).to_string())
            .collect(),
        failure_states: required_failure_states(),
        research_basis_refs: vec![
            research(
                "https://kpt.dev/reference/schema/crd-status-convention/",
                "Controller status is derived from observed state and reconcile conditions, not prose.",
                "Use runtime state plus validation output as status projection inputs.",
            ),
            research(
                "https://backstage.io/docs/features/techdocs/architecture/",
                "Production docs are generated in CI and served from generated artifacts/storage.",
                "Treat Markdown/operator docs as generated views over contracts.",
            ),
            research(
                "https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands",
                "Job summaries aggregate run output for readers while run status remains separately governed.",
                "Generate operator summaries from receipts and validation outputs.",
            ),
            research(
                "https://opentelemetry.io/docs/specs/semconv/",
                "Stable semantic attributes improve correlation across signals and consumers.",
                "Use stable source and target kinds for DCC, task board, and summaries.",
            ),
        ],
        folded_source_refs: vec![
            "MT-055 Generated Documentation and Status Projection".to_string(),
            "packet-runtime-projection-lib.mjs".to_string(),
            "wp-execution-state-lib.mjs".to_string(),
            "active-lane-brief-lib.mjs".to_string(),
        ],
    }
}

pub fn validate_generated_documentation_status_projection(
    contract: &GeneratedDocumentationStatusProjectionV1,
) -> Result<(), Vec<GeneratedDocumentationStatusProjectionValidationError>> {
    let mut errors = Vec::new();

    if contract.schema_id != GENERATED_DOCUMENTATION_STATUS_PROJECTION_SCHEMA_ID {
        errors.push(error(
            "schema_id",
            "generated documentation/status projection schema id is required",
        ));
    }
    require_non_empty(&mut errors, "contract_id", &contract.contract_id);
    require_non_empty(&mut errors, "wp_id", &contract.wp_id);
    require_non_empty(&mut errors, "mt_id", &contract.mt_id);
    require_vec(&mut errors, "sources", &contract.sources);
    require_vec(&mut errors, "targets", &contract.targets);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &contract.product_authority_refs,
    );
    require_vec(&mut errors, "failure_states", &contract.failure_states);
    require_vec(
        &mut errors,
        "research_basis_refs",
        &contract.research_basis_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &contract.folded_source_refs,
    );

    validate_sources(&mut errors, &contract.sources);
    validate_targets(&mut errors, &contract.sources, &contract.targets);
    validate_regeneration(&mut errors, &contract.regeneration_contract);
    validate_authority_refs(&mut errors, &contract.product_authority_refs);
    validate_failure_states(&mut errors, &contract.failure_states);
    validate_research_basis(&mut errors, &contract.research_basis_refs);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_generated_documentation_status(
    contract: &GeneratedDocumentationStatusProjectionV1,
) -> Result<
    GeneratedDocumentationStatusProjectionResultV1,
    Vec<GeneratedDocumentationStatusProjectionValidationError>,
> {
    validate_generated_documentation_status_projection(contract)?;

    Ok(GeneratedDocumentationStatusProjectionResultV1 {
        schema_id: GENERATED_DOCUMENTATION_STATUS_PROJECTION_RESULT_SCHEMA_ID.to_string(),
        contract_id: contract.contract_id.clone(),
        wp_id: contract.wp_id.clone(),
        source_kinds: required_source_kinds(),
        target_kinds: required_target_kinds(),
        target_refs: contract
            .targets
            .iter()
            .map(|target| target.target_ref.clone())
            .collect(),
        source_lineage_refs: contract
            .sources
            .iter()
            .map(|source| format!("{}#{}", source.source_ref, source.source_hash))
            .collect(),
        allowed_advisory_action_ids: vec![
            contract
                .regeneration_contract
                .advisory_capture_action_id
                .clone(),
            contract
                .regeneration_contract
                .advisory_normalize_action_id
                .clone(),
        ],
        direct_edit_denial_action_id: contract
            .regeneration_contract
            .direct_edit_denial_action_id
            .clone(),
        generated_docs_are_authority: contract.regeneration_contract.generated_docs_are_authority,
        manual_status_edits_are_authority: contract
            .regeneration_contract
            .manual_status_edits_are_authority,
        mutates_task_board: contract.targets.iter().any(|target| {
            target.kind == GeneratedStatusTargetKind::TaskBoardRow && target.authority_mutation
        }),
    })
}

fn validate_sources(
    errors: &mut Vec<GeneratedDocumentationStatusProjectionValidationError>,
    sources: &[GeneratedStatusSourceRefV1],
) {
    let mut seen = HashSet::new();
    for source in sources {
        require_non_empty(errors, "sources.source_id", &source.source_id);
        require_non_empty(errors, "sources.source_ref", &source.source_ref);
        require_non_empty(errors, "sources.schema_id", &source.schema_id);
        require_non_empty(errors, "sources.source_hash", &source.source_hash);
        if !seen.insert(source.source_id.as_str()) {
            errors.push(error("sources.source_id", "source ids must be unique"));
        }
        if !source.kind.is_authoritative() {
            errors.push(error(
                "sources.kind",
                "status projections cannot use prose, chronology, or freshness as authority",
            ));
        }
        if !source.authoritative {
            errors.push(error(
                "sources.authoritative",
                "declared projection sources must be authoritative",
            ));
        }
    }

    for required in required_source_kinds() {
        if !sources.iter().any(|source| source.kind == required) {
            errors.push(error(
                "sources",
                "contracts, receipts, runtime state, and validation outputs are all required",
            ));
        }
    }
}

fn validate_targets(
    errors: &mut Vec<GeneratedDocumentationStatusProjectionValidationError>,
    sources: &[GeneratedStatusSourceRefV1],
    targets: &[GeneratedStatusTargetV1],
) {
    let source_by_id: HashMap<&str, GeneratedStatusSourceKind> = sources
        .iter()
        .map(|source| (source.source_id.as_str(), source.kind))
        .collect();
    let mut seen = HashSet::new();

    for target in targets {
        require_non_empty(errors, "targets.target_id", &target.target_id);
        require_non_empty(errors, "targets.target_ref", &target.target_ref);
        require_non_empty(errors, "targets.output_schema_id", &target.output_schema_id);
        require_non_empty(
            errors,
            "targets.generation_hook_id",
            &target.generation_hook_id,
        );
        require_non_empty(
            errors,
            "targets.projection_hash_ref",
            &target.projection_hash_ref,
        );
        require_vec(errors, "targets.source_ids", &target.source_ids);
        if !seen.insert(target.target_id.as_str()) {
            errors.push(error("targets.target_id", "target ids must be unique"));
        }
        if !target.generated_from_machine_authority {
            errors.push(error(
                "targets.generated_from_machine_authority",
                "targets must derive from machine-readable authority",
            ));
        }
        if target.authority_mutation {
            errors.push(error(
                "targets.authority_mutation",
                "generated targets are projections and must not mutate authority",
            ));
        }
        if target.manual_edit_disposition == ManualStatusEditDisposition::AcceptAsAuthority {
            errors.push(error(
                "targets.manual_edit_disposition",
                "manual status edits must be denied or captured as advisory normalization",
            ));
        }

        if target
            .source_ids
            .iter()
            .any(|source_id| !source_by_id.contains_key(source_id.as_str()))
        {
            errors.push(error(
                "targets.source_ids",
                "target references an unknown source id",
            ));
        }
        let target_source_kinds: HashSet<_> = target
            .source_ids
            .iter()
            .filter_map(|source_id| source_by_id.get(source_id.as_str()).copied())
            .collect();
        for required in required_source_kinds() {
            if !target_source_kinds.contains(&required) {
                errors.push(error(
                    "targets.source_ids",
                    "each target must derive from contract, receipt, runtime, and validation sources",
                ));
            }
        }
    }

    for required in required_target_kinds() {
        if !targets.iter().any(|target| target.kind == required) {
            errors.push(error(
                "targets",
                "packet, microtask, board, traceability, DCC, mirror, and operator summary targets are required",
            ));
        }
    }
}

fn validate_regeneration(
    errors: &mut Vec<GeneratedDocumentationStatusProjectionValidationError>,
    regeneration: &GeneratedStatusRegenerationContractV1,
) {
    if !regeneration.deterministic_regeneration {
        errors.push(error(
            "regeneration_contract.deterministic_regeneration",
            "regeneration must be deterministic",
        ));
    }
    if !regeneration.machine_readable_authority_only {
        errors.push(error(
            "regeneration_contract.machine_readable_authority_only",
            "projection inputs must be machine-readable authority only",
        ));
    }
    if regeneration.generated_docs_are_authority {
        errors.push(error(
            "regeneration_contract.generated_docs_are_authority",
            "generated docs cannot be authority",
        ));
    }
    if regeneration.manual_status_edits_are_authority {
        errors.push(error(
            "regeneration_contract.manual_status_edits_are_authority",
            "manual edits cannot be authority",
        ));
    }
    if regeneration.direct_edit_denial_action_id != "kernel.direct_edit.deny" {
        errors.push(error(
            "regeneration_contract.direct_edit_denial_action_id",
            "direct edits must route through kernel.direct_edit.deny",
        ));
    }
    for denied in [
        GeneratedStatusSourceKind::PacketProse,
        GeneratedStatusSourceKind::MailboxChronology,
        GeneratedStatusSourceKind::MarkdownFreshness,
    ] {
        if !regeneration.denied_source_kinds.contains(&denied) {
            errors.push(error(
                "regeneration_contract.denied_source_kinds",
                "non-authority prose/chonology/freshness sources must be denied",
            ));
        }
    }
}

fn validate_authority_refs(
    errors: &mut Vec<GeneratedDocumentationStatusProjectionValidationError>,
    authority_refs: &[String],
) {
    for required in required_authority_refs() {
        if !authority_refs
            .iter()
            .any(|authority_ref| authority_ref == required)
        {
            errors.push(error(
                "product_authority_refs",
                "existing contract, runtime, mirror, DCC, direct-edit, and catalog authorities must be cited",
            ));
        }
    }
}

fn validate_failure_states(
    errors: &mut Vec<GeneratedDocumentationStatusProjectionValidationError>,
    failure_states: &[GeneratedStatusFailureState],
) {
    for required in required_failure_states() {
        if !failure_states.contains(&required) {
            errors.push(error(
                "failure_states",
                "manual edit, source loss, stale doc, drift, and direct board mutation states are required",
            ));
        }
    }
}

fn validate_research_basis(
    errors: &mut Vec<GeneratedDocumentationStatusProjectionValidationError>,
    research_basis_refs: &[GeneratedStatusResearchBasisV1],
) {
    for required in [
        "kpt.dev/reference/schema/crd-status-convention",
        "backstage.io/docs/features/techdocs/architecture",
        "docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands",
        "opentelemetry.io/docs/specs/semconv",
    ] {
        if !research_basis_refs
            .iter()
            .any(|basis| basis.source_ref.contains(required))
        {
            errors.push(error(
                "research_basis_refs",
                "external status/doc/summary/correlation basis must be recorded",
            ));
        }
    }
}

fn required_source_kinds() -> Vec<GeneratedStatusSourceKind> {
    vec![
        GeneratedStatusSourceKind::Contract,
        GeneratedStatusSourceKind::ReceiptLedger,
        GeneratedStatusSourceKind::RuntimeState,
        GeneratedStatusSourceKind::ValidationOutput,
    ]
}

fn required_target_kinds() -> Vec<GeneratedStatusTargetKind> {
    vec![
        GeneratedStatusTargetKind::PacketStatus,
        GeneratedStatusTargetKind::MicroTaskStatus,
        GeneratedStatusTargetKind::TaskBoardRow,
        GeneratedStatusTargetKind::TraceabilityRow,
        GeneratedStatusTargetKind::DccWorkView,
        GeneratedStatusTargetKind::MirrorDoc,
        GeneratedStatusTargetKind::OperatorSummary,
    ]
}

fn required_authority_refs() -> Vec<&'static str> {
    vec![
        "kernel.task_contract_lifecycle",
        "kernel.software_delivery_runtime_truth",
        "kernel.markdown_mirror_sync_drift_guard",
        "kernel.dcc_layout_projection_registry",
        "kernel.role_mailbox_triage_queue",
        "kernel.direct_edit_guard",
        "kernel.action_catalog",
    ]
}

fn required_failure_states() -> Vec<GeneratedStatusFailureState> {
    vec![
        GeneratedStatusFailureState::ManualStatusEditAttempt,
        GeneratedStatusFailureState::NonAuthoritativeSourceUsed,
        GeneratedStatusFailureState::StaleGeneratedDocument,
        GeneratedStatusFailureState::MissingReceipt,
        GeneratedStatusFailureState::MissingRuntimeState,
        GeneratedStatusFailureState::MissingValidationOutput,
        GeneratedStatusFailureState::ProjectionHashDrift,
        GeneratedStatusFailureState::DirectTaskBoardMutation,
    ]
}

fn source(
    source_id: &str,
    kind: GeneratedStatusSourceKind,
    source_ref: &str,
    schema_id: &str,
) -> GeneratedStatusSourceRefV1 {
    GeneratedStatusSourceRefV1 {
        source_id: source_id.to_string(),
        kind,
        source_ref: source_ref.to_string(),
        schema_id: schema_id.to_string(),
        source_hash: source_hash("kernel002-mt055", &[source_id, source_ref, schema_id]),
        authoritative: true,
    }
}

fn source_hash(domain: &str, parts: &[&str]) -> String {
    format!(
        "sha256:{}",
        sha256_hex(format!("{domain}|{}", parts.join("|")).as_bytes())
    )
}

fn target(
    target_id: &str,
    kind: GeneratedStatusTargetKind,
    target_ref: &str,
    output_schema_id: &str,
    source_ids: &[String],
    manual_edit_disposition: ManualStatusEditDisposition,
) -> GeneratedStatusTargetV1 {
    GeneratedStatusTargetV1 {
        target_id: target_id.to_string(),
        kind,
        target_ref: target_ref.to_string(),
        output_schema_id: output_schema_id.to_string(),
        source_ids: source_ids.to_vec(),
        generation_hook_id: format!("generated_status_{target_id}"),
        projection_hash_ref: format!("projection-hash://{target_id}"),
        generated_from_machine_authority: true,
        authority_mutation: false,
        manual_edit_disposition,
    }
}

fn research(
    source_ref: &str,
    pattern_found: &str,
    selected_reuse: &str,
) -> GeneratedStatusResearchBasisV1 {
    GeneratedStatusResearchBasisV1 {
        source_ref: source_ref.to_string(),
        pattern_found: pattern_found.to_string(),
        selected_reuse: selected_reuse.to_string(),
    }
}

fn error(
    field: &'static str,
    message: &'static str,
) -> GeneratedDocumentationStatusProjectionValidationError {
    GeneratedDocumentationStatusProjectionValidationError { field, message }
}

fn require_non_empty(
    errors: &mut Vec<GeneratedDocumentationStatusProjectionValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(error(field, "value must not be empty"));
    }
}

fn require_vec<T>(
    errors: &mut Vec<GeneratedDocumentationStatusProjectionValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(error(field, "at least one value is required"));
    }
}

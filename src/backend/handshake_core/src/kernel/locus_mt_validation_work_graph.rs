use serde::{Deserialize, Serialize};

use super::crdt::persistence::sha256_hex;
use super::validator_verdict_mediation_contract::{ValidatorVerdictKind, VerdictRoutingOutcome};

pub const LOCUS_MT_VALIDATION_WORK_GRAPH_SCHEMA_ID: &str =
    "hsk.kernel.locus_mt_validation_work_graph@1";
pub const LOCUS_MT_VALIDATION_WORK_GRAPH_PROJECTION_SCHEMA_ID: &str =
    "hsk.kernel.locus_mt_validation_work_graph_projection@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocusGraphNodeKind {
    MicroTask,
    ValidatorVerdict,
    RemediationWork,
    ActorLease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocusGraphNodeState {
    Ready,
    InProgress,
    Passed,
    Failed,
    Blocked,
    Escalated,
    Remediation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocusGraphEdgeKind {
    DependsOn,
    Blocks,
    Validates,
    Remediates,
    LeasedBy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocusGraphSourceKind {
    MachineContract,
    ProseReport,
    ChatMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocusGraphFailureState {
    MissingMtNodes,
    MissingValidatorVerdicts,
    MissingRemediationEdges,
    MissingBlockedEscalatedStates,
    MissingActorLeases,
    MissingPassFailHistory,
    ProseReportUsedAsTruth,
    ChatMessageUsedAsTruth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusMtGraphNodeV1 {
    pub node_id: String,
    pub mt_id: String,
    pub node_kind: LocusGraphNodeKind,
    pub state: LocusGraphNodeState,
    pub source_ref: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusWorkGraphEdgeV1 {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_kind: LocusGraphEdgeKind,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusValidatorVerdictGraphV1 {
    pub verdict_id: String,
    pub mt_id: String,
    pub verdict: ValidatorVerdictKind,
    pub routing_outcome: VerdictRoutingOutcome,
    pub evidence_refs: Vec<String>,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusActorLeaseGraphV1 {
    pub lease_id: String,
    pub mt_id: String,
    pub actor_session_id: String,
    pub active: bool,
    pub expired: bool,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusPassFailHistoryEntryV1 {
    pub history_id: String,
    pub mt_id: String,
    pub verdict_id: String,
    pub passed: bool,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusWorkGraphSourceRefV1 {
    pub source_ref: String,
    pub source_kind: LocusGraphSourceKind,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusWorkGraphResearchBasisV1 {
    pub source_ref: String,
    pub pattern_found: String,
    pub selected_reuse: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusMtValidationWorkGraphContractV1 {
    pub schema_id: String,
    pub contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub nodes: Vec<LocusMtGraphNodeV1>,
    pub dependency_edges: Vec<LocusWorkGraphEdgeV1>,
    pub remediation_edges: Vec<LocusWorkGraphEdgeV1>,
    pub verdicts: Vec<LocusValidatorVerdictGraphV1>,
    pub actor_leases: Vec<LocusActorLeaseGraphV1>,
    pub pass_fail_history: Vec<LocusPassFailHistoryEntryV1>,
    pub source_refs: Vec<LocusWorkGraphSourceRefV1>,
    pub failure_states: Vec<LocusGraphFailureState>,
    pub research_basis_refs: Vec<LocusWorkGraphResearchBasisV1>,
    pub prose_reports_authoritative: bool,
    pub chat_messages_authoritative: bool,
    pub status_mutation_allowed: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusMtValidationWorkGraphProjectionV1 {
    pub schema_id: String,
    pub source_contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub mt_node_ids: Vec<String>,
    pub verdict_count: usize,
    pub remediation_edge_count: usize,
    pub blocked_node_ids: Vec<String>,
    pub escalated_node_ids: Vec<String>,
    pub actor_lease_refs: Vec<String>,
    pub pass_fail_history_refs: Vec<String>,
    pub prose_source_allowed: bool,
    pub chat_source_allowed: bool,
    pub status_mutation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusMtValidationWorkGraphError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_kernel002_locus_mt_validation_work_graph() -> LocusMtValidationWorkGraphContractV1 {
    let wp_id = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";

    LocusMtValidationWorkGraphContractV1 {
        schema_id: LOCUS_MT_VALIDATION_WORK_GRAPH_SCHEMA_ID.to_string(),
        contract_id: "kernel002-locus-mt-validation-work-graph-mt061".to_string(),
        wp_id: wp_id.to_string(),
        mt_id: "MT-061".to_string(),
        nodes: vec![
            mt_node(
                "MT-057",
                LocusGraphNodeState::Escalated,
                "validator-verdict-contract://kernel002/MT-057",
            ),
            mt_node(
                "MT-058",
                LocusGraphNodeState::Blocked,
                "validator-finding-report-contract://kernel002/MT-058",
            ),
            mt_node(
                "MT-059",
                LocusGraphNodeState::Remediation,
                "remediation-work-generation-contract://kernel002/MT-059",
            ),
            mt_node(
                "MT-060",
                LocusGraphNodeState::Passed,
                "mt-loop-scheduler-contract://kernel002/MT-060",
            ),
            mt_node(
                "MT-061",
                LocusGraphNodeState::InProgress,
                "locus-mt-validation-work-graph-contract://kernel002/MT-061",
            ),
        ],
        dependency_edges: vec![
            edge(
                "edge-MT-059-depends-MT-058",
                "locus-mt-node-MT-059",
                "locus-mt-node-MT-058",
                LocusGraphEdgeKind::DependsOn,
                "microtask-contract://kernel002/MT-059",
            ),
            edge(
                "edge-MT-061-depends-MT-060",
                "locus-mt-node-MT-061",
                "locus-mt-node-MT-060",
                LocusGraphEdgeKind::DependsOn,
                "microtask-contract://kernel002/MT-061",
            ),
        ],
        remediation_edges: vec![
            edge(
                "edge-MT-059-remediates-MT-057",
                "locus-mt-node-MT-059",
                "locus-mt-node-MT-057",
                LocusGraphEdgeKind::Remediates,
                "remediation-work-generation-contract://kernel002/MT-059",
            ),
            edge(
                "edge-MT-059-remediates-MT-058",
                "locus-mt-node-MT-059",
                "locus-mt-node-MT-058",
                LocusGraphEdgeKind::Remediates,
                "validator-finding-report-contract://kernel002/MT-058",
            ),
        ],
        verdicts: vec![
            verdict(
                "verdict-MT-057-loopback",
                "MT-057",
                ValidatorVerdictKind::Fail,
                VerdictRoutingOutcome::MustLoopBack,
                "validator-verdict-contract://kernel002/MT-057",
            ),
            verdict(
                "verdict-MT-060-pass",
                "MT-060",
                ValidatorVerdictKind::Pass,
                VerdictRoutingOutcome::MayAdvance,
                "validator-verdict-contract://kernel002/MT-060",
            ),
        ],
        actor_leases: vec![
            LocusActorLeaseGraphV1 {
                lease_id: "claim-lease-kernel002-mt060-coder".to_string(),
                mt_id: "MT-060".to_string(),
                actor_session_id: "KERNEL_BUILDER-20260514-130219".to_string(),
                active: true,
                expired: false,
                source_ref: "role-mailbox-claim-lease://kernel002/MT-060/coder".to_string(),
            },
            LocusActorLeaseGraphV1 {
                lease_id: "claim-lease-kernel002-mt061-coder".to_string(),
                mt_id: "MT-061".to_string(),
                actor_session_id: "KERNEL_BUILDER-20260514-130219".to_string(),
                active: true,
                expired: false,
                source_ref: "role-mailbox-claim-lease://kernel002/MT-061/coder".to_string(),
            },
        ],
        pass_fail_history: vec![
            LocusPassFailHistoryEntryV1 {
                history_id: "history-MT-057-fail".to_string(),
                mt_id: "MT-057".to_string(),
                verdict_id: "verdict-MT-057-loopback".to_string(),
                passed: false,
                evidence_ref: "validator-verdict-contract://kernel002/MT-057".to_string(),
            },
            LocusPassFailHistoryEntryV1 {
                history_id: "history-MT-060-pass".to_string(),
                mt_id: "MT-060".to_string(),
                verdict_id: "verdict-MT-060-pass".to_string(),
                passed: true,
                evidence_ref: "validator-verdict-contract://kernel002/MT-060".to_string(),
            },
        ],
        source_refs: vec![
            machine_source("microtask-contract://kernel002/MT-057"),
            machine_source("microtask-contract://kernel002/MT-058"),
            machine_source("remediation-work-generation-contract://kernel002/MT-059"),
            machine_source("mt-loop-scheduler-contract://kernel002/MT-060"),
            machine_source("locus-mt-validation-work-graph-contract://kernel002/MT-061"),
        ],
        failure_states: vec![
            LocusGraphFailureState::MissingMtNodes,
            LocusGraphFailureState::MissingValidatorVerdicts,
            LocusGraphFailureState::MissingRemediationEdges,
            LocusGraphFailureState::MissingBlockedEscalatedStates,
            LocusGraphFailureState::MissingActorLeases,
            LocusGraphFailureState::MissingPassFailHistory,
            LocusGraphFailureState::ProseReportUsedAsTruth,
            LocusGraphFailureState::ChatMessageUsedAsTruth,
        ],
        research_basis_refs: vec![
            LocusWorkGraphResearchBasisV1 {
                source_ref: "https://docs.github.com/en/rest/issues/timeline".to_string(),
                pattern_found: "Issue timeline APIs preserve event history as typed entries instead of summary prose.".to_string(),
                selected_reuse: "Represent pass/fail history as typed graph events with evidence refs.".to_string(),
            },
            LocusWorkGraphResearchBasisV1 {
                source_ref: "https://docs.gitlab.com/user/work_items/linked_items/".to_string(),
                pattern_found: "Linked work items preserve blocks, blocked-by, and related-work relationships.".to_string(),
                selected_reuse: "Represent MT dependencies and remediation links as explicit graph edges.".to_string(),
            },
            LocusWorkGraphResearchBasisV1 {
                source_ref: "https://kubernetes.io/docs/concepts/overview/working-with-objects/owners-dependents/".to_string(),
                pattern_found: "Owner and dependent references make object relationships inspectable and enforceable.".to_string(),
                selected_reuse: "Bind MT graph nodes back to owner WP and dependent MT references.".to_string(),
            },
            LocusWorkGraphResearchBasisV1 {
                source_ref: "https://opentelemetry.io/docs/concepts/signals/traces/".to_string(),
                pattern_found: "Trace spans and links expose relationships across work without relying on narrative logs.".to_string(),
                selected_reuse: "Project validation-loop graph state from typed nodes and links.".to_string(),
            },
        ],
        prose_reports_authoritative: false,
        chat_messages_authoritative: false,
        status_mutation_allowed: false,
        product_authority_refs: vec![
            "kernel.validator_verdict_mediation_contract".to_string(),
            "kernel.validator_finding_report_contract".to_string(),
            "kernel.remediation_work_generation_contract".to_string(),
            "kernel.mt_loop_scheduler_contract".to_string(),
            "locus.types".to_string(),
        ],
        folded_source_refs: vec![
            "MT-057 validator verdict mediation".to_string(),
            "MT-058 validator finding reports".to_string(),
            "MT-059 remediation work generation".to_string(),
            "MT-060 loop scheduler".to_string(),
        ],
    }
}

pub fn project_locus_mt_validation_work_graph(
    contract: &LocusMtValidationWorkGraphContractV1,
) -> Result<LocusMtValidationWorkGraphProjectionV1, Vec<LocusMtValidationWorkGraphError>> {
    validate_locus_mt_validation_work_graph(contract)?;

    Ok(LocusMtValidationWorkGraphProjectionV1 {
        schema_id: LOCUS_MT_VALIDATION_WORK_GRAPH_PROJECTION_SCHEMA_ID.to_string(),
        source_contract_id: contract.contract_id.clone(),
        wp_id: contract.wp_id.clone(),
        mt_id: contract.mt_id.clone(),
        mt_node_ids: contract
            .nodes
            .iter()
            .filter(|node| node.node_kind == LocusGraphNodeKind::MicroTask)
            .map(|node| node.node_id.clone())
            .collect(),
        verdict_count: contract.verdicts.len(),
        remediation_edge_count: contract.remediation_edges.len(),
        blocked_node_ids: contract
            .nodes
            .iter()
            .filter(|node| node.state == LocusGraphNodeState::Blocked)
            .map(|node| node.node_id.clone())
            .collect(),
        escalated_node_ids: contract
            .nodes
            .iter()
            .filter(|node| node.state == LocusGraphNodeState::Escalated)
            .map(|node| node.node_id.clone())
            .collect(),
        actor_lease_refs: contract
            .actor_leases
            .iter()
            .map(|lease| lease.source_ref.clone())
            .collect(),
        pass_fail_history_refs: contract
            .pass_fail_history
            .iter()
            .map(|entry| entry.evidence_ref.clone())
            .collect(),
        prose_source_allowed: contract.prose_reports_authoritative,
        chat_source_allowed: contract.chat_messages_authoritative,
        status_mutation_allowed: contract.status_mutation_allowed,
    })
}

pub fn validate_locus_mt_validation_work_graph(
    contract: &LocusMtValidationWorkGraphContractV1,
) -> Result<(), Vec<LocusMtValidationWorkGraphError>> {
    let mut errors = Vec::new();

    if contract.schema_id != LOCUS_MT_VALIDATION_WORK_GRAPH_SCHEMA_ID {
        errors.push(error(
            "schema_id",
            "schema id must match Locus MT validation graph",
        ));
    }
    require_non_empty(&mut errors, "contract_id", &contract.contract_id);
    require_non_empty(&mut errors, "wp_id", &contract.wp_id);
    require_non_empty(&mut errors, "mt_id", &contract.mt_id);
    require_vec(&mut errors, "nodes", &contract.nodes);
    require_vec(&mut errors, "verdicts", &contract.verdicts);
    require_vec(
        &mut errors,
        "remediation_edges",
        &contract.remediation_edges,
    );
    require_vec(&mut errors, "actor_leases", &contract.actor_leases);
    require_vec(
        &mut errors,
        "pass_fail_history",
        &contract.pass_fail_history,
    );
    require_vec(&mut errors, "source_refs", &contract.source_refs);
    require_vec(&mut errors, "failure_states", &contract.failure_states);
    require_vec(
        &mut errors,
        "research_basis_refs",
        &contract.research_basis_refs,
    );
    require_vec(
        &mut errors,
        "product_authority_refs",
        &contract.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &contract.folded_source_refs,
    );

    validate_nodes(&mut errors, contract);
    validate_edges(&mut errors, contract);
    validate_verdicts(&mut errors, contract);
    validate_actor_leases(&mut errors, contract);
    validate_history(&mut errors, contract);
    validate_sources(&mut errors, contract);

    if contract.prose_reports_authoritative {
        errors.push(error(
            "prose_reports_authoritative",
            "prose reports cannot be Locus graph authority",
        ));
    }
    if contract.chat_messages_authoritative {
        errors.push(error(
            "chat_messages_authoritative",
            "chat messages cannot be Locus graph authority",
        ));
    }
    if contract.status_mutation_allowed {
        errors.push(error(
            "status_mutation_allowed",
            "graph projection cannot mutate Locus, MT, WP, or task board status",
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_nodes(
    errors: &mut Vec<LocusMtValidationWorkGraphError>,
    contract: &LocusMtValidationWorkGraphContractV1,
) {
    let has_blocked = contract
        .nodes
        .iter()
        .any(|node| node.state == LocusGraphNodeState::Blocked);
    let has_escalated = contract
        .nodes
        .iter()
        .any(|node| node.state == LocusGraphNodeState::Escalated);

    if !has_blocked || !has_escalated {
        errors.push(error(
            "nodes.state",
            "graph must expose blocked and escalated MT states",
        ));
    }

    for node in &contract.nodes {
        require_non_empty(errors, "nodes.node_id", &node.node_id);
        require_non_empty(errors, "nodes.mt_id", &node.mt_id);
        require_non_empty(errors, "nodes.source_ref", &node.source_ref);
        require_vec(errors, "nodes.evidence_refs", &node.evidence_refs);
    }
}

fn validate_edges(
    errors: &mut Vec<LocusMtValidationWorkGraphError>,
    contract: &LocusMtValidationWorkGraphContractV1,
) {
    for edge in contract
        .dependency_edges
        .iter()
        .chain(contract.remediation_edges.iter())
    {
        require_non_empty(errors, "edges.edge_id", &edge.edge_id);
        require_non_empty(errors, "edges.from_node_id", &edge.from_node_id);
        require_non_empty(errors, "edges.to_node_id", &edge.to_node_id);
        require_non_empty(errors, "edges.source_ref", &edge.source_ref);
    }

    if !contract
        .remediation_edges
        .iter()
        .any(|edge| edge.edge_kind == LocusGraphEdgeKind::Remediates)
    {
        errors.push(error(
            "remediation_edges.edge_kind",
            "remediation edges must include explicit Remediates relationships",
        ));
    }
}

fn validate_verdicts(
    errors: &mut Vec<LocusMtValidationWorkGraphError>,
    contract: &LocusMtValidationWorkGraphContractV1,
) {
    for verdict in &contract.verdicts {
        require_non_empty(errors, "verdicts.verdict_id", &verdict.verdict_id);
        require_non_empty(errors, "verdicts.mt_id", &verdict.mt_id);
        require_non_empty(errors, "verdicts.source_ref", &verdict.source_ref);
        require_vec(errors, "verdicts.evidence_refs", &verdict.evidence_refs);
        if verdict.verdict != ValidatorVerdictKind::Pass
            && verdict.routing_outcome == VerdictRoutingOutcome::MayAdvance
        {
            errors.push(error(
                "verdicts.routing_outcome",
                "non-pass verdicts cannot advance dependents in the graph",
            ));
        }
    }
}

fn validate_actor_leases(
    errors: &mut Vec<LocusMtValidationWorkGraphError>,
    contract: &LocusMtValidationWorkGraphContractV1,
) {
    for lease in &contract.actor_leases {
        require_non_empty(errors, "actor_leases.lease_id", &lease.lease_id);
        require_non_empty(errors, "actor_leases.mt_id", &lease.mt_id);
        require_non_empty(
            errors,
            "actor_leases.actor_session_id",
            &lease.actor_session_id,
        );
        require_non_empty(errors, "actor_leases.source_ref", &lease.source_ref);
    }
}

fn validate_history(
    errors: &mut Vec<LocusMtValidationWorkGraphError>,
    contract: &LocusMtValidationWorkGraphContractV1,
) {
    for entry in &contract.pass_fail_history {
        require_non_empty(errors, "pass_fail_history.history_id", &entry.history_id);
        require_non_empty(errors, "pass_fail_history.mt_id", &entry.mt_id);
        require_non_empty(errors, "pass_fail_history.verdict_id", &entry.verdict_id);
        require_non_empty(
            errors,
            "pass_fail_history.evidence_ref",
            &entry.evidence_ref,
        );
    }
}

fn validate_sources(
    errors: &mut Vec<LocusMtValidationWorkGraphError>,
    contract: &LocusMtValidationWorkGraphContractV1,
) {
    for source in &contract.source_refs {
        require_non_empty(errors, "source_refs.source_ref", &source.source_ref);
        require_non_empty(errors, "source_refs.source_hash", &source.source_hash);
        match source.source_kind {
            LocusGraphSourceKind::MachineContract => {}
            LocusGraphSourceKind::ProseReport | LocusGraphSourceKind::ChatMessage => {
                errors.push(error(
                    "source_refs.source_kind",
                    "Locus graph authority must come from machine contracts, not prose or chat",
                ));
            }
        }
    }
}

fn mt_node(mt_id: &str, state: LocusGraphNodeState, source_ref: &str) -> LocusMtGraphNodeV1 {
    LocusMtGraphNodeV1 {
        node_id: format!("locus-mt-node-{mt_id}"),
        mt_id: mt_id.to_string(),
        node_kind: LocusGraphNodeKind::MicroTask,
        state,
        source_ref: source_ref.to_string(),
        evidence_refs: vec![format!("{source_ref}/evidence")],
    }
}

fn edge(
    edge_id: &str,
    from_node_id: &str,
    to_node_id: &str,
    edge_kind: LocusGraphEdgeKind,
    source_ref: &str,
) -> LocusWorkGraphEdgeV1 {
    LocusWorkGraphEdgeV1 {
        edge_id: edge_id.to_string(),
        from_node_id: from_node_id.to_string(),
        to_node_id: to_node_id.to_string(),
        edge_kind,
        source_ref: source_ref.to_string(),
    }
}

fn verdict(
    verdict_id: &str,
    mt_id: &str,
    verdict: ValidatorVerdictKind,
    routing_outcome: VerdictRoutingOutcome,
    source_ref: &str,
) -> LocusValidatorVerdictGraphV1 {
    LocusValidatorVerdictGraphV1 {
        verdict_id: verdict_id.to_string(),
        mt_id: mt_id.to_string(),
        verdict,
        routing_outcome,
        evidence_refs: vec![format!("{source_ref}/evidence")],
        source_ref: source_ref.to_string(),
    }
}

fn machine_source(source_ref: &str) -> LocusWorkGraphSourceRefV1 {
    LocusWorkGraphSourceRefV1 {
        source_ref: source_ref.to_string(),
        source_kind: LocusGraphSourceKind::MachineContract,
        source_hash: source_hash("kernel002-mt061", &[source_ref]),
    }
}

fn source_hash(domain: &str, parts: &[&str]) -> String {
    format!(
        "sha256:{}",
        sha256_hex(format!("{domain}|{}", parts.join("|")).as_bytes())
    )
}

fn require_non_empty(
    errors: &mut Vec<LocusMtValidationWorkGraphError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(error(field, "field must not be empty"));
    }
}

fn require_vec<T>(
    errors: &mut Vec<LocusMtValidationWorkGraphError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(error(field, "field must not be empty"));
    }
}

fn error(field: &'static str, message: &'static str) -> LocusMtValidationWorkGraphError {
    LocusMtValidationWorkGraphError { field, message }
}

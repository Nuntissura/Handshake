use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    locus_mt_validation_work_graph::{
        build_kernel002_locus_mt_validation_work_graph, project_locus_mt_validation_work_graph,
        validate_locus_mt_validation_work_graph, LocusGraphEdgeKind, LocusGraphFailureState,
        LocusGraphNodeState, LocusGraphSourceKind,
        LOCUS_MT_VALIDATION_WORK_GRAPH_PROJECTION_SCHEMA_ID,
        LOCUS_MT_VALIDATION_WORK_GRAPH_SCHEMA_ID,
    },
    validator_verdict_mediation_contract::{ValidatorVerdictKind, VerdictRoutingOutcome},
};

#[test]
fn locus_mt_validation_work_graph_projects_nodes_verdicts_edges_leases_and_history() {
    let contract = build_kernel002_locus_mt_validation_work_graph();

    validate_locus_mt_validation_work_graph(&contract).expect("work graph validates");
    let projection =
        project_locus_mt_validation_work_graph(&contract).expect("work graph projection derives");

    assert_eq!(contract.schema_id, LOCUS_MT_VALIDATION_WORK_GRAPH_SCHEMA_ID);
    assert_eq!(contract.mt_id, "MT-061");
    assert!(contract.nodes.iter().any(|node| node.mt_id == "MT-061"));
    assert!(contract
        .verdicts
        .iter()
        .any(|verdict| verdict.verdict == ValidatorVerdictKind::Pass
            && verdict.routing_outcome == VerdictRoutingOutcome::MayAdvance));
    assert!(contract
        .remediation_edges
        .iter()
        .any(|edge| edge.edge_kind == LocusGraphEdgeKind::Remediates));
    assert!(contract
        .nodes
        .iter()
        .any(|node| node.state == LocusGraphNodeState::Blocked));
    assert!(contract
        .nodes
        .iter()
        .any(|node| node.state == LocusGraphNodeState::Escalated));
    assert!(contract.actor_leases.iter().any(|lease| lease.active));
    assert!(contract.pass_fail_history.iter().any(|entry| entry.passed));
    assert!(contract.pass_fail_history.iter().any(|entry| !entry.passed));

    assert_eq!(
        projection.schema_id,
        LOCUS_MT_VALIDATION_WORK_GRAPH_PROJECTION_SCHEMA_ID
    );
    assert!(projection
        .mt_node_ids
        .contains(&"locus-mt-node-MT-061".to_string()));
    assert!(projection.remediation_edge_count > 0);
    assert!(projection
        .blocked_node_ids
        .iter()
        .any(|node| node.contains("MT-058")));
    assert!(projection
        .escalated_node_ids
        .iter()
        .any(|node| node.contains("MT-057")));
    assert!(!projection.actor_lease_refs.is_empty());
    assert!(!projection.pass_fail_history_refs.is_empty());
}

#[test]
fn locus_mt_validation_work_graph_rejects_prose_or_chat_truth_sources() {
    let contract = build_kernel002_locus_mt_validation_work_graph();
    assert!(contract
        .source_refs
        .iter()
        .all(|source| source.source_kind == LocusGraphSourceKind::MachineContract));

    let mut prose_source = build_kernel002_locus_mt_validation_work_graph();
    prose_source.source_refs.push(
        handshake_core::kernel::locus_mt_validation_work_graph::LocusWorkGraphSourceRefV1 {
            source_ref: "prose-report://validator-summary".to_string(),
            source_kind: LocusGraphSourceKind::ProseReport,
            source_hash: "sha256:prose".to_string(),
        },
    );
    let errors = validate_locus_mt_validation_work_graph(&prose_source)
        .expect_err("prose reports cannot be authority");
    assert!(errors
        .iter()
        .any(|error| error.field == "source_refs.source_kind"));

    let mut chat_source = build_kernel002_locus_mt_validation_work_graph();
    chat_source.source_refs.push(
        handshake_core::kernel::locus_mt_validation_work_graph::LocusWorkGraphSourceRefV1 {
            source_ref: "chat://thread/validator-message".to_string(),
            source_kind: LocusGraphSourceKind::ChatMessage,
            source_hash: "sha256:chat".to_string(),
        },
    );
    let errors = validate_locus_mt_validation_work_graph(&chat_source)
        .expect_err("chat messages cannot be authority");
    assert!(errors
        .iter()
        .any(|error| error.field == "source_refs.source_kind"));
}

#[test]
fn locus_mt_validation_work_graph_projection_is_read_only_authority() {
    let contract = build_kernel002_locus_mt_validation_work_graph();
    let projection =
        project_locus_mt_validation_work_graph(&contract).expect("work graph projection derives");

    assert!(!projection.prose_source_allowed);
    assert!(!projection.chat_source_allowed);
    assert!(!projection.status_mutation_allowed);
    assert!(!contract.prose_reports_authoritative);
    assert!(!contract.chat_messages_authoritative);
    assert!(!contract.status_mutation_allowed);
}

#[test]
fn locus_mt_validation_work_graph_rejects_missing_nodes_edges_leases_or_history() {
    let mut contract = build_kernel002_locus_mt_validation_work_graph();
    contract.nodes.clear();
    let errors =
        validate_locus_mt_validation_work_graph(&contract).expect_err("nodes are required");
    assert!(errors.iter().any(|error| error.field == "nodes"));

    let mut contract = build_kernel002_locus_mt_validation_work_graph();
    contract.remediation_edges.clear();
    let errors = validate_locus_mt_validation_work_graph(&contract)
        .expect_err("remediation edges are required");
    assert!(errors
        .iter()
        .any(|error| error.field == "remediation_edges"));

    let mut contract = build_kernel002_locus_mt_validation_work_graph();
    contract.actor_leases.clear();
    let errors =
        validate_locus_mt_validation_work_graph(&contract).expect_err("actor leases are required");
    assert!(errors.iter().any(|error| error.field == "actor_leases"));

    let mut contract = build_kernel002_locus_mt_validation_work_graph();
    contract.pass_fail_history.clear();
    let errors = validate_locus_mt_validation_work_graph(&contract)
        .expect_err("pass/fail history is required");
    assert!(errors
        .iter()
        .any(|error| error.field == "pass_fail_history"));
}

#[test]
fn locus_mt_validation_work_graph_records_failure_states_and_research_basis() {
    let contract = build_kernel002_locus_mt_validation_work_graph();

    for failure_state in [
        LocusGraphFailureState::MissingMtNodes,
        LocusGraphFailureState::MissingValidatorVerdicts,
        LocusGraphFailureState::MissingRemediationEdges,
        LocusGraphFailureState::MissingBlockedEscalatedStates,
        LocusGraphFailureState::MissingActorLeases,
        LocusGraphFailureState::MissingPassFailHistory,
        LocusGraphFailureState::ProseReportUsedAsTruth,
        LocusGraphFailureState::ChatMessageUsedAsTruth,
    ] {
        assert!(contract.failure_states.contains(&failure_state));
    }

    for source_fragment in [
        "docs.github.com/en/rest/issues/timeline",
        "docs.gitlab.com/user/work_items/linked_items",
        "kubernetes.io/docs/concepts/overview/working-with-objects/owners-dependents",
        "opentelemetry.io/docs/concepts/signals/traces",
    ] {
        assert!(contract
            .research_basis_refs
            .iter()
            .any(|basis| basis.source_ref.contains(source_fragment)));
    }
}

#[test]
fn kernel_action_catalog_exposes_locus_mt_validation_work_graph_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.locus_mt_validation_work_graph.project")
        .expect("Locus MT validation graph projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "locus_mt_graph_nodes"));
}

use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    git_engine_decision_gate::{
        project_git_engine_decision_gate, validate_git_engine_decision_gate, GitActionGateV1,
        GitActionKind, GitActionRisk, GitEngineDecisionGateV1, GitOssBackendEntryV1,
        RepoEngineBackend,
    },
};

#[test]
fn kernel_git_engine_decision_gate_enforces_single_cli_backend() {
    let gate = sample_gate();

    validate_git_engine_decision_gate(&gate).expect("git engine gate validates");

    assert_eq!(
        gate.selected_backend,
        RepoEngineBackend::GitCliExternalProcess
    );
    assert!(!gate.fallback_backends_allowed);
    assert_eq!(gate.oss_register_entries.len(), 1);
    assert!(gate.oss_register_entries[0].approved_for_phase1);
}

#[test]
fn kernel_git_engine_decision_gate_projects_lawful_dcc_affordances() {
    let gate = sample_gate();
    let projection = project_git_engine_decision_gate(&gate).expect("projection builds");

    assert!(projection.one_backend_enforced);
    assert!(projection
        .lawful_dcc_affordance_ids
        .contains(&"dcc.git.status".to_string()));
    assert!(projection
        .lawful_dcc_affordance_ids
        .contains(&"dcc.git.commit".to_string()));
    assert!(!projection
        .lawful_dcc_affordance_ids
        .contains(&"dcc.git.reset_hard".to_string()));
    assert!(projection
        .dangerous_gated_action_ids
        .contains(&"git.reset_hard".to_string()));
    assert!(!projection.mutates_repo);
}

#[test]
fn kernel_git_engine_decision_gate_rejects_drift_and_ungated_dangerous_actions() {
    let mut gate = sample_gate();
    gate.fallback_backends_allowed = true;
    gate.oss_register_entries.push(GitOssBackendEntryV1 {
        backend: RepoEngineBackend::Libgit2,
        package_ref: "crate://git2".to_string(),
        license_ref: "license://libgit2-exception".to_string(),
        approved_for_phase1: true,
    });
    gate.action_gates[2].approval_required = false;
    gate.action_gates[2].lawful_dcc_affordance = true;
    gate.action_gates[2].flight_recorder_ref.clear();

    let errors = validate_git_engine_decision_gate(&gate).expect_err("unsafe git gate must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "fallback_backends_allowed"));
    assert!(errors
        .iter()
        .any(|error| error.field == "oss_register_entries"));
    assert!(errors
        .iter()
        .any(|error| error.field == "action_gates.approval_required"));
    assert!(errors
        .iter()
        .any(|error| error.field == "action_gates.lawful_dcc_affordance"));
    assert!(errors
        .iter()
        .any(|error| error.field == "action_gates.flight_recorder_ref"));
}

#[test]
fn kernel_git_engine_decision_gate_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.git_engine_decision_gate.project")
        .expect("git engine decision projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "git_single_backend_enforced"));
}

fn sample_gate() -> GitEngineDecisionGateV1 {
    GitEngineDecisionGateV1 {
        schema_id: "hsk.kernel.git_engine_decision_gate@1".to_string(),
        gate_id: "git-engine-mt040".to_string(),
        folded_stub_ids: vec!["WP-1-Git-Engine-Decision-Gate-v1".to_string()],
        selected_backend: RepoEngineBackend::GitCliExternalProcess,
        fallback_backends_allowed: false,
        oss_register_entries: vec![GitOssBackendEntryV1 {
            backend: RepoEngineBackend::GitCliExternalProcess,
            package_ref: "external-process://git".to_string(),
            license_ref: "license://system-git-cli".to_string(),
            approved_for_phase1: true,
        }],
        action_gates: vec![
            action(
                "git.status",
                GitActionKind::Status,
                GitActionRisk::SafeRead,
                false,
                true,
            ),
            action(
                "git.commit",
                GitActionKind::Commit,
                GitActionRisk::GatedWrite,
                true,
                true,
            ),
            action(
                "git.reset_hard",
                GitActionKind::ResetHard,
                GitActionRisk::Dangerous,
                true,
                false,
            ),
        ],
        product_authority_refs: vec![
            "kernel.action_catalog".to_string(),
            "kernel.local_first_mcp_posture".to_string(),
            "flight_recorder.repo_engine".to_string(),
            "oss_register.git_backend".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Git-Engine-Decision-Gate-v1.contract.json".to_string(),
        ],
    }
}

fn action(
    action_id: &str,
    action_kind: GitActionKind,
    risk: GitActionRisk,
    approval_required: bool,
    lawful_dcc_affordance: bool,
) -> GitActionGateV1 {
    GitActionGateV1 {
        action_id: action_id.to_string(),
        action_kind,
        risk,
        capability_id: format!("capability://{action_id}"),
        approval_required,
        flight_recorder_ref: format!("FR-EVT-REPO-{action_id}"),
        dcc_affordance_id: format!("dcc.{action_id}"),
        lawful_dcc_affordance,
    }
}

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const FOLDED_GIT_ENGINE_DECISION_GATE_STUB_ID: &str = "WP-1-Git-Engine-Decision-Gate-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepoEngineBackend {
    GitCliExternalProcess,
    Libgit2,
    GoGit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GitActionKind {
    Status,
    Diff,
    Commit,
    Push,
    ResetHard,
    Rebase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GitActionRisk {
    SafeRead,
    GatedWrite,
    Dangerous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitOssBackendEntryV1 {
    pub backend: RepoEngineBackend,
    pub package_ref: String,
    pub license_ref: String,
    pub approved_for_phase1: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitActionGateV1 {
    pub action_id: String,
    pub action_kind: GitActionKind,
    pub risk: GitActionRisk,
    pub capability_id: String,
    pub approval_required: bool,
    pub flight_recorder_ref: String,
    pub dcc_affordance_id: String,
    pub lawful_dcc_affordance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitEngineDecisionGateV1 {
    pub schema_id: String,
    pub gate_id: String,
    pub folded_stub_ids: Vec<String>,
    pub selected_backend: RepoEngineBackend,
    pub fallback_backends_allowed: bool,
    pub oss_register_entries: Vec<GitOssBackendEntryV1>,
    pub action_gates: Vec<GitActionGateV1>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitEngineDecisionGateProjectionV1 {
    pub schema_id: String,
    pub gate_id: String,
    pub selected_backend: RepoEngineBackend,
    pub one_backend_enforced: bool,
    pub lawful_dcc_affordance_ids: Vec<String>,
    pub dangerous_gated_action_ids: Vec<String>,
    pub gated_write_action_ids: Vec<String>,
    pub oss_license_refs: Vec<String>,
    pub mutates_repo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitEngineDecisionGateValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_git_engine_decision_gate(
    gate: &GitEngineDecisionGateV1,
) -> Result<(), Vec<GitEngineDecisionGateValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &gate.schema_id);
    require_non_empty(&mut errors, "gate_id", &gate.gate_id);
    require_vec(&mut errors, "folded_stub_ids", &gate.folded_stub_ids);
    require_vec(
        &mut errors,
        "oss_register_entries",
        &gate.oss_register_entries,
    );
    require_vec(&mut errors, "action_gates", &gate.action_gates);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &gate.product_authority_refs,
    );
    require_vec(&mut errors, "folded_source_refs", &gate.folded_source_refs);

    if !contains_exact(
        &gate.folded_stub_ids,
        FOLDED_GIT_ENGINE_DECISION_GATE_STUB_ID,
    ) {
        errors.push(GitEngineDecisionGateValidationError {
            field: "folded_stub_ids",
            message: "git engine gate must preserve the folded stub id",
        });
    }
    if !contains_text(
        &gate.folded_source_refs,
        FOLDED_GIT_ENGINE_DECISION_GATE_STUB_ID,
    ) {
        errors.push(GitEngineDecisionGateValidationError {
            field: "folded_source_refs",
            message: "git engine gate must preserve the folded source reference",
        });
    }
    if gate.selected_backend != RepoEngineBackend::GitCliExternalProcess {
        errors.push(GitEngineDecisionGateValidationError {
            field: "selected_backend",
            message: "Phase 1 repo engine backend must default to git CLI external_process",
        });
    }
    if gate.fallback_backends_allowed {
        errors.push(GitEngineDecisionGateValidationError {
            field: "fallback_backends_allowed",
            message: "repo engine must not silently fall back to another backend",
        });
    }

    validate_authority_refs(&mut errors, gate);
    validate_oss_entries(&mut errors, gate);
    validate_action_gates(&mut errors, gate);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_git_engine_decision_gate(
    gate: &GitEngineDecisionGateV1,
) -> Result<GitEngineDecisionGateProjectionV1, Vec<GitEngineDecisionGateValidationError>> {
    validate_git_engine_decision_gate(gate)?;

    Ok(GitEngineDecisionGateProjectionV1 {
        schema_id: "hsk.kernel.git_engine_decision_gate_projection@1".to_string(),
        gate_id: gate.gate_id.clone(),
        selected_backend: gate.selected_backend,
        one_backend_enforced: !gate.fallback_backends_allowed,
        lawful_dcc_affordance_ids: gate
            .action_gates
            .iter()
            .filter(|action| action.lawful_dcc_affordance)
            .map(|action| action.dcc_affordance_id.clone())
            .collect(),
        dangerous_gated_action_ids: gate
            .action_gates
            .iter()
            .filter(|action| action.risk == GitActionRisk::Dangerous && action.approval_required)
            .map(|action| action.action_id.clone())
            .collect(),
        gated_write_action_ids: gate
            .action_gates
            .iter()
            .filter(|action| action.risk == GitActionRisk::GatedWrite)
            .map(|action| action.action_id.clone())
            .collect(),
        oss_license_refs: gate
            .oss_register_entries
            .iter()
            .map(|entry| entry.license_ref.clone())
            .collect(),
        mutates_repo: false,
    })
}

fn validate_authority_refs(
    errors: &mut Vec<GitEngineDecisionGateValidationError>,
    gate: &GitEngineDecisionGateV1,
) {
    for required_ref in [
        "kernel.action_catalog",
        "kernel.local_first_mcp_posture",
        "flight_recorder.repo_engine",
        "oss_register.git_backend",
    ] {
        if !contains_exact(&gate.product_authority_refs, required_ref) {
            errors.push(GitEngineDecisionGateValidationError {
                field: "product_authority_refs",
                message: "git engine gate must cite action catalog, local-first posture, repo Flight Recorder, and OSS register authorities",
            });
        }
    }
}

fn validate_oss_entries(
    errors: &mut Vec<GitEngineDecisionGateValidationError>,
    gate: &GitEngineDecisionGateV1,
) {
    let mut approved_backend_count = 0usize;
    let mut selected_backend_seen = false;
    let mut seen_backends = HashSet::new();

    for entry in &gate.oss_register_entries {
        if !seen_backends.insert(entry.backend) {
            errors.push(GitEngineDecisionGateValidationError {
                field: "oss_register_entries",
                message: "OSS register must not duplicate repo engine backend entries",
            });
        }
        require_non_empty(
            errors,
            "oss_register_entries.package_ref",
            &entry.package_ref,
        );
        require_non_empty(
            errors,
            "oss_register_entries.license_ref",
            &entry.license_ref,
        );

        if entry.backend == gate.selected_backend {
            selected_backend_seen = true;
        }
        if entry.approved_for_phase1 {
            approved_backend_count += 1;
            if entry.backend != gate.selected_backend {
                errors.push(GitEngineDecisionGateValidationError {
                    field: "oss_register_entries",
                    message: "only the selected repo engine backend may be Phase 1 approved",
                });
            }
        }
    }

    if !selected_backend_seen || approved_backend_count != 1 {
        errors.push(GitEngineDecisionGateValidationError {
            field: "oss_register_entries",
            message: "OSS register must approve exactly the selected backend",
        });
    }
}

fn validate_action_gates(
    errors: &mut Vec<GitEngineDecisionGateValidationError>,
    gate: &GitEngineDecisionGateV1,
) {
    let mut action_ids = HashSet::new();

    for action in &gate.action_gates {
        if !action_ids.insert(action.action_id.as_str()) {
            errors.push(GitEngineDecisionGateValidationError {
                field: "action_gates.action_id",
                message: "git action gate ids must be unique",
            });
        }

        require_non_empty(errors, "action_gates.action_id", &action.action_id);
        require_non_empty(errors, "action_gates.capability_id", &action.capability_id);
        require_non_empty(
            errors,
            "action_gates.flight_recorder_ref",
            &action.flight_recorder_ref,
        );
        if !action.flight_recorder_ref.starts_with("FR-EVT-REPO-") {
            errors.push(GitEngineDecisionGateValidationError {
                field: "action_gates.flight_recorder_ref",
                message: "repo engine action gates must cite FR-EVT-REPO evidence",
            });
        }
        if action.lawful_dcc_affordance {
            require_non_empty(
                errors,
                "action_gates.dcc_affordance_id",
                &action.dcc_affordance_id,
            );
        }

        if action.risk != GitActionRisk::SafeRead && !action.approval_required {
            errors.push(GitEngineDecisionGateValidationError {
                field: "action_gates.approval_required",
                message: "write and dangerous git actions must be approval gated",
            });
        }
        if action.risk == GitActionRisk::Dangerous && action.lawful_dcc_affordance {
            errors.push(GitEngineDecisionGateValidationError {
                field: "action_gates.lawful_dcc_affordance",
                message: "dangerous git actions must not be exposed as lawful DCC affordances",
            });
        }
    }
}

fn require_non_empty(
    errors: &mut Vec<GitEngineDecisionGateValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(GitEngineDecisionGateValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<GitEngineDecisionGateValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(GitEngineDecisionGateValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::action_envelope::AuthorityEffect;

pub const FOLDED_GOVERNANCE_PACK_STUB_ID: &str = "WP-1-Governance-Pack-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NamingStyle {
    Underscore,
    Kebab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GovernancePackComponentKind {
    ProjectIdentity,
    CodexAuthority,
    MasterSpec,
    RoleProtocol,
    WorkflowStateRegistry,
    TransitionAutomation,
    ConformanceHarness,
    ImportedOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GovernancePackOverlayKind {
    SoftwareDeliveryProfileOverlay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceProjectIdentityV1 {
    pub project_code: String,
    pub project_display_name: String,
    pub codex_authority_filename: String,
    pub master_spec_filename: String,
    pub naming_style: NamingStyle,
    pub language_layout_profile: String,
    pub external_tool_paths_config_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernancePackPathPolicyV1 {
    pub deterministic_paths: bool,
    pub repo_relative_paths_only: bool,
    pub no_blank_spaces: bool,
    pub no_handshake_hardcoding: bool,
    pub external_tool_paths_project_configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernancePackOverlayBoundaryV1 {
    pub overlay_id: String,
    pub overlay_kind: GovernancePackOverlayKind,
    pub profile_id: String,
    pub replaces_native_governance: bool,
    pub allowed_authority_effect: AuthorityEffect,
    pub expected_write_box_schema_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernancePackManifestItemV1 {
    pub item_id: String,
    pub component_kind: GovernancePackComponentKind,
    pub template_ref: String,
    pub target_path_template: String,
    pub write_box_kind: String,
    pub authority_effect: AuthorityEffect,
    pub promotion_required: bool,
    pub imported_overlay_boundary: Option<GovernancePackOverlayBoundaryV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernancePackConformanceHarnessV1 {
    pub harness_id: String,
    pub required_for_alternate_implementations: bool,
    pub check_refs: Vec<String>,
    pub intent_equivalence_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernancePackInstantiationV1 {
    pub schema_id: String,
    pub pack_id: String,
    pub folded_stub_ids: Vec<String>,
    pub project_identity: GovernanceProjectIdentityV1,
    pub path_policy: GovernancePackPathPolicyV1,
    pub manifest_items: Vec<GovernancePackManifestItemV1>,
    pub conformance_harness: GovernancePackConformanceHarnessV1,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernancePackInstantiationProjectionV1 {
    pub schema_id: String,
    pub pack_id: String,
    pub project_code: String,
    pub manifest_item_ids: Vec<String>,
    pub target_path_templates: Vec<String>,
    pub imported_overlay_ids: Vec<String>,
    pub conformance_check_refs: Vec<String>,
    pub repo_relative_paths_only: bool,
    pub deterministic_naming: bool,
    pub kernel_law_compatible: bool,
    pub mutates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernancePackInstantiationValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_governance_pack_instantiation(
    pack: &GovernancePackInstantiationV1,
) -> Result<(), Vec<GovernancePackInstantiationValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &pack.schema_id);
    require_non_empty(&mut errors, "pack_id", &pack.pack_id);
    require_vec(&mut errors, "folded_stub_ids", &pack.folded_stub_ids);
    require_vec(&mut errors, "manifest_items", &pack.manifest_items);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &pack.product_authority_refs,
    );
    require_vec(&mut errors, "folded_source_refs", &pack.folded_source_refs);

    if !contains_exact(&pack.folded_stub_ids, FOLDED_GOVERNANCE_PACK_STUB_ID) {
        errors.push(GovernancePackInstantiationValidationError {
            field: "folded_stub_ids",
            message: "governance pack must preserve the folded stub id",
        });
    }
    if !contains_text(&pack.folded_source_refs, FOLDED_GOVERNANCE_PACK_STUB_ID) {
        errors.push(GovernancePackInstantiationValidationError {
            field: "folded_source_refs",
            message: "governance pack must preserve the folded source reference",
        });
    }

    validate_project_identity(&mut errors, pack);
    validate_path_policy(&mut errors, pack);
    validate_authority_refs(&mut errors, pack);
    validate_manifest_items(&mut errors, pack);
    validate_manifest_coverage(&mut errors, pack);
    validate_conformance_harness(&mut errors, pack);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_governance_pack_instantiation(
    pack: &GovernancePackInstantiationV1,
) -> Result<GovernancePackInstantiationProjectionV1, Vec<GovernancePackInstantiationValidationError>>
{
    validate_governance_pack_instantiation(pack)?;

    Ok(GovernancePackInstantiationProjectionV1 {
        schema_id: "hsk.kernel.governance_pack_instantiation_projection@1".to_string(),
        pack_id: pack.pack_id.clone(),
        project_code: pack.project_identity.project_code.clone(),
        manifest_item_ids: pack
            .manifest_items
            .iter()
            .map(|item| item.item_id.clone())
            .collect(),
        target_path_templates: pack
            .manifest_items
            .iter()
            .map(|item| item.target_path_template.clone())
            .collect(),
        imported_overlay_ids: pack
            .manifest_items
            .iter()
            .filter_map(|item| item.imported_overlay_boundary.as_ref())
            .map(|boundary| boundary.overlay_id.clone())
            .collect(),
        conformance_check_refs: pack.conformance_harness.check_refs.clone(),
        repo_relative_paths_only: pack.path_policy.repo_relative_paths_only,
        deterministic_naming: deterministic_naming(pack),
        kernel_law_compatible: kernel_law_compatible(pack),
        mutates_authority: false,
    })
}

fn validate_project_identity(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    pack: &GovernancePackInstantiationV1,
) {
    let identity = &pack.project_identity;

    require_non_empty(
        errors,
        "project_identity.project_code",
        &identity.project_code,
    );
    require_non_empty(
        errors,
        "project_identity.project_display_name",
        &identity.project_display_name,
    );
    require_non_empty(
        errors,
        "project_identity.codex_authority_filename",
        &identity.codex_authority_filename,
    );
    require_non_empty(
        errors,
        "project_identity.master_spec_filename",
        &identity.master_spec_filename,
    );
    require_non_empty(
        errors,
        "project_identity.language_layout_profile",
        &identity.language_layout_profile,
    );
    require_non_empty(
        errors,
        "project_identity.external_tool_paths_config_ref",
        &identity.external_tool_paths_config_ref,
    );

    if !is_underscore_name(&identity.project_code) {
        errors.push(GovernancePackInstantiationValidationError {
            field: "project_identity.project_code",
            message: "project code must use deterministic underscore naming",
        });
    }

    for (field, filename) in [
        (
            "project_identity.codex_authority_filename",
            identity.codex_authority_filename.as_str(),
        ),
        (
            "project_identity.master_spec_filename",
            identity.master_spec_filename.as_str(),
        ),
    ] {
        if contains_blank_space(filename) {
            errors.push(GovernancePackInstantiationValidationError {
                field,
                message: "authority filenames must not contain blank spaces",
            });
        }
        if contains_handshake(filename) {
            errors.push(GovernancePackInstantiationValidationError {
                field,
                message: "governance pack filenames must not be Handshake-hardcoded",
            });
        }
        if !filename.contains(&identity.project_code) {
            errors.push(GovernancePackInstantiationValidationError {
                field,
                message: "authority filenames must include the project code",
            });
        }
    }
}

fn validate_path_policy(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    pack: &GovernancePackInstantiationV1,
) {
    let policy = &pack.path_policy;

    for (field, enabled) in [
        (
            "path_policy.deterministic_paths",
            policy.deterministic_paths,
        ),
        (
            "path_policy.repo_relative_paths_only",
            policy.repo_relative_paths_only,
        ),
        ("path_policy.no_blank_spaces", policy.no_blank_spaces),
        (
            "path_policy.no_handshake_hardcoding",
            policy.no_handshake_hardcoding,
        ),
        (
            "path_policy.external_tool_paths_project_configured",
            policy.external_tool_paths_project_configured,
        ),
    ] {
        if !enabled {
            errors.push(GovernancePackInstantiationValidationError {
                field,
                message: "governance pack path policy control must be enabled",
            });
        }
    }
}

fn validate_authority_refs(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    pack: &GovernancePackInstantiationV1,
) {
    for required_ref in [
        "kernel.action_catalog",
        "kernel.write_boxes",
        "kernel.governance_overlay_boundary",
        "kernel.workflow_transition_registry",
        "kernel.local_first_mcp_posture",
    ] {
        if !contains_exact(&pack.product_authority_refs, required_ref) {
            errors.push(GovernancePackInstantiationValidationError {
                field: "product_authority_refs",
                message: "governance pack must cite action catalog, write boxes, overlay boundary, transition registry, and local-first posture authorities",
            });
        }
    }
}

fn validate_manifest_items(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    pack: &GovernancePackInstantiationV1,
) {
    let mut item_ids = HashSet::new();

    for item in &pack.manifest_items {
        if !item_ids.insert(item.item_id.as_str()) {
            errors.push(GovernancePackInstantiationValidationError {
                field: "manifest_items.item_id",
                message: "governance pack manifest item ids must be unique",
            });
        }

        require_non_empty(errors, "manifest_items.item_id", &item.item_id);
        require_non_empty(errors, "manifest_items.template_ref", &item.template_ref);
        require_non_empty(
            errors,
            "manifest_items.target_path_template",
            &item.target_path_template,
        );
        require_non_empty(
            errors,
            "manifest_items.write_box_kind",
            &item.write_box_kind,
        );

        validate_manifest_path(errors, pack, item);
        validate_manifest_authority(errors, item);
        validate_imported_overlay_boundary(errors, item);
    }
}

fn validate_manifest_path(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    pack: &GovernancePackInstantiationV1,
    item: &GovernancePackManifestItemV1,
) {
    let path = item.target_path_template.as_str();

    if !path.contains("{project_code}") {
        errors.push(GovernancePackInstantiationValidationError {
            field: "manifest_items.target_path_template",
            message: "governance pack target paths must be project-code parameterized",
        });
    }
    if is_absolute_or_unc(path) {
        errors.push(GovernancePackInstantiationValidationError {
            field: "manifest_items.target_path_template",
            message: "governance pack target paths must be repo-relative",
        });
    }
    if pack.path_policy.no_blank_spaces && contains_blank_space(path) {
        errors.push(GovernancePackInstantiationValidationError {
            field: "manifest_items.target_path_template",
            message: "governance pack target paths must not contain blank spaces",
        });
    }
    if pack.path_policy.no_handshake_hardcoding && contains_handshake(path) {
        errors.push(GovernancePackInstantiationValidationError {
            field: "manifest_items.target_path_template",
            message: "governance pack target paths must not be Handshake-hardcoded",
        });
    }
}

fn validate_manifest_authority(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    item: &GovernancePackManifestItemV1,
) {
    if item.authority_effect == AuthorityEffect::EventLedgerAuthorityWrite {
        errors.push(GovernancePackInstantiationValidationError {
            field: "manifest_items.authority_effect",
            message: "governance pack instantiation must not write authority directly",
        });
    }
    if item.authority_effect != AuthorityEffect::ProjectionOnly && !item.promotion_required {
        errors.push(GovernancePackInstantiationValidationError {
            field: "manifest_items.promotion_required",
            message: "non-projection governance pack items must require promotion",
        });
    }
}

fn validate_imported_overlay_boundary(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    item: &GovernancePackManifestItemV1,
) {
    match (&item.component_kind, &item.imported_overlay_boundary) {
        (GovernancePackComponentKind::ImportedOverlay, Some(boundary)) => {
            require_non_empty(
                errors,
                "manifest_items.imported_overlay_boundary.overlay_id",
                &boundary.overlay_id,
            );
            require_non_empty(
                errors,
                "manifest_items.imported_overlay_boundary.profile_id",
                &boundary.profile_id,
            );
            require_non_empty(
                errors,
                "manifest_items.imported_overlay_boundary.expected_write_box_schema_id",
                &boundary.expected_write_box_schema_id,
            );

            if boundary.replaces_native_governance {
                errors.push(GovernancePackInstantiationValidationError {
                    field: "manifest_items.imported_overlay_boundary",
                    message: "imported overlays must not replace Handshake-native governance",
                });
            }
            if boundary.allowed_authority_effect == AuthorityEffect::EventLedgerAuthorityWrite {
                errors.push(GovernancePackInstantiationValidationError {
                    field: "manifest_items.imported_overlay_boundary",
                    message: "imported overlay boundary must not allow direct authority writes",
                });
            }
            if boundary.allowed_authority_effect != item.authority_effect {
                errors.push(GovernancePackInstantiationValidationError {
                    field: "manifest_items.imported_overlay_boundary",
                    message: "imported overlay boundary must match the manifest authority effect",
                });
            }
            if !boundary
                .expected_write_box_schema_id
                .starts_with("hsk.write_box.")
            {
                errors.push(GovernancePackInstantiationValidationError {
                    field: "manifest_items.imported_overlay_boundary",
                    message: "imported overlay boundary must bind a Handshake write-box schema",
                });
            }
        }
        (GovernancePackComponentKind::ImportedOverlay, None) => {
            errors.push(GovernancePackInstantiationValidationError {
                field: "manifest_items.imported_overlay_boundary",
                message: "imported overlays require an explicit boundary",
            });
        }
        (_, Some(_)) => {
            errors.push(GovernancePackInstantiationValidationError {
                field: "manifest_items.imported_overlay_boundary",
                message: "only imported overlay items may carry overlay boundaries",
            });
        }
        (_, None) => {}
    }
}

fn validate_manifest_coverage(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    pack: &GovernancePackInstantiationV1,
) {
    for component_kind in [
        GovernancePackComponentKind::ProjectIdentity,
        GovernancePackComponentKind::CodexAuthority,
        GovernancePackComponentKind::MasterSpec,
        GovernancePackComponentKind::ConformanceHarness,
        GovernancePackComponentKind::ImportedOverlay,
    ] {
        if !pack
            .manifest_items
            .iter()
            .any(|item| item.component_kind == component_kind)
        {
            errors.push(GovernancePackInstantiationValidationError {
                field: "manifest_items.component_kind",
                message: "governance pack manifest must cover identity, Codex authority, Master Spec, conformance harness, and imported overlay components",
            });
        }
    }
}

fn validate_conformance_harness(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    pack: &GovernancePackInstantiationV1,
) {
    let harness = &pack.conformance_harness;

    require_non_empty(
        errors,
        "conformance_harness.harness_id",
        &harness.harness_id,
    );
    require_vec(
        errors,
        "conformance_harness.check_refs",
        &harness.check_refs,
    );
    require_vec(
        errors,
        "conformance_harness.intent_equivalence_notes",
        &harness.intent_equivalence_notes,
    );

    if !harness.required_for_alternate_implementations {
        errors.push(GovernancePackInstantiationValidationError {
            field: "conformance_harness.required_for_alternate_implementations",
            message: "alternate implementations must require the conformance harness",
        });
    }

    for required_check in [
        "check.gate_semantics_equivalence",
        "check.intent_equivalence_notes",
        "check.action_write_box_law",
    ] {
        if !contains_exact(&harness.check_refs, required_check) {
            errors.push(GovernancePackInstantiationValidationError {
                field: "conformance_harness.check_refs",
                message: "conformance harness must prove gate semantics, intent equivalence, and action/write-box law",
            });
        }
    }
}

fn deterministic_naming(pack: &GovernancePackInstantiationV1) -> bool {
    pack.project_identity.naming_style == NamingStyle::Underscore
        && is_underscore_name(&pack.project_identity.project_code)
        && pack.path_policy.deterministic_paths
        && pack
            .project_identity
            .codex_authority_filename
            .contains(&pack.project_identity.project_code)
        && pack
            .project_identity
            .master_spec_filename
            .contains(&pack.project_identity.project_code)
}

fn kernel_law_compatible(pack: &GovernancePackInstantiationV1) -> bool {
    pack.manifest_items.iter().all(|item| {
        item.authority_effect != AuthorityEffect::EventLedgerAuthorityWrite
            && (item.authority_effect == AuthorityEffect::ProjectionOnly || item.promotion_required)
            && item
                .imported_overlay_boundary
                .as_ref()
                .map(|boundary| {
                    !boundary.replaces_native_governance
                        && boundary.allowed_authority_effect
                            != AuthorityEffect::EventLedgerAuthorityWrite
                        && boundary.allowed_authority_effect == item.authority_effect
                })
                .unwrap_or(true)
    })
}

fn is_underscore_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn is_absolute_or_unc(path: &str) -> bool {
    path.starts_with('/')
        || path.starts_with('\\')
        || path.as_bytes().get(1) == Some(&b':')
        || path.starts_with("//")
}

fn contains_blank_space(value: &str) -> bool {
    value.chars().any(|ch| ch.is_whitespace())
}

fn contains_handshake(value: &str) -> bool {
    value.to_ascii_lowercase().contains("handshake")
}

fn require_non_empty(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(GovernancePackInstantiationValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<GovernancePackInstantiationValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(GovernancePackInstantiationValidationError {
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

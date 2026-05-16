use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    governance_pack_instantiation::{
        project_governance_pack_instantiation, validate_governance_pack_instantiation,
        GovernancePackComponentKind, GovernancePackConformanceHarnessV1,
        GovernancePackInstantiationV1, GovernancePackManifestItemV1,
        GovernancePackOverlayBoundaryV1, GovernancePackOverlayKind, GovernancePackPathPolicyV1,
        GovernanceProjectIdentityV1, NamingStyle,
    },
};

#[test]
fn kernel_governance_pack_instantiation_projects_identity_manifest_and_paths() {
    let pack = sample_pack();
    validate_governance_pack_instantiation(&pack).expect("governance pack validates");

    let projection = project_governance_pack_instantiation(&pack).expect("projection builds");

    assert_eq!(projection.project_code, "nova_lab");
    assert!(projection.repo_relative_paths_only);
    assert!(projection.deterministic_naming);
    assert!(projection.kernel_law_compatible);
    assert!(projection
        .manifest_item_ids
        .contains(&"component.project_identity".to_string()));
    assert!(projection
        .target_path_templates
        .iter()
        .all(|path| path.contains("{project_code}")));
    assert!(!projection.mutates_authority);
}

#[test]
fn kernel_governance_pack_instantiation_projects_overlay_and_conformance_boundaries() {
    let projection = project_governance_pack_instantiation(&sample_pack()).expect("projection");

    assert!(projection
        .imported_overlay_ids
        .contains(&"overlay.software_delivery.reference".to_string()));
    assert!(projection
        .conformance_check_refs
        .contains(&"check.gate_semantics_equivalence".to_string()));
    assert!(projection
        .conformance_check_refs
        .contains(&"check.action_write_box_law".to_string()));
}

#[test]
fn kernel_governance_pack_instantiation_rejects_hardcoded_paths_and_overlay_authority() {
    let mut pack = sample_pack();
    pack.project_identity.codex_authority_filename = "Handshake_CODEX.md".to_string();
    pack.manifest_items[0].target_path_template =
        "C:/Handshake/GOV/project_identity.json".to_string();
    pack.manifest_items[4].authority_effect = AuthorityEffect::EventLedgerAuthorityWrite;
    pack.manifest_items[4]
        .imported_overlay_boundary
        .as_mut()
        .expect("overlay boundary")
        .replaces_native_governance = true;
    pack.conformance_harness
        .required_for_alternate_implementations = false;

    let errors = validate_governance_pack_instantiation(&pack).expect_err("unsafe pack must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "project_identity.codex_authority_filename"));
    assert!(errors
        .iter()
        .any(|error| error.field == "manifest_items.target_path_template"));
    assert!(errors
        .iter()
        .any(|error| error.field == "manifest_items.authority_effect"));
    assert!(errors
        .iter()
        .any(|error| error.field == "manifest_items.imported_overlay_boundary"));
    assert!(errors
        .iter()
        .any(|error| error.field == "conformance_harness.required_for_alternate_implementations"));
}

#[test]
fn kernel_governance_pack_instantiation_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.governance_pack_instantiation.project")
        .expect("governance pack instantiation projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "governance_pack_path_policy"));
}

fn sample_pack() -> GovernancePackInstantiationV1 {
    GovernancePackInstantiationV1 {
        schema_id: "hsk.kernel.governance_pack_instantiation@1".to_string(),
        pack_id: "governance-pack-mt042".to_string(),
        folded_stub_ids: vec!["WP-1-Governance-Pack-v1".to_string()],
        project_identity: GovernanceProjectIdentityV1 {
            project_code: "nova_lab".to_string(),
            project_display_name: "Nova Lab".to_string(),
            codex_authority_filename: "nova_lab_CODEX.md".to_string(),
            master_spec_filename: "nova_lab_MASTER_SPEC.md".to_string(),
            naming_style: NamingStyle::Underscore,
            language_layout_profile: "rust_workspace".to_string(),
            external_tool_paths_config_ref: "config://nova_lab/external-tools".to_string(),
        },
        path_policy: GovernancePackPathPolicyV1 {
            deterministic_paths: true,
            repo_relative_paths_only: true,
            no_blank_spaces: true,
            no_handshake_hardcoding: true,
            external_tool_paths_project_configured: true,
        },
        manifest_items: vec![
            item(
                "component.project_identity",
                GovernancePackComponentKind::ProjectIdentity,
                "{project_code}/governance/project_identity.json",
                AuthorityEffect::PrePromotionEvidenceOnly,
                true,
                None,
            ),
            item(
                "component.codex_authority",
                GovernancePackComponentKind::CodexAuthority,
                "{project_code}/governance/{project_code}_CODEX.md",
                AuthorityEffect::PrePromotionEvidenceOnly,
                true,
                None,
            ),
            item(
                "component.master_spec",
                GovernancePackComponentKind::MasterSpec,
                "{project_code}/spec/{project_code}_MASTER_SPEC.md",
                AuthorityEffect::PrePromotionEvidenceOnly,
                true,
                None,
            ),
            item(
                "component.conformance_harness",
                GovernancePackComponentKind::ConformanceHarness,
                "{project_code}/tests/governance_pack_conformance.json",
                AuthorityEffect::ProjectionOnly,
                false,
                None,
            ),
            item(
                "component.software_delivery_overlay",
                GovernancePackComponentKind::ImportedOverlay,
                "{project_code}/overlays/software_delivery/reference_overlay.json",
                AuthorityEffect::PrePromotionEvidenceOnly,
                true,
                Some(GovernancePackOverlayBoundaryV1 {
                    overlay_id: "overlay.software_delivery.reference".to_string(),
                    overlay_kind: GovernancePackOverlayKind::SoftwareDeliveryProfileOverlay,
                    profile_id: "software_delivery".to_string(),
                    replaces_native_governance: false,
                    allowed_authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
                    expected_write_box_schema_id: "hsk.write_box.governance_pack_overlay@1"
                        .to_string(),
                }),
            ),
        ],
        conformance_harness: GovernancePackConformanceHarnessV1 {
            harness_id: "governance-pack-conformance".to_string(),
            required_for_alternate_implementations: true,
            check_refs: vec![
                "check.gate_semantics_equivalence".to_string(),
                "check.intent_equivalence_notes".to_string(),
                "check.action_write_box_law".to_string(),
            ],
            intent_equivalence_notes: vec![
                "Imported overlay preserves software-delivery intent without replacing native governance"
                    .to_string(),
            ],
        },
        product_authority_refs: vec![
            "kernel.action_catalog".to_string(),
            "kernel.write_boxes".to_string(),
            "kernel.governance_overlay_boundary".to_string(),
            "kernel.workflow_transition_registry".to_string(),
            "kernel.local_first_mcp_posture".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Governance-Pack-v1.contract.json".to_string(),
        ],
    }
}

fn item(
    item_id: &str,
    component_kind: GovernancePackComponentKind,
    target_path_template: &str,
    authority_effect: AuthorityEffect,
    promotion_required: bool,
    imported_overlay_boundary: Option<GovernancePackOverlayBoundaryV1>,
) -> GovernancePackManifestItemV1 {
    GovernancePackManifestItemV1 {
        item_id: item_id.to_string(),
        component_kind,
        template_ref: format!("template://{item_id}"),
        target_path_template: target_path_template.to_string(),
        write_box_kind: "GovernancePackInstantiationBox".to_string(),
        authority_effect,
        promotion_required,
        imported_overlay_boundary,
    }
}

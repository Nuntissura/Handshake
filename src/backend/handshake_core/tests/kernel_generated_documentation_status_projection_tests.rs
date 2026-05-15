use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    generated_documentation_status_projection::{
        build_kernel002_generated_documentation_status_projection,
        project_generated_documentation_status, validate_generated_documentation_status_projection,
        GeneratedStatusFailureState, GeneratedStatusSourceKind, GeneratedStatusTargetKind,
        ManualStatusEditDisposition, GENERATED_DOCUMENTATION_STATUS_PROJECTION_RESULT_SCHEMA_ID,
        GENERATED_DOCUMENTATION_STATUS_PROJECTION_SCHEMA_ID,
    },
};

#[test]
fn generated_status_projection_contract_declares_authoritative_sources_and_targets() {
    let contract = build_kernel002_generated_documentation_status_projection();

    validate_generated_documentation_status_projection(&contract)
        .expect("generated status projection contract validates");

    assert_eq!(
        contract.schema_id,
        GENERATED_DOCUMENTATION_STATUS_PROJECTION_SCHEMA_ID
    );
    for source_kind in [
        GeneratedStatusSourceKind::Contract,
        GeneratedStatusSourceKind::ReceiptLedger,
        GeneratedStatusSourceKind::RuntimeState,
        GeneratedStatusSourceKind::ValidationOutput,
    ] {
        assert!(
            contract
                .sources
                .iter()
                .any(|source| source.kind == source_kind && source.authoritative),
            "{source_kind:?} must be an authoritative projection source"
        );
    }
    assert!(contract
        .sources
        .iter()
        .all(|source| is_sha256_digest(&source.source_hash)));

    for target_kind in [
        GeneratedStatusTargetKind::PacketStatus,
        GeneratedStatusTargetKind::MicroTaskStatus,
        GeneratedStatusTargetKind::TaskBoardRow,
        GeneratedStatusTargetKind::TraceabilityRow,
        GeneratedStatusTargetKind::DccWorkView,
        GeneratedStatusTargetKind::MirrorDoc,
        GeneratedStatusTargetKind::OperatorSummary,
    ] {
        assert!(
            contract
                .targets
                .iter()
                .any(|target| target.kind == target_kind
                    && target.generated_from_machine_authority
                    && !target.authority_mutation),
            "{target_kind:?} must regenerate as a projection-only target"
        );
    }
    assert!(contract.targets.iter().any(|target| {
        target.kind == GeneratedStatusTargetKind::MicroTaskStatus
            && target.target_ref.ends_with("/MT-055.md#status")
            && !target.target_ref.contains("/microtasks/")
    }));
    assert!(contract.targets.iter().any(|target| {
        target.kind == GeneratedStatusTargetKind::TaskBoardRow
            && target.target_ref
                == ".GOV/roles_shared/records/TASK_BOARD.md#WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
    }));
    assert!(contract.targets.iter().any(|target| {
        target.kind == GeneratedStatusTargetKind::TraceabilityRow
            && target.target_ref
                == ".GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md#WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
    }));
}

#[test]
fn generated_status_projection_derives_operator_views_without_manual_status_authority() {
    let contract = build_kernel002_generated_documentation_status_projection();

    let projection = project_generated_documentation_status(&contract)
        .expect("generated status projection can be derived");

    assert_eq!(
        projection.schema_id,
        GENERATED_DOCUMENTATION_STATUS_PROJECTION_RESULT_SCHEMA_ID
    );
    assert_eq!(projection.source_kinds.len(), 4);
    assert_eq!(projection.target_kinds.len(), 7);
    assert!(!projection.generated_docs_are_authority);
    assert!(!projection.manual_status_edits_are_authority);
    assert!(!projection.mutates_task_board);
    assert!(projection
        .allowed_advisory_action_ids
        .contains(&"kernel.mirror_advisory.capture".to_string()));
    assert!(projection
        .allowed_advisory_action_ids
        .contains(&"kernel.mirror_advisory.normalize".to_string()));
    assert_eq!(
        projection.direct_edit_denial_action_id,
        "kernel.direct_edit.deny"
    );
    assert!(projection
        .source_lineage_refs
        .iter()
        .any(|source| source.contains("receipt-ledger")));
    assert!(projection
        .source_lineage_refs
        .iter()
        .any(|source| source.contains("validation-output")));
}

#[test]
fn generated_status_projection_rejects_non_authority_and_direct_manual_status_edits() {
    let mut contract = build_kernel002_generated_documentation_status_projection();
    contract.sources[0].kind = GeneratedStatusSourceKind::PacketProse;
    let errors = validate_generated_documentation_status_projection(&contract)
        .expect_err("packet prose cannot be a status source of truth");
    assert!(errors.iter().any(|error| error.field == "sources.kind"));

    let mut contract = build_kernel002_generated_documentation_status_projection();
    contract
        .sources
        .retain(|source| source.kind != GeneratedStatusSourceKind::ValidationOutput);
    let errors = validate_generated_documentation_status_projection(&contract)
        .expect_err("validation output source is required");
    assert!(errors.iter().any(|error| error.field == "sources"));

    let mut contract = build_kernel002_generated_documentation_status_projection();
    contract.targets[0].manual_edit_disposition = ManualStatusEditDisposition::AcceptAsAuthority;
    let errors = validate_generated_documentation_status_projection(&contract)
        .expect_err("manual status edits must not become authority");
    assert!(errors
        .iter()
        .any(|error| error.field == "targets.manual_edit_disposition"));
}

#[test]
fn generated_status_projection_json_round_trips() {
    let contract = build_kernel002_generated_documentation_status_projection();
    let projection = project_generated_documentation_status(&contract).expect("projection derives");

    let json = serde_json::to_string(&contract).expect("contract serializes");
    let decoded: handshake_core::kernel::generated_documentation_status_projection::GeneratedDocumentationStatusProjectionV1 =
        serde_json::from_str(&json).expect("contract deserializes");
    assert_eq!(decoded, contract);

    let json = serde_json::to_string(&projection).expect("projection serializes");
    let decoded: handshake_core::kernel::generated_documentation_status_projection::GeneratedDocumentationStatusProjectionResultV1 =
        serde_json::from_str(&json).expect("projection deserializes");
    assert_eq!(decoded, projection);
}

#[test]
fn generated_status_projection_records_failure_states_and_existing_authority_refs() {
    let contract = build_kernel002_generated_documentation_status_projection();

    validate_generated_documentation_status_projection(&contract)
        .expect("generated status projection contract validates");

    for failure_state in [
        GeneratedStatusFailureState::ManualStatusEditAttempt,
        GeneratedStatusFailureState::NonAuthoritativeSourceUsed,
        GeneratedStatusFailureState::StaleGeneratedDocument,
        GeneratedStatusFailureState::MissingReceipt,
        GeneratedStatusFailureState::MissingRuntimeState,
        GeneratedStatusFailureState::MissingValidationOutput,
        GeneratedStatusFailureState::ProjectionHashDrift,
        GeneratedStatusFailureState::DirectTaskBoardMutation,
    ] {
        assert!(contract.failure_states.contains(&failure_state));
    }

    for authority_ref in [
        "kernel.task_contract_lifecycle",
        "kernel.software_delivery_runtime_truth",
        "kernel.markdown_mirror_sync_drift_guard",
        "kernel.dcc_layout_projection_registry",
        "kernel.role_mailbox_triage_queue",
        "kernel.direct_edit_guard",
        "kernel.action_catalog",
    ] {
        assert!(contract
            .product_authority_refs
            .contains(&authority_ref.to_string()));
    }
}

#[test]
fn kernel_action_catalog_exposes_generated_status_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.generated_documentation_status_projection.project")
        .expect("generated documentation/status projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "generated_status_machine_authority_sources"));
}

fn is_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.chars().all(|ch| ch.is_ascii_hexdigit()))
}

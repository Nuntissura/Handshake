use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    mechanical_contract_generation::{
        build_kernel002_mechanical_contract_generation, validate_mechanical_contract_generation,
        GeneratedContractArtifactKind, MechanicalContractFailureState,
        MechanicalContractOperationKind, PreservedContractField,
    },
};

#[test]
fn mechanical_generation_defines_stub_promotion_and_mt_extraction_operations() {
    let generation = build_kernel002_mechanical_contract_generation();

    validate_mechanical_contract_generation(&generation)
        .expect("mechanical contract generation validates");

    assert_eq!(
        generation.schema_id,
        "hsk.kernel.mechanical_contract_generation@1"
    );
    let promotion = generation
        .operation(MechanicalContractOperationKind::StubToWorkPacketPromotion)
        .expect("stub promotion operation exists");
    assert_eq!(
        promotion.source_schema_id,
        "hsk.work_packet_stub_contract@1"
    );
    assert_eq!(promotion.target_schema_id, "hsk.work_packet_contract@1");
    assert_eq!(promotion.action_id, "kernel.work_packet_contract.activate");
    assert_eq!(
        promotion.command.command_line,
        "just task-packet-stub-contracts --all"
    );
    assert!(promotion
        .transition_action_ids
        .contains(&"kernel.stub_contract.prepare_promotion".to_string()));

    let extraction = generation
        .operation(MechanicalContractOperationKind::WorkPacketToMicrotaskExtraction)
        .expect("microtask extraction operation exists");
    assert_eq!(extraction.source_schema_id, "hsk.work_packet_contract@1");
    assert_eq!(extraction.target_schema_id, "hsk.microtask_contract@1");
    assert_eq!(extraction.action_id, "kernel.microtask_contract.extract");
    assert_eq!(
        extraction.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
}

#[test]
fn mechanical_generation_records_exact_durable_command_receipts_for_preuse_acceptance() {
    let generation = build_kernel002_mechanical_contract_generation();

    validate_mechanical_contract_generation(&generation)
        .expect("mechanical contract generation validates");

    for command in [
        "just task-packet-stub-contracts --all",
        "just build-order-sync",
        "just gov-check",
    ] {
        let receipt = generation
            .durable_command_receipts
            .iter()
            .find(|receipt| receipt.command_line == command)
            .expect("exact durable command receipt exists");
        assert!(receipt
            .receipt_ref
            .starts_with("receipt://mechanical-contract-generation/"));
        assert_eq!(receipt.workdir_ref, "repo-root://");
        assert_eq!(receipt.script_resolution, "resolve-script-ref-from-workdir");
        assert!(receipt.blocks_activation_on_failure);
    }
    assert!(!generation
        .durable_command_receipts
        .iter()
        .any(|receipt| receipt.command_line.contains("--check")));
}

#[test]
fn mechanical_generation_receipts_resolve_workdir_and_existing_script_refs() {
    let generation = build_kernel002_mechanical_contract_generation();
    validate_mechanical_contract_generation(&generation)
        .expect("mechanical contract generation validates");
    let repo_root = repo_root();

    for receipt in &generation.durable_command_receipts {
        assert_eq!(receipt.workdir_ref, "repo-root://");
        assert_eq!(receipt.script_resolution, "resolve-script-ref-from-workdir");
        assert!(
            repo_root.join(&receipt.script_ref).exists(),
            "script ref must exist or be intentionally resolved from workdir: {}",
            receipt.script_ref
        );
    }

    let stub_receipt = generation
        .durable_command_receipts
        .iter()
        .find(|receipt| receipt.command_line == "just task-packet-stub-contracts --all")
        .expect("task-packet-stub-contracts exact receipt exists");
    assert_eq!(stub_receipt.workdir_ref, "repo-root://");
    assert_eq!(
        repo_root.join(&stub_receipt.script_ref),
        repo_root.join(".GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs")
    );
}

#[test]
fn mechanical_generation_preserves_required_contract_detail_and_provenance() {
    let generation = build_kernel002_mechanical_contract_generation();

    validate_mechanical_contract_generation(&generation)
        .expect("mechanical contract generation validates");

    for operation in &generation.operations {
        for required in [
            PreservedContractField::OperatorIntent,
            PreservedContractField::SourceHashes,
            PreservedContractField::FoldedDetails,
            PreservedContractField::Dependencies,
            PreservedContractField::Constraints,
            PreservedContractField::AcceptanceCriteria,
            PreservedContractField::Verification,
            PreservedContractField::StatusProvenance,
        ] {
            assert!(operation.required_preserved_fields.contains(&required));
        }
        assert!(operation.generated_artifacts.iter().all(|artifact| {
            !artifact.source_contract_id.is_empty() && is_sha256_digest(&artifact.source_hash)
        }));
        assert!(operation
            .status_provenance_fields
            .contains(&"markdown_projection.source_hash".to_string()));
        assert!(operation
            .status_provenance_fields
            .contains(&"markdown_projection.projection_hash".to_string()));
        assert!(operation.source_fold_map.iter().all(
            |entry| !entry.source_contract_id.is_empty() && !entry.destination_field.is_empty()
        ));
        assert!(operation
            .field_preservation_manifest
            .iter()
            .all(|entry| entry.required && !entry.source_path.is_empty()));
    }

    let extraction = generation
        .operation(MechanicalContractOperationKind::WorkPacketToMicrotaskExtraction)
        .expect("microtask extraction operation exists");
    assert!(extraction
        .generated_artifacts
        .iter()
        .any(|artifact| artifact.kind == GeneratedContractArtifactKind::MicrotaskContract));
    assert!(extraction
        .generated_artifacts
        .iter()
        .any(|artifact| artifact.kind == GeneratedContractArtifactKind::TaskBoardProjection));
    assert!(extraction.generated_artifacts.iter().any(|artifact| {
        artifact.kind == GeneratedContractArtifactKind::TraceabilityProjection
            && artifact.path_template
                == ".GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md#{wp_id}"
    }));
    assert!(extraction.generated_artifacts.iter().any(|artifact| {
        artifact.kind == GeneratedContractArtifactKind::TaskBoardProjection
            && artifact.path_template == ".GOV/roles_shared/records/TASK_BOARD.md#{wp_id}"
    }));
}

#[test]
fn mechanical_generation_rejects_detail_loss_or_missing_hashes() {
    let mut generation = build_kernel002_mechanical_contract_generation();
    generation.operations[0]
        .required_preserved_fields
        .retain(|field| *field != PreservedContractField::OperatorIntent);
    let errors = validate_mechanical_contract_generation(&generation)
        .expect_err("operator intent preservation is required");
    assert!(errors
        .iter()
        .any(|error| error.field == "operations.required_preserved_fields"));

    let mut generation = build_kernel002_mechanical_contract_generation();
    generation.operations[1].generated_artifacts[0]
        .source_contract_id
        .clear();
    let errors = validate_mechanical_contract_generation(&generation)
        .expect_err("generated artifacts must cite source contract ids");
    assert!(errors
        .iter()
        .any(|error| error.field == "operations.generated_artifacts.source_contract_id"));

    let mut generation = build_kernel002_mechanical_contract_generation();
    generation.operations[1].generated_artifacts[0].source_hash = "zzzzzzzzzzzzzzzz".to_string();
    let errors = validate_mechanical_contract_generation(&generation)
        .expect_err("generated artifacts must reject fake source hashes");
    assert!(errors
        .iter()
        .any(|error| error.field == "operations.generated_artifacts.source_hash"));

    assert!(generation
        .failure_states
        .contains(&MechanicalContractFailureState::FoldedDetailLoss));
    assert!(generation
        .failure_states
        .contains(&MechanicalContractFailureState::StatusProvenanceLoss));
}

#[test]
fn mechanical_generation_json_round_trips() {
    let generation = build_kernel002_mechanical_contract_generation();

    let json = serde_json::to_string(&generation).expect("mechanical generation serializes");
    let decoded: handshake_core::kernel::mechanical_contract_generation::MechanicalContractGenerationV1 =
        serde_json::from_str(&json).expect("mechanical generation deserializes");

    assert_eq!(decoded, generation);
}

#[test]
fn kernel_action_catalog_exposes_mechanical_promotion_and_extraction_actions() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let activation = catalog
        .action("kernel.work_packet_contract.activate")
        .expect("catalog contains work packet activation action");
    assert_eq!(
        activation.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert!(activation
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "stub_promotion_preserves_operator_intent"));

    let extraction = catalog
        .action("kernel.microtask_contract.extract")
        .expect("catalog contains microtask extraction action");
    assert_eq!(
        extraction.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert!(extraction
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "microtask_extraction_preserves_source_hashes"));
}

fn is_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.chars().all(|ch| ch.is_ascii_hexdigit()))
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root is three levels above handshake_core")
        .to_path_buf()
}

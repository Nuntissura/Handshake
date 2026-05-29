use std::{fs, path::PathBuf};

use chrono::Utc;
use handshake_core::{
    kernel::{
        action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
        action_envelope::{ApprovalPosture, AuthorityEffect},
        write_boxes::{
            validate_write_box_common, ArtifactBox, WriteBoxCommon, WriteBoxKind,
            WriteBoxLifecycleState, WriteBoxOwnerRef, WriteBoxPayloadRef, WriteBoxReplayMetadataV1,
            WriteBoxTargetRef, WriteBoxValidationState, WriteBoxValidationStatus,
        },
    },
    model_runtime::{
        BaseModelTag, ModelCapabilities, ModelId, ModelRegistration, ModelRegistry, OperatorId,
        ProviderKind, RuntimeBinding,
    },
};
use uuid::Uuid;

fn capabilities_with_activation_steering(enabled: bool) -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_activation_steering: enabled,
        supports_speculative_draft: false,
        supports_eagle3: false,
        ..Default::default()
    }
}

fn registration(
    model_id: ModelId,
    runtime_binding: RuntimeBinding,
    declared_capabilities: ModelCapabilities,
) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: PathBuf::from("fixtures/models/local-test.gguf"),
        sha256: [7; 32],
        runtime_binding,
        declared_capabilities,
        base_model_tag: BaseModelTag::new("local-test-base"),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("operator-ilja"),
        provider: ProviderKind::Local,
    }
}

#[test]
fn model_registry_tests_register_llamacpp_lookup_and_deterministic_list() {
    let first_id = ModelId::new_v7();
    let second_id = ModelId::new_v7();
    let first = registration(
        first_id,
        RuntimeBinding::LlamaCpp,
        capabilities_with_activation_steering(false),
    );
    let second = registration(
        second_id,
        RuntimeBinding::LlamaCpp,
        capabilities_with_activation_steering(false),
    );
    let mut registry = ModelRegistry::default();

    registry.register(second).expect("second registration");
    registry
        .register(first.clone())
        .expect("first registration");

    assert_eq!(first_id.as_uuid().get_version_num(), 7);
    assert_eq!(registry.lookup(first_id), Some(&first));

    let listed_ids = registry
        .list()
        .into_iter()
        .map(|registration| registration.model_id.to_string())
        .collect::<Vec<_>>();
    let mut sorted = listed_ids.clone();
    sorted.sort();
    assert_eq!(listed_ids, sorted, "list order must be deterministic");
}

#[test]
fn model_registry_tests_rebind_requires_unload_before_binding_changes() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(
            model_id,
            RuntimeBinding::LlamaCpp,
            capabilities_with_activation_steering(false),
        ))
        .expect("registration succeeds");

    registry
        .mark_loaded(model_id)
        .expect("load marker succeeds");
    let loaded_err = registry
        .rebind(model_id, RuntimeBinding::Candle)
        .expect_err("loaded rebind must fail");
    assert!(
        loaded_err.to_string().contains("loaded"),
        "error should explain loaded rebind denial: {loaded_err}"
    );

    registry
        .mark_unloaded(model_id)
        .expect("unload marker succeeds");
    registry
        .rebind(model_id, RuntimeBinding::Candle)
        .expect("unloaded rebind succeeds");

    assert_eq!(
        registry.lookup(model_id).map(|reg| reg.runtime_binding),
        Some(RuntimeBinding::Candle)
    );
}

#[test]
fn model_registry_tests_rejects_capability_binding_mismatch() {
    let mut registry = ModelRegistry::default();
    let err = registry
        .register(registration(
            ModelId::new_v7(),
            RuntimeBinding::LlamaCpp,
            capabilities_with_activation_steering(true),
        ))
        .expect_err("llama.cpp registrations cannot claim activation steering");

    let message = err.to_string();
    assert!(message.contains("activation_steering"), "{message}");
    assert!(message.contains("llama_cpp"), "{message}");
}

#[test]
fn model_registry_tests_rejects_non_v7_model_ids_and_non_local_providers() {
    let mut registry = ModelRegistry::default();
    let err = registry
        .register(registration(
            ModelId::from(Uuid::nil()),
            RuntimeBinding::Candle,
            capabilities_with_activation_steering(true),
        ))
        .expect_err("registry must reject non-v7 ids");
    assert!(err.to_string().contains("UUID v7"), "{err}");

    let mut external = registration(
        ModelId::new_v7(),
        RuntimeBinding::Candle,
        capabilities_with_activation_steering(true),
    );
    external.provider = ProviderKind::ExternalCompat;
    let err = registry
        .register(external)
        .expect_err("local adapter registry must reject ExternalCompat");
    assert!(err.to_string().contains("local provider"), "{err}");
}

#[test]
fn model_registry_tests_catalog_register_action_routes_through_write_box_v1() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let register = catalog
        .action("kernel.model_runtime.register_model")
        .expect("register-model action");
    assert_eq!(
        register.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        register.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );
    assert!(register
        .capability_requirements
        .iter()
        .any(|capability| capability.capability_id == "kernel.model_runtime.register_model"));
    assert!(register.expected_write_boxes.iter().any(|write_box| {
        write_box.write_box_kind == "ArtifactBox"
            && write_box.write_box_schema_id == "hsk.write_box.artifact@1"
            && write_box.target_id == "model_runtime_registration"
    }));

    let query = catalog
        .action("kernel.model_runtime.list_registrations")
        .expect("list-registrations query action");
    assert_eq!(query.authority_effect, AuthorityEffect::ProjectionOnly);
    assert_eq!(query.approval_posture, ApprovalPosture::NoApprovalRequired);
    assert!(query.expected_write_boxes.iter().any(|write_box| {
        write_box.write_box_kind == "ReadOnlyProjectionBox"
            && write_box.write_box_schema_id == "hsk.write_box.readonly_projection@1"
            && write_box.target_id == "model_runtime_registration"
    }));
}

#[test]
fn model_registry_tests_registration_artifact_box_uses_write_box_v1_contract() {
    let common = model_registration_common_write_box();
    validate_write_box_common(&common).expect("model registration write box common validates");
    assert_eq!(common.kind, WriteBoxKind::Artifact);
    assert_eq!(common.schema_version, "hsk.write_box.artifact@1");
    assert_eq!(
        common.target_refs[0].target_id,
        "model_runtime_registration"
    );
    assert_eq!(
        common.operation_payload_refs[0].payload_kind,
        "model_registration"
    );

    let artifact = ArtifactBox {
        common,
        artifact_ref: "artifact://model-runtime/registrations/model-001.json".to_string(),
    };
    assert_eq!(
        artifact.common.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        artifact.artifact_ref,
        artifact.common.operation_payload_refs[0].payload_ref
    );
}

#[test]
fn model_registry_tests_registry_has_no_direct_persistence_write_path() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/model_runtime/registry.rs"),
    )
    .expect("read registry source");

    let banned_tokens = vec![
        "std::fs::write".to_string(),
        "File::create".to_string(),
        "OpenOptions".to_string(),
        "sqlx::query".to_string(),
        "INSERT INTO".to_string(),
        "UPDATE ".to_string(),
        "DELETE FROM".to_string(),
        ["sql", "ite"].concat(),
    ];

    for banned in banned_tokens {
        assert!(
            !source.contains(&banned),
            "registry must not expose direct persistence token `{banned}`"
        );
    }
}

fn model_registration_common_write_box() -> WriteBoxCommon {
    WriteBoxCommon {
        write_box_id: "wb-model-runtime-registration-001".to_string(),
        kind: WriteBoxKind::Artifact,
        schema_version: "hsk.write_box.artifact@1".to_string(),
        workspace_id: "kernel-runtime-workspace".to_string(),
        owner: WriteBoxOwnerRef {
            actor_id: "operator-ilja".to_string(),
            actor_kind: "operator".to_string(),
            role_id: "OPERATOR".to_string(),
        },
        crdt_site_id: "site-kernel-builder".to_string(),
        target_refs: vec![WriteBoxTargetRef {
            target_id: "model_runtime_registration".to_string(),
            target_kind: "model_registry".to_string(),
            authority_class: "pre_promotion_artifact".to_string(),
        }],
        base_snapshot_refs: vec!["snapshot://model-runtime/registry/base-empty".to_string()],
        intent_summary: "Register per-model runtime binding through WriteBoxV1".to_string(),
        operation_payload_refs: vec![WriteBoxPayloadRef {
            payload_id: "payload-model-registration-001".to_string(),
            payload_kind: "model_registration".to_string(),
            payload_ref: "artifact://model-runtime/registrations/model-001.json".to_string(),
            payload_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
        }],
        lifecycle_state: WriteBoxLifecycleState::Open,
        allowed_transitions: vec![
            WriteBoxLifecycleState::ReadyForValidation,
            WriteBoxLifecycleState::Denied,
        ],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        evidence_refs: vec!["evidence://model-runtime/registration/contract".to_string()],
        receipt_refs: vec![
            "receipt://write-box-created/wb-model-runtime-registration-001".to_string(),
        ],
        denial_receipt_refs: Vec::new(),
        promotion_receipt_refs: Vec::new(),
        validation_status: WriteBoxValidationStatus {
            state: WriteBoxValidationState::Pending,
            check_ids: vec![
                "model_runtime_registration_schema".to_string(),
                "model_runtime_binding_capability_consistency".to_string(),
            ],
        },
        projection_rules: vec![
            "dcc.write_box.queue".to_string(),
            "dcc.model_runtime_registration.preview".to_string(),
        ],
        replay_metadata: WriteBoxReplayMetadataV1 {
            replay_plan_ref: "replay://model-runtime/register-model".to_string(),
            replay_order_key: "model-runtime/registration/00000000000000000001".to_string(),
            idempotency_key: "model-runtime-register-model:model-001".to_string(),
            source_event_refs: vec![
                "eventledger://kernel/model-runtime-registration-requested".to_string()
            ],
        },
    }
}

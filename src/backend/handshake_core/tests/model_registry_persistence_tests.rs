//! Embedded-SurrealDB behavioral proof for the exact-scope ModelRuntime registry.
//!
//! The suite uses one cloned product-store handle and canonical
//! `kernel_event_ledger` receipts. It tests behavior rather than mechanisms
//! belonging to another database engine.

use std::path::PathBuf;

use chrono::Utc;
use handshake_core::{
    kernel::KernelActor,
    model_runtime::{
        BaseModelTag, ExplicitModelRuntimeRebind, ModelCapabilities, ModelId, ModelRegistration,
        ModelRegistryLifecycleState, ModelRegistryPersistenceError, ModelRegistryStore,
        ModelRuntimeRole, ModelRuntimeSelection, ModelRuntimeSelectionPurpose, OperatorId,
        ProviderKind, RoleBoundModelRegistration, RuntimeBinding,
        MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID, MODEL_RUNTIME_REGISTRY_SCHEMA_ID,
    },
    storage::surreal::{
        bootstrap_loom_search_schema, bootstrap_model_registry_schema, bootstrap_schema,
        SurrealStorage, SurrealStorageConfig,
    },
    swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
        OwnerAccountId, WorkspaceScopeRef,
    },
};
use tempfile::TempDir;

fn exact_scope(workspace: &str) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(workspace).expect("valid workspace id"),
    }
}

fn with_workspace(
    source: &ExactResourceScopeAttribution,
    workspace: &str,
) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: source.owner_account_id,
        actor_principal_id: source.actor_principal_id,
        authenticated_session_id: source.authenticated_session_id,
        access_space_id: source.access_space_id,
        workspace_id: WorkspaceScopeRef::new(workspace).expect("valid workspace id"),
    }
}

fn capabilities(role: ModelRuntimeRole, binding: RuntimeBinding) -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: role == ModelRuntimeRole::Completion,
        supports_activation_steering: role == ModelRuntimeRole::Completion
            && binding == RuntimeBinding::Candle,
        supports_embedding: role == ModelRuntimeRole::Embedding,
        embedding_dimension: (role == ModelRuntimeRole::Embedding).then_some(768),
        ..ModelCapabilities::default()
    }
}

fn registration(
    artifact_byte: u8,
    role: ModelRuntimeRole,
    binding: RuntimeBinding,
    label: &str,
) -> RoleBoundModelRegistration {
    let registration = ModelRegistration {
        model_id: ModelId::new_v7(),
        artifact_path: PathBuf::from(format!("fixtures/models/{label}.safetensors")),
        sha256: [artifact_byte; 32],
        runtime_binding: binding,
        declared_capabilities: capabilities(role, binding),
        base_model_tag: BaseModelTag::new(label),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("model-registry-surreal-proof"),
        provider: ProviderKind::Local,
    };
    match role {
        ModelRuntimeRole::Completion => RoleBoundModelRegistration::completion(registration),
        ModelRuntimeRole::Embedding => RoleBoundModelRegistration::embedding(registration),
    }
}

fn selection(registration: &RoleBoundModelRegistration) -> ModelRuntimeSelection {
    ModelRuntimeSelection {
        artifact_sha256: registration.registration.sha256,
        runtime_binding: registration.registration.runtime_binding,
        runtime_role: registration.runtime_role,
        declared_capabilities: registration.registration.declared_capabilities.clone(),
        provider: registration.registration.provider,
    }
}

async fn open_store(config: SurrealStorageConfig) -> (SurrealStorage, ModelRegistryStore) {
    let storage = SurrealStorage::open(config)
        .await
        .expect("open embedded SurrealDB registry authority");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap canonical product schema");
    bootstrap_model_registry_schema(&storage)
        .await
        .expect("bootstrap canonical model registry schema");
    (storage.clone(), ModelRegistryStore::new(storage))
}

async fn prepare_scope(store: &ModelRegistryStore, scope: &ExactResourceScopeAttribution) {
    store
        .ensure_workspace_for_tests(scope)
        .await
        .expect("create exact workspace predecessor");
}

#[test]
fn loom_schema_does_not_redefine_canonical_model_registry_authority() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let loom =
        std::fs::read_to_string(manifest.join("src/storage/surreal/loom_search_schema.surql"))
            .expect("read Loom extension schema");
    let canonical =
        std::fs::read_to_string(manifest.join("src/storage/surreal/model_registry_schema.surql"))
            .expect("read canonical model registry schema");

    assert!(!loom.contains("DEFINE TABLE OVERWRITE model_runtime_registry"));
    assert!(!loom.contains(" ON TABLE model_runtime_registry "));
    assert!(canonical.contains("DEFINE TABLE OVERWRITE model_runtime_registry"));
    assert!(canonical.contains("DEFINE TABLE OVERWRITE model_runtime_active_selection"));
}

#[tokio::test]
async fn registry_schema_survives_loom_bootstrap_after_canonical_registry() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "model_registry",
        "schema_order",
    )
    .expect("config");
    let (storage, store) = open_store(config).await;
    bootstrap_loom_search_schema(&storage)
        .await
        .expect("bootstrap Loom-owned extensions after canonical registry");
    let scope = exact_scope("WS-MODEL-REGISTRY-SCHEMA");
    prepare_scope(&store, &scope).await;
    let rows = store
        .persist_role_bound_boot_set_and_read_back(
            &scope,
            &[
                registration(
                    0x10,
                    ModelRuntimeRole::Completion,
                    RuntimeBinding::Candle,
                    "schema-completion",
                ),
                registration(
                    0x11,
                    ModelRuntimeRole::Embedding,
                    RuntimeBinding::Candle,
                    "schema-embedding",
                ),
            ],
        )
        .await
        .expect("Loom bootstrap must not narrow registry authority");
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .any(|row| row.runtime_role == ModelRuntimeRole::Completion));
    assert!(rows
        .iter()
        .any(|row| row.runtime_role == ModelRuntimeRole::Embedding));
}

#[tokio::test]
async fn registry_restart_recovers_configured_set_defaults_and_stable_receipts() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "model_registry",
        "restart",
    )
    .expect("config");
    let scope = exact_scope("WS-MODEL-REGISTRY-RESTART");
    let (storage, store) = open_store(config.clone()).await;
    prepare_scope(&store, &scope).await;
    let completion = registration(
        0x20,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "restart-completion",
    );
    let embedding = registration(
        0x21,
        ModelRuntimeRole::Embedding,
        RuntimeBinding::Candle,
        "restart-embedding",
    );
    let configured = [selection(&completion), selection(&embedding)];
    let first = store
        .persist_role_bound_boot_set_and_read_back(&scope, &[completion.clone(), embedding.clone()])
        .await
        .expect("persist boot set");
    store
        .ensure_active_defaults(
            &scope,
            &[
                (
                    ModelRuntimeSelectionPurpose::ApplicationDefault,
                    completion.registration.sha256,
                ),
                (
                    ModelRuntimeSelectionPurpose::EmbeddingsDefault,
                    embedding.registration.sha256,
                ),
            ],
        )
        .await
        .expect("persist active defaults");
    let receipt_ids = first
        .iter()
        .map(|row| row.selection_updated_event_id.clone())
        .collect::<Vec<_>>();

    storage.shutdown().await.expect("close embedded store");
    drop(store);
    let (_reopened, reopened_store) = open_store(config).await;
    let recovered = reopened_store
        .recover_configured_runtime_binding_set(&scope, &configured)
        .await
        .expect("recover configured set after restart");
    assert_eq!(recovered.len(), 2);
    for (index, recovered) in recovered.into_iter().enumerate() {
        let recovered = recovered.expect("configured row survives restart");
        assert_eq!(recovered.schema_id, MODEL_RUNTIME_REGISTRY_SCHEMA_ID);
        assert_eq!(
            recovered.capabilities_schema_id,
            MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID
        );
        assert_eq!(recovered.selection_revision, 1);
        assert_eq!(recovered.selection_updated_event_id, receipt_ids[index]);
    }
    let defaults = reopened_store
        .list_active_selections(&scope)
        .await
        .expect("recover defaults");
    assert_eq!(defaults.len(), 2);
    assert!(defaults.iter().all(|row| row.selection_revision == 1));
}

#[tokio::test]
async fn exact_scope_and_runtime_role_never_widen() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "model_registry",
        "scope_role",
    )
    .expect("config");
    let (_storage, store) = open_store(config).await;
    let owner_scope = exact_scope("WS-MODEL-REGISTRY-OWNER");
    let foreign_scope = with_workspace(&owner_scope, "WS-MODEL-REGISTRY-FOREIGN");
    prepare_scope(&store, &owner_scope).await;
    prepare_scope(&store, &foreign_scope).await;
    let completion = registration(
        0x30,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "scope-completion",
    );
    let embedding = registration(
        0x31,
        ModelRuntimeRole::Embedding,
        RuntimeBinding::Candle,
        "scope-embedding",
    );
    store
        .persist_role_bound_boot_set_and_read_back(&owner_scope, &[completion.clone(), embedding])
        .await
        .expect("persist owner rows");
    assert!(store
        .load_by_artifact_sha256(&foreign_scope, &completion.registration.sha256)
        .await
        .expect("foreign read is a closed absence")
        .is_none());

    let foreign = store
        .persist_role_bound_boot_set_and_read_back(
            &foreign_scope,
            &[registration(
                0x30,
                ModelRuntimeRole::Completion,
                RuntimeBinding::Candle,
                "foreign-same-hash",
            )],
        )
        .await
        .expect("same artifact is isolated by exact scope");
    let owner = store
        .load_by_artifact_sha256(&owner_scope, &completion.registration.sha256)
        .await
        .expect("owner read")
        .expect("owner row");
    assert_ne!(owner.registry_row_id, foreign[0].registry_row_id);
    assert!(store
        .ensure_active_defaults(
            &owner_scope,
            &[(
                ModelRuntimeSelectionPurpose::EmbeddingsDefault,
                completion.registration.sha256
            )],
        )
        .await
        .is_err());
}

#[tokio::test]
async fn active_selection_cas_has_one_winner_stable_retry_and_no_failed_mutation() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "model_registry",
        "active_cas",
    )
    .expect("config");
    let (_storage, store) = open_store(config).await;
    let scope = exact_scope("WS-MODEL-REGISTRY-CAS");
    prepare_scope(&store, &scope).await;
    let initial = registration(
        0x40,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "cas-a",
    );
    let left = registration(
        0x41,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "cas-b",
    );
    let right = registration(
        0x42,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "cas-c",
    );
    store
        .persist_role_bound_boot_set_and_read_back(
            &scope,
            &[initial.clone(), left.clone(), right.clone()],
        )
        .await
        .expect("persist CAS candidates");
    store
        .ensure_active_defaults(
            &scope,
            &[(
                ModelRuntimeSelectionPurpose::ApplicationDefault,
                initial.registration.sha256,
            )],
        )
        .await
        .expect("initial default");

    let left_store = store.clone();
    let left_scope = scope.clone();
    let right_store = store.clone();
    let right_scope = scope.clone();
    let (left_result, right_result) = tokio::join!(
        left_store.select_active_model(
            &left_scope,
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            left.registration.sha256,
            1,
            KernelActor::Operator("cas-left".to_owned()),
            "verified left selection",
        ),
        right_store.select_active_model(
            &right_scope,
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            right.registration.sha256,
            1,
            KernelActor::Operator("cas-right".to_owned()),
            "verified right selection",
        )
    );
    assert_ne!(
        left_result.is_ok(),
        right_result.is_ok(),
        "exactly one CAS wins"
    );
    let (winner, actor, reason) = match (left_result, right_result) {
        (Ok(row), Err(_)) => (row, "cas-left", "verified left selection"),
        (Err(_), Ok(row)) => (row, "cas-right", "verified right selection"),
        _ => unreachable!("winner cardinality asserted"),
    };
    assert_eq!(winner.selection_revision, 2);
    let retry = store
        .select_active_model(
            &scope,
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            winner.artifact_sha256,
            1,
            KernelActor::Operator(actor.to_owned()),
            reason,
        )
        .await
        .expect("identical committed retry is stable");
    assert_eq!(retry.selection_revision, winner.selection_revision);
    assert_eq!(
        retry.selection_updated_event_id,
        winner.selection_updated_event_id
    );

    let failed = store
        .select_active_model(
            &scope,
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            initial.registration.sha256,
            1,
            KernelActor::Operator("stale-cas".to_owned()),
            "stale revision must fail",
        )
        .await;
    assert!(matches!(
        failed,
        Err(ModelRegistryPersistenceError::SelectionRevisionMismatch { .. })
    ));
    let after = store
        .list_active_selections(&scope)
        .await
        .expect("read after failed CAS")
        .into_iter()
        .find(|row| row.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application default");
    assert_eq!(after.selection_revision, winner.selection_revision);
    assert_eq!(
        after.selection_updated_event_id,
        winner.selection_updated_event_id
    );
}

#[tokio::test]
async fn verified_unload_rebind_increments_once_and_identical_retry_is_stable() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "model_registry",
        "rebind",
    )
    .expect("config");
    let (_storage, store) = open_store(config).await;
    let scope = exact_scope("WS-MODEL-REGISTRY-REBIND");
    prepare_scope(&store, &scope).await;
    let original = registration(
        0x50,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "rebind-model",
    );
    store
        .persist_role_bound_boot_set_and_read_back(&scope, &[original.clone()])
        .await
        .expect("persist original binding");
    let target = ModelRuntimeSelection {
        artifact_sha256: original.registration.sha256,
        runtime_binding: RuntimeBinding::LlamaCpp,
        runtime_role: ModelRuntimeRole::Completion,
        declared_capabilities: capabilities(ModelRuntimeRole::Completion, RuntimeBinding::LlamaCpp),
        provider: ProviderKind::Local,
    };
    let request = ExplicitModelRuntimeRebind::new(
        KernelActor::Operator("verified-unload-operator".to_owned()),
        "runtime owner verified old adapter unload",
        1,
    )
    .expect("valid rebind request");
    let rebound = store
        .rebind_selection_for_tests(&scope, &target, request.clone())
        .await
        .expect("rebind after verified unload");
    let retry = store
        .rebind_selection_for_tests(&scope, &target, request)
        .await
        .expect("identical rebind retry");
    assert_eq!(rebound.runtime_role, ModelRuntimeRole::Completion);
    assert_eq!(rebound.selection_revision, 2);
    assert_eq!(retry.selection_revision, 2);
    assert_eq!(
        retry.selection_updated_event_id,
        rebound.selection_updated_event_id
    );
    assert_eq!(
        retry.last_rebind_request_fingerprint,
        rebound.last_rebind_request_fingerprint
    );
}

#[tokio::test]
async fn boot_set_conflict_rolls_back_every_earlier_row_in_the_transaction() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "model_registry",
        "boot_atomic",
    )
    .expect("config");
    let (_storage, store) = open_store(config).await;
    let scope = exact_scope("WS-MODEL-REGISTRY-ATOMIC");
    prepare_scope(&store, &scope).await;
    let existing = registration(
        0x60,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "atomic-existing",
    );
    store
        .persist_role_bound_boot_set_and_read_back(&scope, &[existing.clone()])
        .await
        .expect("persist conflict predecessor");
    let new_row = registration(
        0x61,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "atomic-new",
    );
    let conflict = registration(
        0x60,
        ModelRuntimeRole::Completion,
        RuntimeBinding::LlamaCpp,
        "atomic-conflict",
    );
    assert!(store
        .persist_role_bound_boot_set_and_read_back(&scope, &[new_row.clone(), conflict])
        .await
        .is_err());
    assert!(store
        .load_by_artifact_sha256(&scope, &new_row.registration.sha256)
        .await
        .expect("read rolled-back artifact")
        .is_none());
    let preserved = store
        .load_by_artifact_sha256(&scope, &existing.registration.sha256)
        .await
        .expect("read original")
        .expect("original remains");
    assert_eq!(preserved.runtime_binding, RuntimeBinding::Candle);
    assert_eq!(preserved.selection_revision, 1);
}

#[tokio::test]
async fn receipt_and_projection_tampering_fail_closed() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "model_registry",
        "tamper",
    )
    .expect("config");
    let (_storage, store) = open_store(config).await;
    let receipt_scope = exact_scope("WS-MODEL-REGISTRY-RECEIPT-TAMPER");
    let projection_scope = exact_scope("WS-MODEL-REGISTRY-PROJECTION-TAMPER");
    prepare_scope(&store, &receipt_scope).await;
    prepare_scope(&store, &projection_scope).await;
    let receipt_row = registration(
        0x70,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "receipt-tamper",
    );
    let projection_row = registration(
        0x71,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "projection-tamper",
    );
    store
        .persist_role_bound_boot_set_and_read_back(&receipt_scope, &[receipt_row.clone()])
        .await
        .expect("persist receipt-tamper row");
    store
        .persist_role_bound_boot_set_and_read_back(&projection_scope, &[projection_row.clone()])
        .await
        .expect("persist projection-tamper row");
    store
        .corrupt_latest_receipt_for_tests(&receipt_scope, &receipt_row.registration.sha256)
        .await
        .expect("corrupt receipt semantics");
    assert!(store
        .load_by_artifact_sha256(&receipt_scope, &receipt_row.registration.sha256)
        .await
        .is_err());
    store
        .advance_projection_without_receipt_for_tests(
            &projection_scope,
            &projection_row.registration.sha256,
        )
        .await
        .expect("advance projection without receipt");
    assert!(store
        .load_by_artifact_sha256(&projection_scope, &projection_row.registration.sha256)
        .await
        .is_err());
}

#[tokio::test]
async fn orphan_receipt_stale_and_revoked_authority_are_rejected() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "model_registry",
        "negative_authority",
    )
    .expect("config");
    let (_storage, store) = open_store(config).await;
    let orphan_scope = exact_scope("WS-MODEL-REGISTRY-ORPHAN");
    let lifecycle_scope = exact_scope("WS-MODEL-REGISTRY-LIFECYCLE");
    prepare_scope(&store, &orphan_scope).await;
    prepare_scope(&store, &lifecycle_scope).await;
    let orphan = registration(
        0x80,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "orphan-receipt",
    );
    let orphan_event = store
        .inject_orphan_initial_receipt_for_tests(&orphan_scope, &orphan)
        .await
        .expect("inject receipt without mutation");
    assert!(!orphan_event.is_empty());
    assert!(store
        .persist_role_bound_boot_set_and_read_back(&orphan_scope, &[orphan])
        .await
        .is_err());
    assert!(store
        .list_recoverable(&orphan_scope)
        .await
        .expect("orphan scope has no mutation")
        .is_empty());

    let lifecycle = registration(
        0x81,
        ModelRuntimeRole::Completion,
        RuntimeBinding::Candle,
        "lifecycle",
    );
    let configured = selection(&lifecycle);
    store
        .persist_role_bound_boot_set_and_read_back(&lifecycle_scope, &[lifecycle.clone()])
        .await
        .expect("persist lifecycle row");
    store
        .set_registration_lifecycle_for_tests(
            &lifecycle_scope,
            &lifecycle.registration.sha256,
            ModelRegistryLifecycleState::Stale,
        )
        .await
        .expect("mark stale");
    assert!(matches!(
        store
            .recover_configured_selection(&lifecycle_scope, &configured)
            .await,
        Err(ModelRegistryPersistenceError::SelectionInactive { .. })
    ));
    store
        .set_registration_lifecycle_for_tests(
            &lifecycle_scope,
            &lifecycle.registration.sha256,
            ModelRegistryLifecycleState::Revoked,
        )
        .await
        .expect("mark revoked");
    assert!(matches!(
        store
            .recover_configured_runtime_binding_set(&lifecycle_scope, &[configured])
            .await,
        Err(ModelRegistryPersistenceError::SelectionInactive { .. })
    ));
    assert!(store
        .persist_role_bound_boot_set_and_read_back(&lifecycle_scope, &[lifecycle])
        .await
        .is_err());
}

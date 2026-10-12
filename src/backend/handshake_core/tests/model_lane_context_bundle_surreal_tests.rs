//! MT-005 ContextBundle handoff authority over embedded SurrealDB.

mod surreal_test_store_support;
mod account_session_support;
mod model_lane_cloud_support;
#[path = "model_lane_context_bundle_pg_tests.rs"]
mod context_bundle_assertions;

use handshake_core::storage::surreal::{bootstrap_production_schemas, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    model_lane_context_bundle_id_for_handoff, ModelLaneAuthority, ModelLaneCrdtHandoffMetadata,
    ModelLaneHandoffSelectionState, ModelLaneHandoffSourceKind, ModelLaneLoomHandoffRef,
    ModelLaneMemoryPackHandoffRef, ModelLaneStore, NewModelLaneContextBundleArtifactBinding,
    NewModelLaneContextBundleHandoff, ModelLaneMessageKind, ModelLaneError,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
    WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture_authenticated, AuthenticatedModelLaneTestOperation, SurrealAdmissibleCrdtPosture,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use handshake_core::storage::surreal::resource_authority::{AuthorizationRequest, ResourceAction, ResourceKind};
use handshake_core::swarm_orchestration::resource_scope::{ExactResourceScopeAttribution, ResourceAccessLifecycleRegistry, ResourceAccessContext};
use surreal_test_store_support::IsolatedSurrealTestStore;

struct Harness {
    isolated: IsolatedSurrealTestStore,
    storage: SurrealStorage,
    scope: ResourceScope,
    store: ModelLaneStore,
    reader_store: ModelLaneStore,
    account: account_session_support::AccountFixture,
    state: handshake_core::AppState,
    workspace_create: AuthenticatedModelLaneTestOperation,
    document_update: AuthenticatedModelLaneTestOperation,
    workspace_read: AuthenticatedModelLaneTestOperation,
    document_id: String,
    model_id: String,
    catalog: Arc<handshake_core::model_runtime::ModelCatalog>,
}

impl Harness {
    async fn create(label: &str) -> Self {
        let mut isolated = IsolatedSurrealTestStore::create()
            .await
            .expect("allocate MT-005 embedded scope");
        let storage = isolated.storage().clone();
        bootstrap_production_schemas(&storage)
            .await
            .expect("bootstrap canonical schema");
        let account = account_session_support::AccountFixture::install(&storage).await;
        let recorder = Arc::new(handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7)
            .expect("create bounded ContextBundle recorder"));
        let state = handshake_core::AppState {
            storage: Arc::new(handshake_core::storage::surreal::SurrealDatabase::new(storage.clone())),
            surreal: storage.clone(), flight_recorder: recorder.clone(), diagnostics: recorder,
            llm_client: Arc::new(handshake_core::llm::InMemoryLlmClient::new("context-bundle".into())),
            capability_registry: Arc::new(handshake_core::capabilities::CapabilityRegistry::new()),
            session_registry: Arc::new(handshake_core::workflows::SessionRegistry::new_from_env()),
        };
        let workspace_id = account.create_workspace(&state).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind owned document fixture");
        let address = listener.local_addr().expect("document fixture address");
        let app = handshake_core::api::knowledge_documents::routes(state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.expect("owned document fixture server"); });
        let response = account.apply(reqwest::Client::new().post(format!("http://{address}/knowledge/documents")))
            .header("x-hsk-actor-id", &account.actor_id).header("x-hsk-actor-kind", "operator")
            .header("x-hsk-kernel-task-run-id", format!("context-fixture-{label}"))
            .header("x-hsk-session-run-id", &account.session_id)
            .json(&json!({"workspace_id": workspace_id, "title": format!("ContextBundle {label}"),
                "content_json": {"type": "doc", "content": []}})).send().await.expect("authenticated document create");
        let status = response.status();
        let body: serde_json::Value = response.json().await.expect("canonical document response");
        server.abort(); let _ = server.await;
        assert_eq!(status, reqwest::StatusCode::CREATED, "actual document producer failed: {body}");
        let document_id = body["document"]["rich_document_id"].as_str().expect("canonical document ID").to_owned();
        let scope = ResourceScope::new(
            OwnerAccountId::from_uuid(uuid::Uuid::parse_str(&account.account_id).expect("actual account UUID")),
            ActorPrincipalId::from_uuid(uuid::Uuid::parse_str(&account.principal_id).expect("actual principal UUID")))
            .with_session(AuthenticatedSessionRef::from_uuid(uuid::Uuid::parse_str(&account.session_id).expect("actual session UUID")))
            .with_access_space(AccessSpaceRef::from_uuid(uuid::Uuid::parse_str(&account.access_space_id).expect("actual Space UUID")))
            .with_workspace(WorkspaceScopeRef::new(workspace_id.clone()).expect("canonical workspace"));
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope).expect("actual complete attribution");
        let lifecycle = ResourceAccessLifecycleRegistry::default();
        lifecycle.register_active(exact.clone()).expect("register actual authenticated lifecycle");
        let model = handshake_core::model_runtime::ModelId::new_v7();
        let model_id = model.to_string();
        let mut registry = handshake_core::model_runtime::ModelRegistry::default();
        registry.register(handshake_core::model_runtime::ModelRegistration {
            model_id: model.clone(), artifact_path: "models/context-bundle-fixture.gguf".into(), sha256: [0x5a; 32],
            runtime_binding: handshake_core::model_runtime::RuntimeBinding::LlamaCpp,
            declared_capabilities: handshake_core::model_runtime::ModelCapabilities::default(),
            base_model_tag: handshake_core::model_runtime::BaseModelTag::new("context-bundle-fixture"),
            registered_at_utc: chrono::Utc::now(),
            registered_by: handshake_core::model_runtime::OperatorId::new("context-bundle-fixture"),
            provider: handshake_core::model_runtime::ProviderKind::Local,
        }).expect("register component catalog identity");
        registry.mark_loaded(model).expect("mark component catalog identity loaded");
        let catalog = handshake_core::model_runtime::ModelCatalog::from_registry(Arc::new(registry));
        let reader_store = ModelLaneStore::new(storage.clone(),
            ResourceAccessContext::for_exact_reader_with_lifecycle(exact.clone(), lifecycle.clone()))
            .with_model_catalog(catalog.clone());
        let store = ModelLaneStore::new_scoped_with_lifecycle(storage.clone(), scope.clone(), lifecycle)
            .with_model_catalog(catalog.clone());
        let request = |kind, external_resource_id: String, action, capability: &str| AuthorizationRequest {
            session_token: account.session_token.clone(), channel_binding_hash: Some({
                use sha2::{Digest, Sha256}; hex::encode(Sha256::digest(account.channel_binding_token.as_bytes()))
            }), capability_id: capability.into(), resource_kind: kind, external_resource_id, action,
        };
        let workspace_create = AuthenticatedModelLaneTestOperation::authorize(storage.clone(),
            request(ResourceKind::Workspace, workspace_id.clone(), ResourceAction::Create, "fs.write"), exact.clone())
            .await.expect("original workspace producer authorization");
        let document_update = AuthenticatedModelLaneTestOperation::authorize(storage.clone(),
            request(ResourceKind::RichDocument, document_id.clone(), ResourceAction::Update, "fs.write"), exact.clone())
            .await.expect("original document editing authorization");
        let workspace_read = AuthenticatedModelLaneTestOperation::authorize(storage.clone(),
            request(ResourceKind::Workspace, workspace_id, ResourceAction::Read, "fs.read"), exact)
            .await.expect("original consumer authorization");
        bootstrap_production_schemas(&storage).await.expect("administratively prepare canonical metadata for the persisted session/workspace before business operations");
        Self { isolated, storage, scope, store, reader_store, account, state, workspace_create, document_update,
            workspace_read, document_id, model_id, catalog }
    }

    async fn posture(&self, label: &str) -> SurrealAdmissibleCrdtPosture {
        let mut posture = build_surreal_admissible_crdt_posture_authenticated(
            &self.store, self.scope.workspace.as_ref().expect("exact workspace").as_str(), label,
            &self.model_id, &self.document_id, &self.workspace_create, &self.document_update,
        ).await.expect("build production CRDT posture");
        use sha2::{Digest, Sha256};
        let payload = json!({"schema_id": "hsk.model_lane_context_payload@1",
            "message_id": posture.message.message_id, "summary": posture.message.summary});
        posture.message.payload_sha256 = hex::encode(Sha256::digest(
            handshake_storage_support::canonical_json::canonical_json_bytes(&payload)));
        posture
    }

    async fn reopen(self) -> Self {
        let Self { mut isolated, storage, scope, store, reader_store, account, state, workspace_create,
            document_update, workspace_read, document_id, model_id, catalog } = self;
        drop(store); drop(reader_store); drop(state); drop(workspace_create); drop(document_update); drop(workspace_read); drop(storage);
isolated.reopen().await.expect("reopen original embedded owner");
        let storage = isolated.storage().clone();
        bootstrap_production_schemas(&storage).await.expect("reconcile canonical administrative schemas before authenticated reopen operations");
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope).expect("original complete scope");
        let lifecycle = ResourceAccessLifecycleRegistry::default();
        lifecycle.register_active(exact.clone()).expect("reactivate original authenticated tuple");
        let reader_store = ModelLaneStore::new(storage.clone(),
            ResourceAccessContext::for_exact_reader_with_lifecycle(exact.clone(), lifecycle.clone()))
            .with_model_catalog(catalog.clone());
        let store = ModelLaneStore::new_scoped_with_lifecycle(storage.clone(), scope.clone(), lifecycle)
            .with_model_catalog(catalog.clone());
        let recorder = Arc::new(handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7)
            .expect("reopened bounded recorder"));
        let state = handshake_core::AppState {
            storage: Arc::new(handshake_core::storage::surreal::SurrealDatabase::new(storage.clone())),
            surreal: storage.clone(), flight_recorder: recorder.clone(), diagnostics: recorder,
            llm_client: Arc::new(handshake_core::llm::InMemoryLlmClient::new("context-bundle".into())),
            capability_registry: Arc::new(handshake_core::capabilities::CapabilityRegistry::new()),
            session_registry: Arc::new(handshake_core::workflows::SessionRegistry::new_from_env()),
        };
        let request = |kind, external_resource_id: String, action, capability: &str| AuthorizationRequest {
            session_token: account.session_token.clone(), channel_binding_hash: Some({
                use sha2::{Digest, Sha256}; hex::encode(Sha256::digest(account.channel_binding_token.as_bytes()))
            }), capability_id: capability.into(), resource_kind: kind, external_resource_id, action,
        };
        let workspace_id = scope.workspace.as_ref().expect("original workspace").as_str().to_owned();
        let workspace_create = AuthenticatedModelLaneTestOperation::authorize(storage.clone(),
            request(ResourceKind::Workspace, workspace_id.clone(), ResourceAction::Create, "fs.write"), exact.clone())
            .await.expect("reread original persisted producer grant");
        let document_update = AuthenticatedModelLaneTestOperation::authorize(storage.clone(),
            request(ResourceKind::RichDocument, document_id.clone(), ResourceAction::Update, "fs.write"), exact.clone())
            .await.expect("reread original persisted document grant");
        let workspace_read = AuthenticatedModelLaneTestOperation::authorize(storage.clone(),
            request(ResourceKind::Workspace, workspace_id, ResourceAction::Read, "fs.read"), exact)
            .await.expect("reread original persisted consumer grant");
        Self { isolated, storage, scope, store, reader_store, account, state, workspace_create, document_update,
            workspace_read, document_id, model_id, catalog }
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.reader_store);
        drop(self.state);
        drop(self.workspace_create); drop(self.document_update); drop(self.workspace_read);
        drop(self.storage);
        self.isolated.cleanup().await.expect("clean MT-005 scope");
        drop(self.account);
    }
}

#[tokio::test]
async fn model_lane_context_bundle_persists_selection_state_and_replays() {
    let mut harness = Harness::create("handoff-replay").await;
    let posture = harness.posture("handoff-replay").await;
    let downstream = record_downstream_lane(&harness, &posture).await;
    let cloud_lane = record_cloud_source_lane(&harness, &posture).await;
    let mut expected = Vec::new();
    for (index, (state, kind, source_kind)) in [
        (ModelLaneHandoffSelectionState::Selected, ModelLaneMessageKind::Proposal, ModelLaneHandoffSourceKind::Proposal),
        (ModelLaneHandoffSelectionState::Rejected, ModelLaneMessageKind::Critique, ModelLaneHandoffSourceKind::Critique),
        (ModelLaneHandoffSelectionState::Unresolved, ModelLaneMessageKind::Status, ModelLaneHandoffSourceKind::Status),
        (ModelLaneHandoffSelectionState::Superseded, ModelLaneMessageKind::Recovery, ModelLaneHandoffSourceKind::Recovery),
    ].into_iter().enumerate() {
        let lane = if matches!(kind, ModelLaneMessageKind::Critique | ModelLaneMessageKind::Recovery) {
            &cloud_lane
        } else { &posture.lane_id };
        let source = record_advisory_context_source(&harness, &posture, lane, kind, index).await;
        let artifact = harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(
            artifact_binding(&posture, &source))).await.expect("persist actual source artifact");
        let mut input = handoff(harness.scope.workspace.as_ref().expect("canonical workspace").as_str(), &posture, &source, &downstream, &format!("selection-{index}"), state, None);
        input.source_kind = source_kind;
        input.authority_state = ModelLaneAuthority::Advisory;
        let stored = harness.workspace_create.run(harness.store.record_context_bundle_handoff(input.clone()))
            .await.expect("record typed selection handoff");
        context_bundle_assertions::selection_assert!(equal, harness.workspace_create.run(harness.store.record_context_bundle_handoff(input))
            .await.expect("identical handoff retry"), stored);
        assert_final_context_receipt(&harness, "context_artifact", &artifact.artifact_binding_id,
            &artifact.event_ledger_event_id, artifact.event_ledger_seq, &serde_json::to_value(&artifact).unwrap()).await;
        assert_final_context_receipt(&harness, "context_handoff", &stored.handoff_id,
            &stored.event_ledger_event_id, stored.event_ledger_seq, &serde_json::to_value(&stored).unwrap()).await;
        expected.push(stored);
    }
    let context_bundle_id = expected[0].context_bundle_id.clone();
    let replay = harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id))
        .await
        .expect("replay exact context handoffs");
    context_bundle_assertions::selection_assert!(equal, replay.len(), 4, "one ContextBundle must replay all handoff selection states together");
    context_bundle_assertions::selection_assert!(condition, replay.windows(2).all(|pair| pair[0].event_ledger_seq <= pair[1].event_ledger_seq),
        "ContextBundle replay must be ordered by EventLedger sequence");
    context_bundle_assertions::selection_assert!(equal, replay, expected, "all four selection states share one canonical bundle");
    let consumed = harness.workspace_read.run(harness.reader_store.consume_context_bundle_for_downstream(&posture.run_id, &context_bundle_id, &downstream))
        .await
        .expect("consume without raw prompt replay");
    context_bundle_assertions::selection_assert!(equal, consumed.records, replay);
    context_bundle_assertions::selection_assert!(equal, consumed.downstream_lane_id, downstream);
    context_bundle_assertions::selection_assert!(equal, consumed.allowed_context["selected"].as_array().unwrap().len(), 1);
    let kernel_context = consumed.to_kernel_context_bundle().unwrap();
    context_bundle_assertions::selection_assert!(equal, kernel_context.context_bundle_id, format!("CTX-{}", &consumed.context_hash[..16]));
    context_bundle_assertions::selection_assert!(equal, kernel_context.session_run_id, downstream);
    context_bundle_assertions::selection_assert!(equal, kernel_context.context_hash, consumed.context_hash);
    let coordinator = context_bundle_coordinator(harness.reader_store.clone());
    let adapter = handshake_core::kernel::DummyEchoModelAdapter::new("dummy-echo-mt005");
    let output = harness.workspace_read.run(async { Ok::<_, ModelLaneError>(coordinator.invoke_downstream_context_bundle(
        &posture.run_id, &context_bundle_id, &downstream, &adapter,
        handshake_core::kernel::KernelActor::ModelAdapter(downstream.clone()),
    ).await) }).await.expect("authorize actual coordinator ContextBundle invocation")
        .expect("actual coordinator passes the canonical ContextBundle into DummyEcho");
    context_bundle_assertions::selection_assert!(equal, output.context_bundle_id, kernel_context.context_bundle_id);
    context_bundle_assertions::selection_assert!(condition, output.response_text.contains(&kernel_context.context_hash));
    let wrong = harness.workspace_read.run(async { Ok::<_, ModelLaneError>(coordinator.context_bundle_for_downstream_lane(
        &posture.run_id, &context_bundle_id, "lane-missing",
    ).await) }).await.expect("authorize wrong-downstream coordinator probe").unwrap_err();
    context_bundle_assertions::selection_assert!(condition, wrong.to_string().contains("downstream_lane_id"), "wrong downstream identity is explicit: {wrong}");
    let schemas = harness.workspace_read.run(harness.reader_store.schema_registry_rows()).await.unwrap();
    for schema_id in ["hsk.model_lane_context_bundle_artifact@1", "hsk.model_lane_context_bundle_handoff@1"] {
        context_bundle_assertions::selection_assert!(condition, schemas.iter().any(|row| row.schema_id == schema_id));
    }
    drop(coordinator);
    let owner_receipts = harness.workspace_read.run(harness.reader_store.test_scoped_authority_receipts(&posture.run_id, 128))
        .await
        .expect("owner authority watermark");
    context_bundle_assertions::selection_assert!(equal, 
        harness.workspace_read.run(harness.reader_store.test_scoped_authority_receipts(&posture.run_id, 128))
            .await
            .expect("foreign denials are non-mutating"),
        owner_receipts
    );

    harness = harness.reopen().await;
    context_bundle_assertions::selection_assert!(equal, harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id))
        .await.expect("handoff survives authenticated same-owner restart"), replay);
    {
        use handshake_core::storage::surreal::{RowFilter, ScalarValue, TestFieldMutation, TestMutationValue};
        let inspector = harness.storage.test_inspector();
        let table = inspector.table_selector("model_lane_authority").await.unwrap();
        let field = table.field("record_json").unwrap();
        let rows = inspector.project(&table, &[field.clone()], RowFilter::FieldEquals {
            field: table.field("aggregate_id").unwrap(), value: ScalarValue::String(replay[0].handoff_id.clone()),
        }).await.unwrap();
        context_bundle_assertions::selection_assert!(equal, rows.len(), 1);
        let key = rows[0].record_id.key_string().unwrap().to_owned();
        let before = rows[0].values["record_json"].clone();
        let mut altered: serde_json::Value = serde_json::from_str(before.as_str().unwrap()).unwrap();
        altered["context_bundle_hash"] = json!("forged-projection-hash");
        let mutator = harness.storage.test_mutator();
        mutator.corrupt_row_field(&table, key.clone(), &TestFieldMutation::new(field.clone(),
            TestMutationValue::string(serde_json::to_string(&altered).unwrap()))).await.unwrap();
        let denied = harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(
            &posture.run_id, &context_bundle_id)).await;
        mutator.corrupt_row_field(&table, key.clone(), &TestFieldMutation::new(field.clone(),
            TestMutationValue::json(before.clone()))).await.unwrap();
        let error = denied.expect_err("mutable context projection hash drift fails closed");
        context_bundle_assertions::selection_assert!(condition, error.to_string().contains("context_bundle_hash"), "exact immutable hash diagnostic: {error}");
        context_bundle_assertions::selection_assert!(equal, inspector.project(&table, &[field], RowFilter::IdEquals(key)).await.unwrap()[0]
            .values["record_json"], before);
    }
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_context_bundle_consume_preserves_exact_scope_and_denies_foreign_or_mixed_sources() {
    let harness = Harness::create("exact-scope").await;
    let posture = harness.posture("exact-scope").await;
    let source = harness.workspace_create.run(harness.store.record_message(posture.message.clone())).await.unwrap();
    let downstream = record_downstream_lane(&harness, &posture).await;
    let artifact = harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(artifact_binding(&posture, &source))).await.unwrap();
    let input = handoff(harness.scope.workspace.as_ref().unwrap().as_str(), &posture, &source, &downstream,
        "exact-owner", ModelLaneHandoffSelectionState::Selected, None);
    let canonical = harness.workspace_create.run(harness.store.record_context_bundle_handoff(input)).await.unwrap();
    let original = harness.workspace_read.run(harness.reader_store.consume_context_bundle_for_downstream(
        &posture.run_id, &canonical.context_bundle_id, &downstream)).await.unwrap();
    assert_final_context_receipt(&harness, "context_artifact", &artifact.artifact_binding_id,
        &artifact.event_ledger_event_id, artifact.event_ledger_seq, &serde_json::to_value(&artifact).unwrap()).await;
    assert_final_context_receipt(&harness, "context_handoff", &canonical.handoff_id,
        &canonical.event_ledger_event_id, canonical.event_ledger_seq, &serde_json::to_value(&canonical).unwrap()).await;
    assert_context_receipt_scope_tamper_denied(&harness, "context_artifact", &artifact.event_ledger_event_id,
        &posture.run_id, &canonical.context_bundle_id, &downstream).await;
    assert_context_receipt_scope_tamper_denied(&harness, "context_handoff", &canonical.event_ledger_event_id,
        &posture.run_id, &canonical.context_bundle_id, &downstream).await;
    let foreign = account_session_support::OwnerSession::provision(&harness.storage, harness.account.binding_token()).await;
    let workspace = foreign.create_workspace(&harness.state).await;
    let foreign_scope = ResourceScope::new(OwnerAccountId::from_uuid(uuid::Uuid::parse_str(&foreign.account_id).unwrap()),
        ActorPrincipalId::from_uuid(uuid::Uuid::parse_str(&foreign.principal_id).unwrap()))
        .with_session(AuthenticatedSessionRef::from_uuid(uuid::Uuid::parse_str(&foreign.session_id).unwrap()))
        .with_access_space(AccessSpaceRef::from_uuid(uuid::Uuid::parse_str(&foreign.access_space_id).unwrap()))
        .with_workspace(WorkspaceScopeRef::new(workspace.clone()).unwrap());
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&foreign_scope).unwrap();
    let lifecycle = ResourceAccessLifecycleRegistry::default(); lifecycle.register_active(exact.clone()).unwrap();
    let reader = ModelLaneStore::new(harness.storage.clone(), ResourceAccessContext::for_exact_reader_with_lifecycle(exact.clone(), lifecycle.clone()));
    let writer = ModelLaneStore::new_scoped_with_lifecycle(harness.storage.clone(), foreign_scope, lifecycle);
    let request = |action, capability: &str| AuthorizationRequest {
        session_token: foreign.session_token.clone(), channel_binding_hash: Some({ use sha2::{Digest, Sha256};
            hex::encode(Sha256::digest(foreign.channel_binding_token.as_bytes())) }),
        capability_id: capability.into(), resource_kind: ResourceKind::Workspace, external_resource_id: workspace.clone(), action,
    };
    let read = AuthenticatedModelLaneTestOperation::authorize(harness.storage.clone(), request(ResourceAction::Read, "fs.read"), exact.clone()).await.unwrap();
    let write = AuthenticatedModelLaneTestOperation::authorize(harness.storage.clone(), request(ResourceAction::Create, "fs.write"), exact).await.unwrap();
    bootstrap_production_schemas(&harness.storage).await.unwrap();
    use handshake_core::storage::surreal::RowFilter;
    let inspector = harness.storage.test_inspector();
    let authorities = inspector.table_selector("model_lane_authority").await.unwrap();
    let receipts = inspector.table_selector("kernel_event_ledger").await.unwrap();
    let authority_count = inspector.row_count(&authorities, RowFilter::All).await.unwrap();
    let receipt_count = inspector.row_count(&receipts, RowFilter::All).await.unwrap();
    context_bundle_assertions::scope_assert!(condition, read.run(reader.replay_context_bundle_handoffs(&posture.run_id, &canonical.context_bundle_id)).await.unwrap().is_empty());
    let denied = read.run(reader.consume_context_bundle_for_downstream(&posture.run_id, &canonical.context_bundle_id, &downstream)).await.unwrap_err();
    let denial = denied.to_string();
    for secret in [&source.payload_ref, &canonical.handoff_id, &canonical.artifact_ref,
        &harness.account.principal_id, &harness.account.session_id] {
        context_bundle_assertions::scope_assert!(condition, !denial.contains(secret), "scope denial must not disclose foreign identity or content: {denial}");
    }
    let mut attempted = canonical.inner.clone(); attempted.handoff_id = "foreign-handoff".into(); attempted.idempotency_key = attempted.handoff_id.clone();
    context_bundle_assertions::scope_assert!(condition, write.run(writer.record_context_bundle_handoff(attempted.clone())).await.is_err());
    assert_no_context_derivative(&harness, &attempted.handoff_id).await;
    let mut foreign_artifact = artifact_binding(&posture, &source);
    foreign_artifact.artifact_binding_id.push_str("-foreign-write");
    foreign_artifact.idempotency_key = foreign_artifact.artifact_binding_id.clone();
    context_bundle_assertions::scope_assert!(condition, write.run(writer.record_context_bundle_artifact_binding(foreign_artifact.clone())).await.is_err(),
        "a separately authenticated foreign producer cannot derive an owner artifact");
    assert_no_context_derivative(&harness, &foreign_artifact.artifact_binding_id).await;
    use handshake_core::swarm_orchestration::resource_scope::ResourceScopeQuery;
    let incomplete_readers = [
        ("actor", ModelLaneStore::new(harness.storage.clone(), ResourceAccessContext::for_reader(
            ResourceScopeQuery::for_owner(harness.scope.owner_account_id)
                .within_workspace(harness.scope.workspace.clone().unwrap())))),
        ("session", ModelLaneStore::new_scoped(harness.storage.clone(), ResourceScope {
            authenticated_session: None, ..harness.scope.clone()
        })),
        ("access-space", ModelLaneStore::new_scoped(harness.storage.clone(), ResourceScope {
            access_space: None, ..harness.scope.clone()
        })),
        ("workspace", ModelLaneStore::new_scoped(harness.storage.clone(), ResourceScope {
            workspace: None, ..harness.scope.clone()
        })),
    ];
    for (dimension, incomplete) in incomplete_readers {
        let replay_error = harness.workspace_read.run(incomplete.replay_context_bundle_handoffs(
            &posture.run_id, &canonical.context_bundle_id)).await.unwrap_err();
        let consume_error = harness.workspace_read.run(incomplete.consume_context_bundle_for_downstream(
            &posture.run_id, &canonical.context_bundle_id, &downstream)).await.unwrap_err();
        for error in [replay_error, consume_error] {
            context_bundle_assertions::scope_assert!(condition, error.to_string().contains(
                "exact owner, Principal, authenticated session, AccessSpace, and workspace"),
                "missing {dimension} attribution is an explicit shape denial: {error}");
        }
    }
    for wrong_scope in one_field_mismatches(&harness.scope) {
        context_bundle_assertions::scope_assert!(condition, ResourceScope::derive_from_sources([&harness.scope, &wrong_scope], harness.scope.actor_principal_id).is_err(),
            "a mixed five-field source set must never derive widened ContextBundle authority");
    }
    for field in ["owner_account_id", "actor_principal_id", "authenticated_session_id", "access_space_id", "workspace_id"] {
        assert_mixed_source_denial(&harness, &posture, &source, &downstream, field).await;
    }
    context_bundle_assertions::scope_assert!(equal, inspector.row_count(&authorities, RowFilter::All).await.unwrap(), authority_count,
        "all denied foreign and mixed-source writes preserve the complete canonical authority set");
    context_bundle_assertions::scope_assert!(equal, inspector.row_count(&receipts, RowFilter::All).await.unwrap(), receipt_count,
        "all denied foreign and mixed-source writes preserve the complete canonical receipt set");
    context_bundle_assertions::scope_assert!(equal, harness.workspace_read.run(harness.reader_store.consume_context_bundle_for_downstream(
        &posture.run_id, &canonical.context_bundle_id, &downstream)).await.unwrap(), original);
    drop(inspector); drop(read); drop(write); drop(reader); drop(writer);
    harness.cleanup().await;
}

async fn assert_missing_crdt_binding_denied(harness: &Harness,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) {
    use handshake_core::storage::surreal::{RowFilter, ScalarValue, TestFieldMutation, TestMutationValue};
    use sha2::{Digest, Sha256};
    let inspector = harness.storage.test_inspector();
    let authority = inspector.table_selector("model_lane_authority").await.unwrap();
    let record_field = authority.field("record_json").unwrap();
    let records = inspector.project(&authority, &[record_field.clone()], RowFilter::FieldEquals {
        field: authority.field("aggregate_id").unwrap(), value: ScalarValue::String(source.message_id.clone()),
    }).await.unwrap();
    assert_eq!(records.len(), 1);
    let authority_key = records[0].record_id.key_string().unwrap().to_owned();
    let original_record = records[0].values["record_json"].clone();
    let mut altered_record: serde_json::Value = serde_json::from_str(original_record.as_str().unwrap()).unwrap();
    assert!(altered_record.as_object_mut().unwrap().remove("crdt_authority_binding").is_some());
    let ledger = inspector.table_selector("kernel_event_ledger").await.unwrap();
    let payload_field = ledger.field("payload").unwrap();
    let hash_field = ledger.field("payload_hash").unwrap();
    let receipts = inspector.project(&ledger, &[payload_field.clone(), hash_field.clone()],
        RowFilter::IdEquals(source.event_ledger_event_id.clone())).await.unwrap();
    assert_eq!(receipts.len(), 1);
    let original_receipt = receipts[0].values.clone();
    let mut altered_payload = original_receipt["payload"].clone();
    let mut inner: serde_json::Value = serde_json::from_str(altered_payload["event_payload_json"].as_str().unwrap()).unwrap();
    assert!(inner["record"].as_object_mut().unwrap().remove("crdt_authority_binding").is_some());
    altered_payload["event_payload_json"] = json!(serde_json::to_string(&inner).unwrap());
    let correct_hash = hex::encode(Sha256::digest(altered_payload["event_payload_json"].as_str().unwrap().as_bytes()));
    let mutator = harness.storage.test_mutator();
    mutator.corrupt_row_field(&authority, authority_key.clone(), &TestFieldMutation::new(record_field.clone(),
        TestMutationValue::string(serde_json::to_string(&altered_record).unwrap()))).await.unwrap();
    mutator.update_row(&ledger, source.event_ledger_event_id.clone(), &[
        TestFieldMutation::new(payload_field.clone(), TestMutationValue::json(altered_payload)),
        TestFieldMutation::new(hash_field.clone(), TestMutationValue::string(correct_hash)),
    ]).await.unwrap();
    let denied = harness.workspace_read.run(harness.reader_store.replay_run(&source.run_id)).await;
    mutator.corrupt_row_field(&authority, authority_key.clone(), &TestFieldMutation::new(record_field.clone(),
        TestMutationValue::json(original_record.clone()))).await.unwrap();
    mutator.update_row(&ledger, source.event_ledger_event_id.clone(), &[
        TestFieldMutation::new(payload_field.clone(), TestMutationValue::json(original_receipt["payload"].clone())),
        TestFieldMutation::new(hash_field.clone(), TestMutationValue::json(original_receipt["payload_hash"].clone())),
    ]).await.unwrap();
    let error = denied.expect_err("CRDT source without projection AND receipt lease binding must deny");
    assert!(error.to_string().contains("CRDT metadata without durable authority binding"),
        "the missing-binding guard is reached after removing both surfaces with the correct payload byte hash: {error}");
    assert_eq!(inspector.project(&authority, &[record_field], RowFilter::IdEquals(authority_key)).await.unwrap()[0]
        .values["record_json"], original_record);
    assert_eq!(inspector.project(&ledger, &[payload_field, hash_field],
        RowFilter::IdEquals(source.event_ledger_event_id.clone())).await.unwrap()[0].values, original_receipt);
}

async fn assert_context_receipt_scope_tamper_denied(harness: &Harness, kind: &str, event_id: &str,
    run_id: &str, bundle_id: &str, downstream: &str,
) {
    use handshake_core::storage::surreal::{RowFilter, TestFieldMutation, TestMutationValue};
    use sha2::{Digest, Sha256};
    let inspector = harness.storage.test_inspector();
    let table = inspector.table_selector("kernel_event_ledger").await.unwrap();
    let payload_field = table.field("payload").unwrap();
    let hash_field = table.field("payload_hash").unwrap();
    let before = inspector.project(&table, &[payload_field.clone(), hash_field.clone()],
        RowFilter::IdEquals(event_id.to_owned())).await.unwrap();
    assert_eq!(before.len(), 1);
    let original = &before[0].values;
    let mut altered = original["payload"].clone();
    let mut inner: serde_json::Value = serde_json::from_str(altered["event_payload_json"].as_str().unwrap()).unwrap();
    inner["actor_principal_id"] = json!(uuid::Uuid::now_v7().to_string());
    altered["event_payload_json"] = json!(serde_json::to_string(&inner).unwrap());
    let hash = hex::encode(Sha256::digest(altered["event_payload_json"].as_str().unwrap().as_bytes()));
    let mutator = harness.storage.test_mutator();
    mutator.update_row(&table, event_id.to_owned(), &[
        TestFieldMutation::new(payload_field.clone(), TestMutationValue::json(altered)),
        TestFieldMutation::new(hash_field.clone(), TestMutationValue::string(hash)),
    ]).await.expect("isolated typed receipt scope counterfactual with a correct outer payload hash");
    let denied = match kind {
        "context_artifact" => harness.workspace_read.run(harness.reader_store.consume_context_bundle_for_downstream(run_id, bundle_id, downstream)).await.map(|_| ()),
        "context_handoff" => harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(run_id, bundle_id)).await.map(|_| ()),
        other => panic!("unexpected context receipt counterfactual {other}"),
    };
    mutator.update_row(&table, event_id.to_owned(), &[
        TestFieldMutation::new(payload_field.clone(), TestMutationValue::json(original["payload"].clone())),
        TestFieldMutation::new(hash_field.clone(), TestMutationValue::json(original["payload_hash"].clone())),
    ]).await.unwrap();
    assert!(denied.is_err(), "{kind} canonical receipt scope counterfactual must deny despite its correct byte hash");
    let restored = inspector.project(&table, &[payload_field, hash_field], RowFilter::IdEquals(event_id.to_owned())).await.unwrap();
    assert_eq!(restored[0].values, *original, "independent reread verifies exact receipt restoration");
}

async fn assert_forged_run_scope_cannot_authorize_artifact(harness: &Harness,
    posture: &SurrealAdmissibleCrdtPosture,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) {
    use handshake_core::storage::surreal::RowFilter;
    let foreign = account_session_support::OwnerSession::provision(&harness.storage, harness.account.binding_token()).await;
    let workspace = foreign.create_workspace(&harness.state).await;
    let foreign_scope = ResourceScope::new(OwnerAccountId::from_uuid(uuid::Uuid::parse_str(&foreign.account_id).unwrap()),
        ActorPrincipalId::from_uuid(uuid::Uuid::parse_str(&foreign.principal_id).unwrap()))
        .with_session(AuthenticatedSessionRef::from_uuid(uuid::Uuid::parse_str(&foreign.session_id).unwrap()))
        .with_access_space(AccessSpaceRef::from_uuid(uuid::Uuid::parse_str(&foreign.access_space_id).unwrap()))
        .with_workspace(WorkspaceScopeRef::new(workspace.clone()).unwrap());
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&foreign_scope).unwrap();
    let lifecycle = ResourceAccessLifecycleRegistry::default(); lifecycle.register_active(exact.clone()).unwrap();
    let writer = ModelLaneStore::new_scoped_with_lifecycle(harness.storage.clone(), foreign_scope, lifecycle);
    let write = AuthenticatedModelLaneTestOperation::authorize(harness.storage.clone(), AuthorizationRequest {
        session_token: foreign.session_token.clone(), channel_binding_hash: Some(hex::encode(Sha256::digest(foreign.channel_binding_token.as_bytes()))),
        capability_id: "fs.write".into(), resource_kind: ResourceKind::Workspace,
        external_resource_id: workspace.clone(), action: ResourceAction::Create,
    }, exact).await.unwrap();
    bootstrap_production_schemas(&harness.storage).await.unwrap();
    let inspector = harness.storage.test_inspector();
    let authorities = inspector.table_selector("model_lane_authority").await.unwrap();
    {
        use handshake_core::storage::surreal::{ScalarValue, TestFieldMutation, TestMutationValue};
        let fields = ["owner_account_id", "actor_principal_id", "authenticated_session_id", "access_space_id", "workspace_id"];
        let selectors: Vec<_> = fields.iter().map(|field| authorities.field(field).unwrap()).collect();
        let original_run = inspector.project(&authorities, &selectors, RowFilter::FieldEquals {
            field: authorities.field("aggregate_id").unwrap(), value: ScalarValue::String(posture.run_id.clone()),
        }).await.unwrap();
        assert_eq!(original_run.len(), 1);
        let key = original_run[0].record_id.key_string().unwrap().to_owned();
        let candidate = [&foreign.account_id, &foreign.principal_id, &foreign.session_id,
            &foreign.access_space_id, &workspace];
        let mutations: Vec<_> = selectors.iter().zip(candidate).map(|(field, value)|
            TestFieldMutation::new(field.clone(), TestMutationValue::string(value.clone()))).collect();
        let mutator = harness.storage.test_mutator();
        mutator.update_row(&authorities, key.clone(), &mutations).await.unwrap();
        let forged = inspector.project(&authorities, &selectors, RowFilter::IdEquals(key.clone())).await.unwrap();
        assert_eq!(forged[0].values["actor_principal_id"], foreign.principal_id,
            "the mutable run projection really matches the independently authenticated foreign principal");
        let mut attempted = artifact_binding(posture, source);
        attempted.artifact_binding_id.push_str("-forged-run-matching-foreign-authority");
        attempted.idempotency_key = attempted.artifact_binding_id.clone();
        let rejected = write.run(writer.record_context_bundle_artifact_binding(attempted.clone())).await;
        let restores: Vec<_> = selectors.iter().zip(fields).map(|(field, name)|
            TestFieldMutation::new(field.clone(), TestMutationValue::json(original_run[0].values[name].clone()))).collect();
        mutator.update_row(&authorities, key.clone(), &restores).await.unwrap();
        assert!(rejected.is_err(), "a forged run projection matching foreign authority cannot manufacture an artifact from immutable owner receipts");
        assert_no_context_derivative(harness, &attempted.artifact_binding_id).await;
        assert_eq!(inspector.project(&authorities, &selectors, RowFilter::IdEquals(key)).await.unwrap()[0].values,
            original_run[0].values, "independent canonical reread verifies restoration of all five run fields");
    }
}

async fn assert_source_receipt_scope_counterfactual(harness: &Harness, posture: &SurrealAdmissibleCrdtPosture,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
    downstream: &str, kind: &str, identity: &str,
) {
    use handshake_core::storage::surreal::{RowFilter, ScalarValue, TestFieldMutation, TestMutationValue};
    assert!(matches!(kind, "run" | "lane" | "message"));
    let inspector = harness.storage.test_inspector();
    let authority = inspector.table_selector("model_lane_authority").await.unwrap();
    let links = inspector.project(&authority, &[authority.field("record_kind").unwrap(), authority.field("event_id").unwrap()],
        RowFilter::FieldEquals { field: authority.field("aggregate_id").unwrap(), value: ScalarValue::String(identity.into()) }).await.unwrap();
    assert_eq!(links.len(), 1); assert_eq!(links[0].values["record_kind"], kind);
    let event_id = links[0].values["event_id"].as_str().unwrap().to_owned();
    let ledger = inspector.table_selector("kernel_event_ledger").await.unwrap();
    let payload_field = ledger.field("payload").unwrap(); let hash_field = ledger.field("payload_hash").unwrap();
    let before = inspector.project(&ledger, &[payload_field.clone(), hash_field.clone()], RowFilter::IdEquals(event_id.clone())).await.unwrap();
    assert_eq!(before.len(), 1);
    let before = before[0].values.clone();
    let mut altered = before["payload"].clone();
    let mut payload: serde_json::Value = serde_json::from_str(altered["event_payload_json"].as_str().unwrap()).unwrap();
    let foreign_principal = uuid::Uuid::now_v7().to_string();
    assert_ne!(payload["actor_principal_id"], foreign_principal);
    payload["actor_principal_id"] = json!(foreign_principal);
    altered["event_payload_json"] = json!(serde_json::to_string(&payload).unwrap());
    let digest = hex::encode(Sha256::digest(altered["event_payload_json"].as_str().unwrap().as_bytes()));
    let mutator = harness.storage.test_mutator();
    mutator.update_row(&ledger, event_id.clone(), &[
        TestFieldMutation::new(payload_field.clone(), TestMutationValue::json(altered)),
        TestFieldMutation::new(hash_field.clone(), TestMutationValue::string(digest)),
    ]).await.unwrap();
    let input = handoff(harness.scope.workspace.as_ref().unwrap().as_str(), posture, source, downstream,
        &format!("source-{kind}-receipt-scope-drift"), ModelLaneHandoffSelectionState::Selected, None);
    let rejected = input.handoff_id.clone();
    let denied = harness.workspace_create.run(harness.store.record_context_bundle_handoff(input)).await;
    mutator.update_row(&ledger, event_id.clone(), &[
        TestFieldMutation::new(payload_field.clone(), TestMutationValue::json(before["payload"].clone())),
        TestFieldMutation::new(hash_field.clone(), TestMutationValue::json(before["payload_hash"].clone())),
    ]).await.unwrap();
    assert!(denied.is_err(), "{kind} immutable receipt scope drift must deny non-CRDT derivation even with a correct payload SHA");
    assert_no_context_derivative(harness, &rejected).await;
    assert_eq!(inspector.project(&ledger, &[payload_field, hash_field], RowFilter::IdEquals(event_id)).await.unwrap()[0].values,
        before, "the isolated canonical receipt counterfactual restores exact bytes and hash");
}

async fn assert_mixed_source_denial(harness: &Harness, posture: &SurrealAdmissibleCrdtPosture,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord, downstream: &str, field_name: &str,
) {
    assert_source_scope_counterfactual(harness, posture, source, downstream, "message", &source.message_id, field_name).await;
}

async fn assert_source_scope_counterfactual(harness: &Harness, posture: &SurrealAdmissibleCrdtPosture,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord, downstream: &str,
    source_kind: &str, aggregate_id: &str, field_name: &str,
) {
    assert!(matches!(source_kind, "run" | "lane" | "message"));
    assert!(matches!(field_name, "owner_account_id" | "actor_principal_id" | "authenticated_session_id" | "access_space_id" | "workspace_id"));
    use handshake_core::storage::surreal::{RowFilter, ScalarValue, TestFieldMutation, TestMutationValue};
    let inspector = harness.storage.test_inspector(); let table = inspector.table_selector("model_lane_authority").await.unwrap();
    let field = table.field(field_name).unwrap();
    let rows = inspector.project(&table, &[field.clone()], RowFilter::FieldEquals {
        field: table.field("aggregate_id").unwrap(), value: ScalarValue::String(aggregate_id.to_owned()),
    }).await.unwrap(); assert_eq!(rows.len(), 1);
    let key = rows[0].record_id.key_string().unwrap().to_owned();
    let before = rows[0].values[field_name].clone();
    let mutator = harness.storage.test_mutator();
    mutator.corrupt_row_field(&table, key.clone(), &TestFieldMutation::new(field.clone(), TestMutationValue::string(
        if field_name == "workspace_id" { "foreign-canonical-workspace".into() } else { uuid::Uuid::now_v7().to_string() }))).await.unwrap();
    let attempted = handoff(harness.scope.workspace.as_ref().unwrap().as_str(), posture, source, downstream,
        &format!("mixed-{source_kind}-{aggregate_id}-{field_name}"), ModelLaneHandoffSelectionState::Selected, None);
    let identity = attempted.handoff_id.clone();
    let denied = harness.workspace_create.run(harness.store.record_context_bundle_handoff(attempted)).await;
    mutator.corrupt_row_field(&table, key.clone(), &TestFieldMutation::new(field.clone(), TestMutationValue::json(before.clone()))).await.unwrap();
    assert!(denied.is_err(), "mixed source {field_name} must not authorize derivatives");
    assert_no_context_derivative(harness, &identity).await;
    let restored = inspector.project(&table, &[field], RowFilter::IdEquals(key)).await.unwrap();
    assert_eq!(restored[0].values[field_name], before, "isolated source counterfactual restores canonical attribution");
}

#[tokio::test]
async fn model_lane_context_bundle_missing_artifact_ref_fails_closed() {
    let harness = Harness::create("handoff-denial").await;
    let posture = harness.posture("handoff-denial").await;
    let source = harness.workspace_create.run(harness.store.record_message(posture.message.clone()))
        .await
        .expect("record source message");
    let downstream = record_downstream_lane(&harness, &posture).await;
    let workspace = harness.scope.workspace.as_ref().unwrap().as_str();
    let input = handoff(workspace, &posture, &source, &downstream, "missing-artifact", ModelLaneHandoffSelectionState::Selected, None);
    assert_handoff_denial(&harness, input, "ArtifactStore/EventLedger").await;
    let binding = artifact_binding(&posture, &source);
    let mut wrong = binding.clone();
    wrong.artifact_binding_id = "artifact-wrong-json-sha".into(); wrong.idempotency_key = wrong.artifact_binding_id.clone();
    wrong.artifact_sha256 = "e".repeat(64); wrong.content_hash = wrong.artifact_sha256.clone();
    let denied = harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(wrong.clone())).await.unwrap_err();
    context_bundle_assertions::denial_assert!(condition, denied.to_string().contains("payload_json sha256"), "{denied}");
    assert_no_context_derivative(&harness, &wrong.artifact_binding_id).await;
    wrong = binding.clone(); wrong.artifact_binding_id = "artifact-wrong-payload-ref".into(); wrong.idempotency_key = wrong.artifact_binding_id.clone();
    wrong.artifact_payload_ref = "artifact-store://mt005/not-the-payload".into();
    let denied = harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(wrong.clone())).await.unwrap_err();
    context_bundle_assertions::denial_assert!(condition, denied.to_string().contains("artifact_payload_ref"), "{denied}");
    assert_no_context_derivative(&harness, &wrong.artifact_binding_id).await;
    harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(binding)).await.unwrap();
    let cloud = record_cloud_source_lane(&harness, &posture).await;
    let base = handoff(workspace, &posture, &source, &downstream, "negative", ModelLaneHandoffSelectionState::Selected, None);
    let mut cases: Vec<(NewModelLaneContextBundleHandoff, &str)> = Vec::new();
    let mut wrong = base.clone(); wrong.artifact_ref = "artifact://mt005/not-the-source".into(); cases.push((wrong, "artifact_ref"));
    let mut wrong = base.clone(); wrong.artifact_sha256 = "f".repeat(64); wrong.content_hash = wrong.artifact_sha256.clone(); cases.push((wrong, "artifact_sha256"));
    let mut wrong = base.clone(); wrong.source_message_id = "msg-missing-404".into(); cases.push((wrong, "not replayable"));
    let mut wrong = base.clone(); wrong.downstream_lane_id.clear(); cases.push((wrong, "downstream_lane_id"));
    let mut wrong = base.clone(); wrong.downstream_lane_id = cloud.clone(); cases.push((wrong, "cloud_safe"));
    let mut wrong = base.clone(); wrong.downstream_lane_id = cloud.clone(); wrong.memory_pack_refs[0].cloud_safe = true; cases.push((wrong, "local_only_context"));
    let mut wrong = base.clone(); wrong.memory_pack_refs[0].memory_pack_ref = "provider-session://openai/thread-hidden".into(); cases.push((wrong, "hidden provider/session memory"));
    let mut wrong = base.clone(); wrong.memory_pack_refs[0].memory_pack_ref = "  PROVIDER-SESSION://openai/thread-hidden".into(); cases.push((wrong, "hidden provider/session memory"));
    let mut wrong = base.clone(); wrong.memory_pack_refs[0].projection_ref = Some("provider-session://openai/projection-hidden".into()); cases.push((wrong, "hidden provider/session memory"));
    let mut wrong = base.clone(); wrong.memory_pack_refs[0].review_status = "draft".into(); cases.push((wrong, "review_status"));
    let mut wrong = base.clone(); wrong.memory_pack_refs = (0..17).map(|index| { let mut entry = base.memory_pack_refs[0].clone();
        entry.memory_pack_ref = format!("memory-pack://mt005/{index}"); entry.projection_ref = Some(format!("memory-projection://mt005/{index}")); entry }).collect();
    cases.push((wrong, "bounded FEMS limit"));
    let mut wrong = base.clone(); wrong.loom_refs = (0..65).map(|index| { let mut entry = base.loom_refs[0].clone();
        entry.block_id = format!("loom-block-{index}"); entry }).collect(); cases.push((wrong, "loom_refs exceeds bounded limit"));
    for (index, (mut wrong, diagnostic)) in cases.into_iter().enumerate() {
        wrong.handoff_id = format!("handoff-denied-{index}"); wrong.idempotency_key = wrong.handoff_id.clone();
        wrong.context_bundle_id = model_lane_context_bundle_id_for_handoff(&wrong).unwrap();
        assert_handoff_denial(&harness, wrong, diagnostic).await;
    }
    let unbound = record_advisory_context_source(&harness, &posture, &cloud, ModelLaneMessageKind::Critique, 99).await;
    let input = handoff(workspace, &posture, &unbound, &downstream, "unbound", ModelLaneHandoffSelectionState::Selected, None);
    assert_handoff_denial(&harness, input, "ArtifactStore/EventLedger authority").await;
    for foreign in one_field_mismatches(&harness.scope) {
        let foreign_store = ModelLaneStore::new_scoped(harness.storage.clone(), foreign);
        context_bundle_assertions::denial_assert!(condition, foreign_store
            .consume_context_bundle_for_downstream(
                &posture.run_id,
                "CTX-does-not-exist",
                &downstream,
            )
            .await
            .is_err());
    }
    context_bundle_assertions::denial_assert!(equal, 
        harness.workspace_read.run(harness.reader_store.replay_run(&posture.run_id))
            .await
            .expect("owner authority survives all denials")
            .messages,
        vec![source, unbound]
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_context_bundle_crdt_state_vector_and_loom_refs_are_replayable() {
    let mut harness = Harness::create("handoff-crdt").await;
    let posture = harness.posture("handoff-crdt").await;
    let source = harness.workspace_create.run(harness.store.record_message(posture.message.clone()))
        .await
        .expect("record CRDT-bearing source message");
    let downstream = record_downstream_lane(&harness, &posture).await;
    let (context_bundle_id, canonical_handoffs, captured_context) = {
    harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(artifact_binding(&posture, &source)))
        .await
        .expect("record source artifact binding");
    let binding = source
        .crdt_authority_binding
        .as_ref()
        .expect("source has exact CRDT binding");
    context_bundle_assertions::crdt_assert!(equal, binding.run_id, posture.run_id);
    context_bundle_assertions::crdt_assert!(equal, binding.lane_id, posture.lane_id);
    context_bundle_assertions::crdt_assert!(equal, binding.crdt_session_id, posture.session_id);
    context_bundle_assertions::crdt_assert!(equal, binding.document_id, posture.document_id);
    context_bundle_assertions::crdt_assert!(equal, binding.crdt_document_id, posture.crdt_document_id);
    context_bundle_assertions::crdt_assert!(equal, binding.workspace_id, harness.scope.workspace.as_ref().unwrap().as_str());
    context_bundle_assertions::crdt_assert!(equal, binding.actor_id, posture.actor_id);
    context_bundle_assertions::crdt_assert!(equal, binding.actor_kind, posture.actor_kind);
    context_bundle_assertions::crdt_assert!(equal, binding.lease_id, posture.lease.lease_id);
    context_bundle_assertions::crdt_assert!(equal, binding.lease_correlation_id, posture.trace_id);
    context_bundle_assertions::crdt_assert!(equal, binding.lease_scope_kind, "document");
    context_bundle_assertions::crdt_assert!(equal, binding.lease_scope_id, posture.crdt_document_id);
    context_bundle_assertions::crdt_assert!(equal, binding.lease_claimed_at_utc, posture.lease.claimed_at_utc);
    context_bundle_assertions::crdt_assert!(equal, binding.lease_expires_at_utc, posture.lease.expires_at_utc);
    context_bundle_assertions::crdt_assert!(condition, binding.lease_admitted_at_utc >= posture.lease.claimed_at_utc);
    context_bundle_assertions::crdt_assert!(condition, binding.lease_admitted_at_utc < posture.lease.expires_at_utc);
    context_bundle_assertions::crdt_assert!(equal, binding.update_id, posture.update.update_id);
    context_bundle_assertions::crdt_assert!(equal, binding.update_seq, posture.update.update_seq);
    context_bundle_assertions::crdt_assert!(equal, binding.update_bytes_ref, posture.update.update_bytes_ref);
    context_bundle_assertions::crdt_assert!(equal, binding.base_snapshot_ref, posture.snapshot.snapshot_bytes_ref);
    context_bundle_assertions::crdt_assert!(equal, posture.snapshot.covered_update_seq, 1);
    context_bundle_assertions::crdt_assert!(equal, posture.update.update_seq, 2);
    {
        use handshake_core::storage::surreal::{RowFilter, ScalarValue};
        use yrs::{Doc, ReadTxn, StateVector, Transact, Update};
        use yrs::updates::{decoder::Decode, encoder::Encode};
        use base64::Engine;
        let inspector = harness.storage.test_inspector();
        let updates = inspector.table_selector("kernel_crdt_updates").await.unwrap();
        let snapshots = inspector.table_selector("kernel_crdt_snapshots").await.unwrap();
        let update_rows = inspector.project(&updates, &[
            updates.field("update_id").unwrap(), updates.field("update_seq").unwrap(),
            updates.field("update_bytes").unwrap(), updates.field("update_sha256").unwrap(),
            updates.field("state_vector_before").unwrap(), updates.field("state_vector_after").unwrap(),
        ], RowFilter::FieldEquals { field: updates.field("crdt_document_id").unwrap(),
            value: ScalarValue::String(posture.crdt_document_id.clone()) }).await.unwrap();
        let mut update_rows = update_rows;
        update_rows.sort_by_key(|row| row.values["update_seq"].as_i64().unwrap());
        context_bundle_assertions::crdt_assert!(equal, update_rows.len(), 2, "the canonical chain contains update1 and the approved update2");
        let snapshot_rows = inspector.project(&snapshots, &[
            snapshots.field("snapshot_bytes").unwrap(), snapshots.field("snapshot_sha256").unwrap(),
            snapshots.field("covered_update_seq").unwrap(), snapshots.field("state_vector").unwrap(),
        ], RowFilter::FieldEquals { field: snapshots.field("snapshot_id").unwrap(),
            value: ScalarValue::String(posture.snapshot.snapshot_id.clone()) }).await.unwrap();
        context_bundle_assertions::crdt_assert!(equal, snapshot_rows.len(), 1);
        let bytes = |value: &serde_json::Value| -> Vec<u8> {
            value.as_array().expect("canonical Surreal bytes project as an array").iter()
                .map(|byte| u8::try_from(byte.as_u64().expect("canonical byte is unsigned")).unwrap()).collect()
        };
        let full_chain = Doc::new();
        for (offset, row) in update_rows.iter().enumerate() {
            context_bundle_assertions::crdt_assert!(equal, row.values["update_seq"], json!((offset + 1) as i64));
            let update_bytes = bytes(&row.values["update_bytes"]);
            context_bundle_assertions::crdt_assert!(equal, row.values["update_sha256"], json!(hex::encode(Sha256::digest(&update_bytes))));
            full_chain.transact_mut().apply_update(Update::decode_v1(&update_bytes).unwrap()).unwrap();
        }
        context_bundle_assertions::crdt_assert!(equal, update_rows[1].values["update_id"], posture.update.update_id);
        context_bundle_assertions::crdt_assert!(equal, update_rows[1].values["state_vector_before"], update_rows[0].values["state_vector_after"]);
        context_bundle_assertions::crdt_assert!(equal, snapshot_rows[0].values["state_vector"], update_rows[0].values["state_vector_after"]);
        context_bundle_assertions::crdt_assert!(equal, snapshot_rows[0].values["covered_update_seq"], 1);
        let snapshot_bytes = bytes(&snapshot_rows[0].values["snapshot_bytes"]);
        context_bundle_assertions::crdt_assert!(equal, snapshot_rows[0].values["snapshot_sha256"], json!(hex::encode(Sha256::digest(&snapshot_bytes))));
        let snapshot_replay = Doc::new();
        snapshot_replay.transact_mut().apply_update(Update::decode_v1(&snapshot_bytes).unwrap()).unwrap();
        snapshot_replay.transact_mut().apply_update(Update::decode_v1(&bytes(&update_rows[1].values["update_bytes"])).unwrap()).unwrap();
        let projection = full_chain.transact().encode_state_as_update_v1(&StateVector::default());
        context_bundle_assertions::crdt_assert!(equal, snapshot_replay.transact().encode_state_as_update_v1(&StateVector::default()), projection);
        context_bundle_assertions::crdt_assert!(equal, binding.materialized_projection_hash, hex::encode(Sha256::digest(&projection)));
        context_bundle_assertions::crdt_assert!(equal, binding.yjs_state_vector_b64, base64::engine::general_purpose::STANDARD
            .encode(full_chain.transact().state_vector().encode_v1()),
            "persisted message authority equals the independently replayed canonical Yjs state vector");
    }
    use handshake_core::storage::surreal::RowFilter as ReceiptRowFilter;
    let inspector = harness.storage.test_inspector();
    let ledger = inspector.table_selector("kernel_event_ledger").await.unwrap();
    let receipts = inspector.project(&ledger, &[ledger.field("payload").unwrap()],
        ReceiptRowFilter::IdEquals(source.event_ledger_event_id.clone())).await.unwrap();
    context_bundle_assertions::crdt_assert!(equal, receipts.len(), 1);
    let immutable: serde_json::Value = serde_json::from_str(receipts[0].values["payload"]["event_payload_json"].as_str().unwrap()).unwrap();
    context_bundle_assertions::crdt_assert!(equal, immutable["record"]["crdt_authority_binding"], serde_json::to_value(binding).unwrap(),
        "immutable EventLedger and message projection bind the complete same CRDT authority");
    context_bundle_assertions::crdt_assert!(equal, posture.proposal.review_state, "approved");
    context_bundle_assertions::crdt_assert!(equal, posture.proposal.applied_update_id.as_deref(), Some(posture.update.update_id.as_str()));
    context_bundle_assertions::crdt_assert!(equal, posture.proposal.applied_update_sha256.as_deref(), Some(posture.proposal.diff_sha256.as_str()));
    context_bundle_assertions::crdt_assert!(different, posture.proposal.diff_sha256, posture.update.update_sha256,
        "approved canonical JSON and actual Yjs bytes retain distinct hash domains");
    let applied_event = posture.proposal.applied_event_id.as_ref().expect("actual immutable application receipt");
    let application_rows = inspector.project(&ledger, &[ledger.field("payload").unwrap(), ledger.field("event_sequence").unwrap()],
        ReceiptRowFilter::IdEquals(applied_event.clone())).await.unwrap();
    context_bundle_assertions::crdt_assert!(equal, application_rows.len(), 1);
    let application = &application_rows[0].values["payload"]["application_evidence"];
    context_bundle_assertions::crdt_assert!(equal, application["schema_id"], "hsk.model_lane_proposal_application@1");
    context_bundle_assertions::crdt_assert!(equal, application["operation"], "append_text");
    context_bundle_assertions::crdt_assert!(equal, application["path"], json!(["mt018-shared-document"]));
    context_bundle_assertions::crdt_assert!(equal, application["proposal_id"], posture.proposal.proposal_id);
    context_bundle_assertions::crdt_assert!(equal, application["recorded_event_id"], posture.proposal.recorded_event_id);
    context_bundle_assertions::crdt_assert!(equal, application["update_id"], posture.update.update_id);
    context_bundle_assertions::crdt_assert!(equal, application["update_event_id"], posture.update.event_ledger_event_id);
    context_bundle_assertions::crdt_assert!(equal, application["base_snapshot_id"], posture.snapshot.snapshot_id);
    context_bundle_assertions::crdt_assert!(equal, application["base_snapshot_event_id"], posture.snapshot.event_ledger_event_id);
    context_bundle_assertions::crdt_assert!(equal, application["approved_diff_sha256"], posture.proposal.diff_sha256);
    context_bundle_assertions::crdt_assert!(equal, application["yjs_update_sha256"], posture.update.update_sha256);
    let approval_id = application["approval_event_id"].as_str().expect("canonical approval identity");
    let approval_rows = inspector.project(&ledger, &[ledger.field("event_sequence").unwrap()],
        ReceiptRowFilter::IdEquals(approval_id.to_owned())).await.unwrap();
    let update_rows = inspector.project(&ledger, &[ledger.field("event_sequence").unwrap()],
        ReceiptRowFilter::IdEquals(posture.update.event_ledger_event_id.clone())).await.unwrap();
    context_bundle_assertions::crdt_assert!(equal, approval_rows.len(), 1); context_bundle_assertions::crdt_assert!(equal, update_rows.len(), 1);
    let approval_sequence = approval_rows[0].values["event_sequence"].as_i64().unwrap();
    let update_sequence = update_rows[0].values["event_sequence"].as_i64().unwrap();
    context_bundle_assertions::crdt_assert!(equal, application["approval_event_sequence"], approval_sequence);
    context_bundle_assertions::crdt_assert!(equal, application["update_event_sequence"], update_sequence);
    context_bundle_assertions::crdt_assert!(condition, approval_sequence < update_sequence);
    context_bundle_assertions::crdt_assert!(condition, update_sequence < application_rows[0].values["event_sequence"].as_i64().unwrap());
    context_bundle_assertions::crdt_assert!(different, application["canonical_base_materialization_sha256"], application["canonical_post_materialization_sha256"]);
    {
        use handshake_core::swarm_orchestration::model_lane::{NewModelLaneCrdtProposal, ModelLaneCrdtProposalDecision};
        let proposal_input = |suffix: &str| NewModelLaneCrdtProposal {
            proposal_id: format!("mt005-realization-{suffix}"), document_id: posture.document_id.clone(),
            crdt_document_id: posture.crdt_document_id.clone(), base_update_seq: 1,
            base_state_vector: posture.snapshot.state_vector.clone(),
            proposed_diff: posture.proposal.proposed_diff.clone(),
            source_span_citations: vec![posture.message.parent_span_id.clone().unwrap()],
            actor_id: posture.actor_id.clone(), actor_kind: posture.actor_kind.clone(),
            session_id: posture.session_id.clone(), correlation_id: posture.trace_id.clone(),
            lease_id: posture.lease.lease_id.clone(), kernel_task_run_id: posture.run_id.clone(),
            idempotency_key: format!("mt005-realization-record-{suffix}"),
        };
        let unapproved = harness.document_update.run(harness.store.record_crdt_proposal(proposal_input("unapproved"))).await.unwrap();
        let rejected = harness.document_update.run(harness.store.bind_crdt_proposal_update(&unapproved.proposal_id,
            &posture.update.update_id, &posture.run_id, "mt005-realization-apply-unapproved")).await.unwrap_err();
        context_bundle_assertions::crdt_assert!(condition, rejected.to_string().contains("canonical approval"), "unapproved application reaches the approval guard: {rejected}");
        let late = harness.document_update.run(harness.store.record_crdt_proposal(proposal_input("preapproval-update"))).await.unwrap();
        let (reviewer, reviewer_session) = harness.document_update.run(harness.store.authenticated_crdt_proposal_reviewer()).await.unwrap();
        let late = harness.document_update.run(harness.store.decide_crdt_proposal(&late.proposal_id,
            ModelLaneCrdtProposalDecision::Approved, &reviewer, Some("approve only subsequent realization".into()),
            &posture.run_id, &reviewer_session, "mt005-realization-approve-preapproval-update")).await.unwrap().unwrap();
        let rejected = harness.document_update.run(harness.store.bind_crdt_proposal_update(&late.proposal_id,
            &posture.update.update_id, &posture.run_id, "mt005-realization-apply-preapproval-update")).await.unwrap_err();
        context_bundle_assertions::crdt_assert!(condition, rejected.to_string().contains("identity") || rejected.to_string().contains("approval"),
            "an actual update persisted before canonical approval cannot realize the proposal: {rejected}");
        let rejected = harness.document_update.run(harness.store.bind_crdt_proposal_update(&posture.proposal.proposal_id,
            "mt005-realization-no-persisted-update", &posture.run_id, "mt005-realization-apply-missing-update")).await.unwrap_err();
        context_bundle_assertions::crdt_assert!(condition, rejected.to_string().contains("applied CRDT update authority is unavailable"),
            "missing real update reaches its exact canonical lookup guard: {rejected}");
        use handshake_core::storage::surreal::{RowFilter, ScalarValue};
        let proposals = inspector.table_selector("knowledge_crdt_ai_edit_proposals").await.unwrap();
        for (proposal, state) in [(&unapproved, "proposed"), (&late, "approved")] {
            let rows = inspector.project(&proposals, &[proposals.field("review_state").unwrap(),
                proposals.field("applied_update_id").unwrap(), proposals.field("applied_event_id").unwrap()],
                RowFilter::FieldEquals { field: proposals.field("proposal_id").unwrap(), value: ScalarValue::String(proposal.proposal_id.clone()) }).await.unwrap();
            context_bundle_assertions::crdt_assert!(equal, rows.len(), 1); context_bundle_assertions::crdt_assert!(equal, rows[0].values["review_state"], state);
            context_bundle_assertions::crdt_assert!(condition, rows[0].values["applied_update_id"].is_null()); context_bundle_assertions::crdt_assert!(condition, rows[0].values["applied_event_id"].is_null());
        }
        for key in ["mt005-realization-apply-unapproved", "mt005-realization-apply-preapproval-update", "mt005-realization-apply-missing-update"] {
            context_bundle_assertions::crdt_assert!(equal, inspector.row_count(&ledger, RowFilter::FieldEquals { field: ledger.field("idempotency_key").unwrap(),
                value: ScalarValue::String(key.into()) }).await.unwrap(), 0, "denied realization leaves no application receipt");
        }
    }
    {
        use handshake_core::storage::surreal::{TestFieldMutation, TestMutationValue};
        let original = inspector.project(&ledger, &[ledger.field("payload").unwrap(), ledger.field("payload_hash").unwrap()],
            ReceiptRowFilter::IdEquals(applied_event.clone())).await.unwrap()[0].values.clone();
        for (suffix, field, forged) in [
            ("wrong-root", "path", json!(["another-unapproved-root"])),
            ("wrong-value", "canonical_post_materialization_sha256", json!("00".repeat(32))),
            ("wrong-update", "update_id", json!("missing-canonical-yjs-update")),
            ("stale-base", "base_update_seq", json!(0)),
        ] {
            let mut altered = original["payload"].clone();
            altered["application_evidence"][field] = forged;
            let hash = hex::encode(Sha256::digest(serde_json::to_string(&altered).unwrap().as_bytes()));
            let mutator = harness.storage.test_mutator();
            mutator.update_row(&ledger, applied_event.clone(), &[
                TestFieldMutation::new(ledger.field("payload").unwrap(), TestMutationValue::json(altered)),
                TestFieldMutation::new(ledger.field("payload_hash").unwrap(), TestMutationValue::string(hash)),
            ]).await.unwrap();
            let mut attempted = posture.message.clone();
            attempted.message_id.push_str(&format!("-forged-application-{suffix}"));
            attempted.idempotency_key = attempted.message_id.clone();
            let rejected_id = attempted.message_id.clone();
            let denied = harness.workspace_create.run(harness.store.record_message(attempted)).await;
            mutator.update_row(&ledger, applied_event.clone(), &[
                TestFieldMutation::new(ledger.field("payload").unwrap(), TestMutationValue::json(original["payload"].clone())),
                TestFieldMutation::new(ledger.field("payload_hash").unwrap(), TestMutationValue::json(original["payload_hash"].clone())),
            ]).await.unwrap();
            let error = denied.expect_err("self-authored application evidence cannot establish realization");
            context_bundle_assertions::crdt_assert!(condition, error.to_string().contains("application") || error.to_string().contains("proposal"),
                "canonical realization counterfactual reaches its proposal guard: {error}");
            assert_no_context_derivative(&harness, &rejected_id).await;
            context_bundle_assertions::crdt_assert!(equal, inspector.project(&ledger, &[ledger.field("payload").unwrap(), ledger.field("payload_hash").unwrap()],
                ReceiptRowFilter::IdEquals(applied_event.clone())).await.unwrap()[0].values, original);
        }
    }
    for (table_name, identity_field, identity, fields, snapshot) in [
        ("kernel_crdt_updates", "update_id", posture.update.update_id.as_str(),
            vec!["update_bytes", "update_sha256", "trace_id", "document_id", "crdt_document_id", "owner_account_id", "actor_principal_id", "authenticated_session_id", "access_space_id", "workspace_id"], false),
        ("kernel_crdt_snapshots", "snapshot_id", posture.snapshot.snapshot_id.as_str(),
            vec!["snapshot_bytes", "snapshot_sha256", "actor_kind", "document_id", "crdt_document_id", "owner_account_id", "actor_principal_id", "authenticated_session_id", "access_space_id", "workspace_id"], true),
    ] {
        use handshake_core::storage::surreal::{RowFilter, ScalarValue};
        let table = inspector.table_selector(table_name).await.unwrap();
        let selectors: Vec<_> = fields.iter().map(|field| table.field(field).unwrap()).collect();
        let filter = || RowFilter::FieldEquals { field: table.field(identity_field).unwrap(), value: ScalarValue::String(identity.to_owned()) };
        let before = inspector.project(&table, &selectors, filter()).await.unwrap();
        context_bundle_assertions::crdt_assert!(equal, before.len(), 1, "the mutation probe addresses an actual canonical source");
        context_bundle_assertions::crdt_assert!(equal, before[0].values["document_id"], posture.document_id);
        let mutator = harness.storage.test_mutator();
        let attempt = if snapshot {
            harness.document_update.run(async { Ok::<_, ModelLaneError>(mutator.attempt_record_user_crdt_snapshot_mutation(identity).await) })
                .await.expect("authorize immutable snapshot mutation probe")
        } else {
            harness.document_update.run(async { Ok::<_, ModelLaneError>(mutator.attempt_record_user_crdt_update_mutation(identity).await) })
                .await.expect("authorize immutable update mutation probe")
        };
        match attempt {
            Ok(changed) => {
                context_bundle_assertions::crdt_assert!(condition, !changed, "immutable source cannot mutate under actual Document Update");
            }
            Err(error) => {
                context_bundle_assertions::crdt_assert!(condition, error.to_string().contains("permission") || error.to_string().contains("append-only"),
                    "the actual immutability boundary must reject the mutation: {error}");
            }
        }
        let after = inspector.project(&table, &selectors, filter()).await.unwrap();
        context_bundle_assertions::crdt_assert!(equal, after[0].values, before[0].values, "independent canonical reread preserves complete source bytes and identity");
    }
    let crdt = crdt_metadata(&posture, &source);
    let input = handoff(harness.scope.workspace.as_ref().expect("canonical workspace").as_str(), 
        &posture,
        &source,
        &downstream,
        "crdt-loom",
        ModelLaneHandoffSelectionState::Selected,
        Some(crdt),
    );
    let context_bundle_id = input.context_bundle_id.clone();
    let mut forged_projection = input.clone();
    forged_projection.handoff_id.push_str("-forged-projection");
    forged_projection.idempotency_key = forged_projection.handoff_id.clone();
    forged_projection.crdt_payload.as_mut().unwrap().materialized_projection_hash = "ff".repeat(32);
    forged_projection.context_bundle_id = model_lane_context_bundle_id_for_handoff(&forged_projection).unwrap();
    assert_handoff_denial(&harness, forged_projection, "materialized_projection_hash").await;
    for (suffix, field, value) in [
        ("order", "replay_order_key", json!("forged/replay/order")),
        ("schema", "schema_version", json!("forged-crdt-schema-v9")),
        ("dependencies", "dependency_update_ids", json!(["forged-missing-dependency"])),
    ] {
        let mut forged = input.clone();
        forged.handoff_id.push_str(&format!("-forged-{suffix}"));
        forged.idempotency_key = forged.handoff_id.clone();
        forged.crdt_payload.as_mut().unwrap().replay_metadata[field] = value;
        forged.context_bundle_id = model_lane_context_bundle_id_for_handoff(&forged).unwrap();
        assert_handoff_denial(&harness, forged, &format!("replay_metadata.{field}")).await;
    }
    let stored = harness.workspace_create.run(harness.store.record_context_bundle_handoff(input))
        .await
        .expect("record CRDT and Loom handoff");
    let replay = harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id))
        .await
        .expect("replay CRDT and Loom authority");
    context_bundle_assertions::crdt_assert!(equal, replay, vec![stored]);
    context_bundle_assertions::crdt_assert!(equal, 
        replay[0]
            .crdt_payload
            .as_ref()
            .expect("CRDT metadata")
            .state_vector,
        binding.state_vector
    );
    context_bundle_assertions::crdt_assert!(equal, replay[0].loom_refs.len(), 1);
    {
        use handshake_core::storage::surreal::{RowFilter, TestFieldMutation, TestMutationValue};
        let payload_field = ledger.field("payload").unwrap(); let hash_field = ledger.field("payload_hash").unwrap();
        let event_id = replay[0].event_ledger_event_id.clone();
        let before = inspector.project(&ledger, &[payload_field.clone(), hash_field.clone()], RowFilter::IdEquals(event_id.clone())).await.unwrap();
        context_bundle_assertions::crdt_assert!(equal, before.len(), 1); let before = before[0].values.clone();
        let mut altered = before["payload"].clone();
        let mut payload: serde_json::Value = serde_json::from_str(altered["event_payload_json"].as_str().unwrap()).unwrap();
        payload["record"]["context_bundle_hash"] = json!("f".repeat(64));
        altered["event_payload_json"] = json!(serde_json::to_string(&payload).unwrap());
        let digest = hex::encode(Sha256::digest(altered["event_payload_json"].as_str().unwrap().as_bytes()));
        let mutator = harness.storage.test_mutator();
        mutator.update_row(&ledger, event_id.clone(), &[
            TestFieldMutation::new(payload_field.clone(), TestMutationValue::json(altered)),
            TestFieldMutation::new(hash_field.clone(), TestMutationValue::string(digest)),
        ]).await.unwrap();
        let denied = harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id)).await;
        mutator.update_row(&ledger, event_id.clone(), &[
            TestFieldMutation::new(payload_field.clone(), TestMutationValue::json(before["payload"].clone())),
            TestFieldMutation::new(hash_field.clone(), TestMutationValue::json(before["payload_hash"].clone())),
        ]).await.unwrap();
        let denied = denied.expect_err("a correctly hashed forged handoff receipt cannot replace canonical context authority");
        context_bundle_assertions::crdt_assert!(condition, denied.to_string().contains("context_bundle_hash"), "the actual immutable handoff hash guard is explicit: {denied}");
        context_bundle_assertions::crdt_assert!(equal, inspector.project(&ledger, &[payload_field, hash_field], RowFilter::IdEquals(event_id)).await.unwrap()[0].values, before);
        context_bundle_assertions::crdt_assert!(equal, harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id)).await.unwrap(), replay);
    }
    let mut missing_proposal = posture.message.clone();
    missing_proposal.message_id.push_str("-missing-proposal-authority");
    missing_proposal.idempotency_key = missing_proposal.message_id.clone();
    missing_proposal.kind = ModelLaneMessageKind::Proposal;
    missing_proposal.crdt_proposal_ref = None;
    let rejected_id = missing_proposal.message_id.clone();
    let denied = harness.workspace_create.run(harness.store.record_message(missing_proposal)).await.unwrap_err();
    context_bundle_assertions::crdt_assert!(condition, denied.to_string().contains("crdt_proposal_ref"), "missing proposal reaches its exact authority guard: {denied}");
    assert_no_context_derivative(&harness, &rejected_id).await;
    let cloud_source = record_cloud_source_lane(&harness, &posture).await;
    let canonical_run = harness.workspace_read.run(harness.reader_store.replay_run(&posture.run_id)).await.unwrap();
    let cloud_lane = canonical_run.lanes.iter().find(|lane| lane.lane_id == cloud_source).unwrap();
    let mut cross_lane = posture.message.clone();
    cross_lane.kind = ModelLaneMessageKind::Status; cross_lane.proposal_ref = None; cross_lane.crdt_proposal_ref = None;
    cross_lane.message_id = "mt005-real-cross-lane-crdt-denial".into(); cross_lane.idempotency_key = cross_lane.message_id.clone();
    cross_lane.from_lane_id = cloud_source.clone(); cross_lane.parent_span_id = Some(cloud_lane.lane_span_id.clone());
    cross_lane.locus_binding = cloud_lane.locus_binding.clone();
    let rejected = cross_lane.message_id.clone();
    let denied = harness.workspace_create.run(harness.store.record_message(cross_lane)).await.unwrap_err();
    context_bundle_assertions::crdt_assert!(condition, denied.to_string().contains(&format!("crdt session {} is not uniquely owned by source lane {}", posture.session_id, cloud_source)),
        "the real cloud source cannot claim the local session's CRDT update: {denied}");
    assert_no_context_derivative(&harness, &rejected).await;
    use handshake_core::kernel::crdt::agent_lease::KnowledgeLeaseScopeKind;
    use handshake_core::storage::surreal::observe_context_coordination_busy_for_test;
    let control = harness.store.context_admission_test_control().await.unwrap();
    let (admission, resume) = control.pause_next().unwrap();
    let mut held_message = posture.message.clone();
    held_message.message_id = format!("{}-admitted-before-release", held_message.message_id);
    held_message.idempotency_key = held_message.message_id.clone();
    held_message.message_span_id = format!("span-{}", held_message.message_id);
    let store = harness.store.clone();
    let operation = harness.workspace_create.clone();
    let admitted = tokio::spawn(async move { operation.run(store.record_message(held_message)).await });
    let observed = tokio::time::timeout(std::time::Duration::from_secs(10), admission).await.unwrap().unwrap();
    context_bundle_assertions::crdt_assert!(equal, observed.document_id, posture.document_id);
    context_bundle_assertions::crdt_assert!(equal, observed.crdt_document_id, posture.crdt_document_id);
    context_bundle_assertions::crdt_assert!(equal, observed.lease_id, posture.lease.lease_id);
    let (busy_tx, mut busy_rx) = tokio::sync::mpsc::unbounded_channel();
    let independent = harness.store.clone();
    let original_operation = harness.document_update.clone();
    let lease_id = posture.lease.lease_id.clone();
    let document_id = posture.document_id.clone();
    let release = tokio::spawn(async move {
        observe_context_coordination_busy_for_test(busy_tx, original_operation.run(independent.release_crdt_lease(
            &lease_id, KnowledgeLeaseScopeKind::Document, &document_id, "mt005-release-after-held-admission"))).await
    });
    let busy = tokio::time::timeout(std::time::Duration::from_secs(10), busy_rx.recv()).await.unwrap().unwrap();
    context_bundle_assertions::crdt_assert!(equal, busy.domain_kind, "workspace");
    context_bundle_assertions::crdt_assert!(equal, busy.domain_id, harness.scope.workspace.as_ref().unwrap().as_str());
    context_bundle_assertions::crdt_assert!(different, busy.operation_id, uuid::Uuid::nil());
    resume.send(()).unwrap();
    let admitted = tokio::time::timeout(std::time::Duration::from_secs(10), admitted).await.unwrap().unwrap().unwrap();
    let released = tokio::time::timeout(std::time::Duration::from_secs(10), release).await.unwrap().unwrap().unwrap().unwrap();
    context_bundle_assertions::crdt_assert!(condition, released.released_at_utc.is_some());
    context_bundle_assertions::crdt_assert!(condition, admitted.crdt_authority_binding.is_some(), "the valid held admission succeeds before release");
    let historical = harness.workspace_read.run(harness.reader_store.replay_run(&posture.run_id)).await.unwrap();
    context_bundle_assertions::crdt_assert!(condition, historical.messages.iter().any(|message| message == &admitted), "canonical admitted binding survives release");
    let mut new_admission = posture.message.clone();
    new_admission.message_id = format!("{}-after-release", new_admission.message_id);
    new_admission.idempotency_key = new_admission.message_id.clone();
    let rejected_id = new_admission.message_id.clone();
    let denied = harness.workspace_create.run(harness.store.record_message(new_admission)).await.unwrap_err();
    context_bundle_assertions::crdt_assert!(condition, denied.to_string().contains("exactly one active scoped lease"), "new admission after release must reach the full covering-set guard: {denied}");
    assert_no_context_derivative(&harness, &rejected_id).await;
    let claim = |suffix: &str, ttl_seconds| handshake_core::swarm_orchestration::model_lane::NewModelLaneCrdtLease {
        lease_id: format!("mt005-{}-{suffix}", posture.run_id), lane_id: posture.lane_id.clone(),
        document_id: posture.document_id.clone(), crdt_document_id: posture.crdt_document_id.clone(),
        actor_id: posture.actor_id.clone(), actor_kind: posture.actor_kind.clone(), session_id: posture.session_id.clone(),
        correlation_id: posture.trace_id.clone(), ttl_seconds, kernel_task_run_id: posture.run_id.clone(),
        idempotency_key: format!("mt005-claim-{}-{suffix}", posture.run_id),
    };
    use handshake_core::swarm_orchestration::model_lane::ModelLaneCrdtLeaseClaimOutcome;
    let mut correlation_probe = claim("correlation-mismatch", 3600);
    correlation_probe.correlation_id = "trace-unrelated-crdt-update".into();
    let correlation_lease = match harness.document_update.run(harness.store.claim_crdt_lease(correlation_probe)).await.unwrap() {
        ModelLaneCrdtLeaseClaimOutcome::Claimed(lease) => lease, other => panic!("actual correlation probe claim required: {other:?}"),
    };
    let mut probe = posture.message.clone();
    probe.kind = ModelLaneMessageKind::Status; probe.proposal_ref = None; probe.crdt_proposal_ref = None;
    probe.message_id = "mt005-correlation-mismatch-admission".into(); probe.idempotency_key = probe.message_id.clone();
    let rejected = probe.message_id.clone();
    let denied = harness.workspace_create.run(harness.store.record_message(probe)).await.unwrap_err();
    context_bundle_assertions::crdt_assert!(condition, denied.to_string().contains("lease"), "actual foreign-correlation lease does not cover admission: {denied}");
    assert_no_context_derivative(&harness, &rejected).await;
    harness.document_update.run(harness.store.release_crdt_lease(&correlation_lease.lease_id,
        KnowledgeLeaseScopeKind::Document, &posture.document_id, "mt005-release-correlation-mismatch")).await.unwrap().unwrap();
    let claim_race = match harness.document_update.run(harness.store.claim_crdt_lease(claim("claim-race", 3600))).await.unwrap() {
        ModelLaneCrdtLeaseClaimOutcome::Claimed(lease) => lease, other => panic!("actual claim required: {other:?}"),
    };
    let renewed = harness.document_update.run(harness.store.renew_crdt_lease(&claim_race.lease_id,
        KnowledgeLeaseScopeKind::Document, &posture.document_id, 3600, "mt005-renew-actual-document-lease"))
        .await.unwrap().expect("canonical lease renews through original Document Update");
    context_bundle_assertions::crdt_assert!(equal, renewed.lease_id, claim_race.lease_id);
    context_bundle_assertions::crdt_assert!(equal, renewed.renewal_count, claim_race.renewal_count + 1);
    context_bundle_assertions::crdt_assert!(different, renewed.last_transition_event_id, claim_race.last_transition_event_id);
    let (entered, resume) = control.pause_next().unwrap();
    let mut message = posture.message.clone(); message.kind = ModelLaneMessageKind::Status; message.proposal_ref = None; message.crdt_proposal_ref = None;
    message.message_id = "mt005-workspace-claim-race".into(); message.idempotency_key = message.message_id.clone();
    let store = harness.store.clone(); let operation = harness.workspace_create.clone();
    let admission = tokio::spawn(async move { operation.run(store.record_message(message)).await });
    let held = tokio::time::timeout(std::time::Duration::from_secs(10), entered).await.unwrap().unwrap();
    context_bundle_assertions::crdt_assert!(equal, held.lease_id, claim_race.lease_id);
    let (busy_tx, mut busy_rx) = tokio::sync::mpsc::unbounded_channel();
    let store = harness.store.clone(); let operation = harness.workspace_create.clone();
    let workspace = harness.scope.workspace.as_ref().unwrap().as_str().to_owned(); let workspace_claim = claim("workspace-race", 3600);
    let second = tokio::spawn(async move { observe_context_coordination_busy_for_test(busy_tx,
        operation.run(store.claim_crdt_lease_for_scope(workspace_claim, KnowledgeLeaseScopeKind::Workspace, workspace))).await });
    let busy = tokio::time::timeout(std::time::Duration::from_secs(10), busy_rx.recv()).await.unwrap().unwrap();
    context_bundle_assertions::crdt_assert!(equal, busy.domain_kind, "workspace"); context_bundle_assertions::crdt_assert!(equal, busy.domain_id, harness.scope.workspace.as_ref().unwrap().as_str());
    resume.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), admission).await.unwrap().unwrap().expect("held admission wins covering workspace claim");
    let workspace_lease = match tokio::time::timeout(std::time::Duration::from_secs(10), second).await.unwrap().unwrap().unwrap() { ModelLaneCrdtLeaseClaimOutcome::Claimed(lease) => lease,
        other => panic!("workspace overlap must be representable after admission: {other:?}"), };
    let mut ambiguous = posture.message.clone(); ambiguous.kind = ModelLaneMessageKind::Status; ambiguous.proposal_ref = None; ambiguous.crdt_proposal_ref = None;
    ambiguous.message_id = "mt005-ambiguous-covering-set".into(); ambiguous.idempotency_key = ambiguous.message_id.clone();
    let ambiguous_id = ambiguous.message_id.clone();
    let denied = harness.workspace_create.run(harness.store.record_message(ambiguous)).await.unwrap_err();
    context_bundle_assertions::crdt_assert!(condition, denied.to_string().contains("exactly one active scoped lease"), "both covering leases remain visible to the cardinality guard: {denied}");
    assert_no_context_derivative(&harness, &ambiguous_id).await;
    harness.document_update.run(harness.store.release_crdt_lease(&claim_race.lease_id, KnowledgeLeaseScopeKind::Document,
        &posture.document_id, "release-document-claim-race")).await.unwrap().unwrap();
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&harness.scope).unwrap();
    let workspace_update = AuthenticatedModelLaneTestOperation::authorize(harness.storage.clone(), AuthorizationRequest {
        session_token: harness.account.session_token.clone(), channel_binding_hash: Some({ use sha2::{Digest,Sha256};
            hex::encode(Sha256::digest(harness.account.channel_binding_token.as_bytes())) }),
        capability_id: "fs.write".into(), resource_kind: ResourceKind::Workspace,
        external_resource_id: harness.scope.workspace.as_ref().unwrap().as_str().to_owned(), action: ResourceAction::Update,
    }, exact).await.unwrap();
    workspace_update.run(harness.store.release_crdt_lease(&workspace_lease.lease_id, KnowledgeLeaseScopeKind::Workspace,
        "", "release-workspace-claim-race")).await.unwrap().unwrap();
    let expiry = match harness.document_update.run(harness.store.claim_crdt_lease(claim("natural-expiry", 3))).await.unwrap() {
        ModelLaneCrdtLeaseClaimOutcome::Claimed(lease) => lease, other => panic!("actual expiring claim required: {other:?}"),
    };
    let (entered, resume) = control.pause_next().unwrap();
    let mut message = posture.message.clone(); message.kind = ModelLaneMessageKind::Status; message.proposal_ref = None; message.crdt_proposal_ref = None;
    message.message_id = "mt005-natural-expiry-admitted".into(); message.idempotency_key = message.message_id.clone();
    let store = harness.store.clone(); let operation = harness.workspace_create.clone();
    let admission = tokio::spawn(async move { operation.run(store.record_message(message)).await });
    let held = tokio::time::timeout(std::time::Duration::from_secs(10), entered).await.unwrap().unwrap();
    context_bundle_assertions::crdt_assert!(condition, held.admitted_at < expiry.expires_at_utc, "engine admission precedes source expiry");
    if let Ok(remaining) = (expiry.expires_at_utc - chrono::Utc::now()).to_std() { tokio::time::sleep(remaining).await; }
    let (busy_tx, mut busy_rx) = tokio::sync::mpsc::unbounded_channel();
    let store = harness.store.clone(); let operation = harness.document_update.clone();
    let lease_id = expiry.lease_id.clone(); let document_id = posture.document_id.clone();
    let sweep = tokio::spawn(async move { observe_context_coordination_busy_for_test(busy_tx,
        operation.run(store.expire_crdt_lease(&lease_id, KnowledgeLeaseScopeKind::Document, &document_id, "mt005-expire-held-source"))).await });
    let busy = tokio::time::timeout(std::time::Duration::from_secs(10), busy_rx.recv()).await.unwrap().unwrap();
    context_bundle_assertions::crdt_assert!(equal, busy.domain_kind, "workspace");
    resume.send(()).unwrap();
    let admitted = tokio::time::timeout(std::time::Duration::from_secs(10), admission).await.unwrap().unwrap().expect("natural source expiry cannot invalidate already held admission");
    let expired = tokio::time::timeout(std::time::Duration::from_secs(10), sweep).await.unwrap().unwrap().unwrap().unwrap();
    context_bundle_assertions::crdt_assert!(different, expired.last_transition_event_id, expired.recorded_event_id);
    use handshake_core::storage::surreal::{RowFilter, ScalarValue};
    let inspector = harness.storage.test_inspector(); let table = inspector.table_selector("knowledge_crdt_agent_lane_leases").await.unwrap();
    let rows = inspector.project(&table, &[table.field("expired_at_utc").unwrap()], RowFilter::FieldEquals {
        field: table.field("lease_id").unwrap(), value: ScalarValue::String(expired.lease_id.clone()),
    }).await.unwrap();
    context_bundle_assertions::crdt_assert!(equal, rows.len(), 1); context_bundle_assertions::crdt_assert!(condition, rows[0].values.get("expired_at_utc").is_some_and(|value| !value.is_null()));
    context_bundle_assertions::crdt_assert!(condition, harness.workspace_read.run(harness.reader_store.replay_run(&posture.run_id)).await.unwrap().messages.contains(&admitted));
    let mut expired_admission = posture.message.clone();
    expired_admission.kind = ModelLaneMessageKind::Status;
    expired_admission.proposal_ref = None;
    expired_admission.crdt_proposal_ref = None;
    expired_admission.message_id = "mt005-new-admission-after-natural-expiry".into();
    expired_admission.idempotency_key = expired_admission.message_id.clone();
    let rejected_id = expired_admission.message_id.clone();
    let denied = harness.workspace_create.run(harness.store.record_message(expired_admission)).await.unwrap_err();
    context_bundle_assertions::crdt_assert!(condition, denied.to_string().contains("lease"), "new admission after natural expiry reaches the lease guard: {denied}");
    assert_no_context_derivative(&harness, &rejected_id).await;
    let unrelated = match harness.workspace_create.run(harness.store.claim_crdt_lease_for_scope(
        claim("unrelated-index-run", 3600), KnowledgeLeaseScopeKind::IndexRun, "unrelated-index-run-mt005".into(),
    )).await.unwrap() {
        ModelLaneCrdtLeaseClaimOutcome::Claimed(lease) => lease, other => panic!("actual unrelated-scope lease required: {other:?}"),
    };
    let mut unrelated_admission = posture.message.clone();
    unrelated_admission.kind = ModelLaneMessageKind::Status;
    unrelated_admission.proposal_ref = None; unrelated_admission.crdt_proposal_ref = None;
    unrelated_admission.message_id = "mt005-unrelated-scope-admission".into();
    unrelated_admission.idempotency_key = unrelated_admission.message_id.clone();
    let rejected = unrelated_admission.message_id.clone();
    let denied = harness.workspace_create.run(harness.store.record_message(unrelated_admission)).await.unwrap_err();
    context_bundle_assertions::crdt_assert!(condition, denied.to_string().contains("exactly one active scoped lease"), "IndexRun does not authorize a Document admission: {denied}");
    assert_no_context_derivative(&harness, &rejected).await;
    workspace_update.run(harness.store.release_crdt_lease(&unrelated.lease_id, KnowledgeLeaseScopeKind::IndexRun,
        "", "mt005-release-unrelated-index-run")).await.unwrap().unwrap();
    assert_missing_crdt_binding_denied(&harness, &source).await;
    use handshake_core::test_harness::crdt_workspace::{
        build_surreal_crdt_realization_counterfactual_authenticated, CrdtProposalRealizationCounterfactual,
    };
    for (suffix, counterfactual) in [
        ("wrong-root", CrdtProposalRealizationCounterfactual::WrongRoot),
        ("wrong-value", CrdtProposalRealizationCounterfactual::WrongValue),
        ("extra-root", CrdtProposalRealizationCounterfactual::ExtraRoot),
    ] {
        let label = format!("mt005-realization-{suffix}");
        let error = match build_surreal_crdt_realization_counterfactual_authenticated(
            &harness.store, harness.scope.workspace.as_ref().unwrap().as_str(), &label, &harness.model_id,
            &harness.document_id, &harness.workspace_create, &harness.document_update, counterfactual,
        ).await {
            Ok(_) => panic!("unapproved real Yjs {suffix} change was accepted"),
            Err(error) => error,
        };
        context_bundle_assertions::crdt_assert!(condition, error.to_string().contains("realize") || error.to_string().contains("root"),
            "the actual next update must reach typed realization rejection: {error}");
        let inspector = harness.storage.test_inspector();
        let proposals = inspector.table_selector("knowledge_crdt_ai_edit_proposals").await.unwrap();
        let proposal_rows = inspector.project(&proposals, &[
            proposals.field("review_state").unwrap(), proposals.field("applied_update_id").unwrap(),
            proposals.field("applied_event_id").unwrap(), proposals.field("diff_sha256").unwrap(),
        ], RowFilter::FieldEquals {
            field: proposals.field("proposal_id").unwrap(), value: ScalarValue::String(format!("proposal-mt018-{label}")),
        }).await.unwrap();
        context_bundle_assertions::crdt_assert!(equal, proposal_rows.len(), 1, "independent canonical proposal remains after binder rejection");
        context_bundle_assertions::crdt_assert!(equal, proposal_rows[0].values["review_state"], "approved");
        context_bundle_assertions::crdt_assert!(condition, proposal_rows[0].values["applied_update_id"].is_null());
        context_bundle_assertions::crdt_assert!(condition, proposal_rows[0].values["applied_event_id"].is_null());
        let table = inspector.table_selector("kernel_event_ledger").await.unwrap();
        let rows = inspector.project(&table, &[table.field("event_id").unwrap()], RowFilter::FieldEquals {
            field: table.field("idempotency_key").unwrap(), value: ScalarValue::String(format!("mt018-proposal-apply-{label}")),
        }).await.unwrap();
        context_bundle_assertions::crdt_assert!(condition, rows.is_empty(), "rejected realization has no application receipt or partial promotion");
        assert_no_context_derivative(&harness, &format!("message-mt018-{label}")).await;
    }
    drop(workspace_update);
    {
        use handshake_core::swarm_orchestration::model_lane::ModelLaneCrdtProposalDecision;
        use handshake_core::storage::surreal::{RowFilter, ScalarValue, TestFieldMutation, TestMutationValue};
        let (reviewer, reviewer_session) = harness.document_update.run(harness.store.authenticated_crdt_proposal_reviewer()).await.unwrap();
        let promotion_key = "mt005-real-canonical-promotion";
        let ledger = inspector.table_selector("kernel_event_ledger").await.unwrap();
        let count = || RowFilter::FieldEquals { field: ledger.field("aggregate_id").unwrap(), value: ScalarValue::String(posture.proposal.proposal_id.clone()) };
        let before_count = inspector.row_count(&ledger, count()).await.unwrap();
        let proposals = inspector.table_selector("knowledge_crdt_ai_edit_proposals").await.unwrap();
        let immutable_fields = [proposals.field("decided_event_id").unwrap(), proposals.field("applied_event_id").unwrap(),
            proposals.field("decided_by").unwrap(), proposals.field("decided_at_utc").unwrap(), proposals.field("decision_reason").unwrap()];
        let proposal_filter = || RowFilter::FieldEquals { field: proposals.field("proposal_id").unwrap(), value: ScalarValue::String(posture.proposal.proposal_id.clone()) };
        let original_evidence = inspector.project(&proposals, &immutable_fields, proposal_filter()).await.unwrap();
        let denied = harness.document_update.run(harness.store.decide_crdt_proposal(&posture.proposal.proposal_id,
            ModelLaneCrdtProposalDecision::Promoted, &posture.actor_id, None, &posture.run_id, &reviewer_session,
            "mt005-deny-model-self-promotion")).await.unwrap_err();
        assert!(denied.to_string().contains("eligible reviewer"), "model creator cannot replace authenticated operator: {denied}");
        assert_eq!(inspector.row_count(&ledger, count()).await.unwrap(), before_count);
        assert!(harness.document_update.run(harness.store.decide_crdt_proposal(&posture.proposal.proposal_id,
            ModelLaneCrdtProposalDecision::Promoted, &reviewer, None, &posture.run_id, "foreign-authenticated-session",
            "mt005-deny-foreign-promotion-session")).await.is_err());
        assert!(harness.document_update.run(harness.store.decide_crdt_proposal("missing-canonical-proposal",
            ModelLaneCrdtProposalDecision::Promoted, &reviewer, None, &posture.run_id, &reviewer_session,
            "mt005-deny-missing-promotion-source")).await.unwrap().is_none());
        assert!(harness.document_update.run(harness.store.decide_crdt_proposal("mt005-realization-unapproved",
            ModelLaneCrdtProposalDecision::Promoted, &reviewer, None, &posture.run_id, &reviewer_session,
            "mt005-deny-unapproved-promotion")).await.is_err());
        let updates = inspector.table_selector("kernel_crdt_updates").await.unwrap();
        let source_update = inspector.project(&updates, &[updates.field("update_seq").unwrap()], RowFilter::FieldEquals {
            field: updates.field("update_id").unwrap(), value: ScalarValue::String(posture.update.update_id.clone()),
        }).await.unwrap();
        assert_eq!(source_update.len(), 1);
        let update_id = source_update[0].record_id.key_string().unwrap().to_owned();
        let mutator = harness.storage.test_mutator();
        mutator.update_row(&updates, update_id.clone(), &[TestFieldMutation::new(updates.field("update_seq").unwrap(),
            TestMutationValue::json(json!(posture.update.update_seq + 1)))]).await.unwrap();
        let stale = harness.document_update.run(harness.store.decide_crdt_proposal(&posture.proposal.proposal_id,
            ModelLaneCrdtProposalDecision::Promoted, &reviewer, None, &posture.run_id, &reviewer_session,
            "mt005-deny-stale-promotion-source")).await;
        mutator.update_row(&updates, update_id, &[TestFieldMutation::new(updates.field("update_seq").unwrap(),
            TestMutationValue::json(source_update[0].values["update_seq"].clone()))]).await.unwrap();
        assert!(stale.is_err() || stale.unwrap().is_none(), "stale actual canonical update cannot be promoted");
        assert_eq!(inspector.row_count(&ledger, count()).await.unwrap(), before_count);
        let promoted = harness.document_update.run(harness.store.decide_crdt_proposal(&posture.proposal.proposal_id,
            ModelLaneCrdtProposalDecision::Promoted, &reviewer, Some("promote exact canonical realization".into()),
            &posture.run_id, &reviewer_session, promotion_key)).await.unwrap().unwrap();
        assert_eq!(promoted.review_state, "promoted");
        assert_eq!(promoted.recorded_event_id, posture.proposal.recorded_event_id);
        assert_eq!(promoted.applied_event_id, posture.proposal.applied_event_id);
        assert_eq!(promoted.decided_by, posture.proposal.decided_by);
        assert_eq!(promoted.actor_id, posture.actor_id);
        assert_eq!(promoted.session_id, posture.session_id);
        assert_eq!(inspector.project(&proposals, &immutable_fields, proposal_filter()).await.unwrap(), original_evidence);
        assert_eq!(inspector.row_count(&ledger, count()).await.unwrap(), before_count + 2);
        let requested = promoted.promotion_requested_event_id.as_ref().unwrap();
        let accepted = promoted.promotion_accepted_event_id.as_ref().unwrap();
        assert_ne!(requested, accepted);
        let fields = [ledger.field("event_sequence").unwrap(), ledger.field("causation_id").unwrap(), ledger.field("actor_id").unwrap(), ledger.field("session_run_id").unwrap()];
        let requested_row = inspector.project(&ledger, &fields, RowFilter::IdEquals(requested.clone())).await.unwrap();
        let accepted_row = inspector.project(&ledger, &fields, RowFilter::IdEquals(accepted.clone())).await.unwrap();
        assert_eq!(requested_row.len(), 1); assert_eq!(accepted_row.len(), 1);
        assert!(requested_row[0].values["event_sequence"].as_i64().unwrap() > application_rows[0].values["event_sequence"].as_i64().unwrap());
        assert!(accepted_row[0].values["event_sequence"].as_i64().unwrap() > requested_row[0].values["event_sequence"].as_i64().unwrap());
        assert_eq!(accepted_row[0].values["causation_id"], json!(requested));
        assert_eq!(accepted_row[0].values["actor_id"], json!(reviewer));
        assert_eq!(accepted_row[0].values["session_run_id"], json!(reviewer_session));
        let retry = harness.document_update.run(harness.store.decide_crdt_proposal(&posture.proposal.proposal_id,
            ModelLaneCrdtProposalDecision::Promoted, &reviewer, Some("promote exact canonical realization".into()),
            &posture.run_id, &reviewer_session, promotion_key)).await.unwrap().unwrap();
        assert_eq!(retry, promoted);
        assert_eq!(inspector.row_count(&ledger, count()).await.unwrap(), before_count + 2);
        assert!(harness.document_update.run(harness.store.decide_crdt_proposal(&posture.proposal.proposal_id,
            ModelLaneCrdtProposalDecision::Promoted, &reviewer, Some("different immutable body".into()),
            &posture.run_id, &reviewer_session, promotion_key)).await.is_err());
        assert_eq!(inspector.row_count(&ledger, count()).await.unwrap(), before_count + 2);
        let mutator = harness.storage.test_mutator();
        mutator.update_row(&ledger, accepted.clone(), &[TestFieldMutation::new(ledger.field("causation_id").unwrap(),
            TestMutationValue::string("missing-or-foreign-request"))]).await.unwrap();
        let malformed = harness.workspace_read.run(harness.reader_store.crdt_proposal(&posture.proposal.proposal_id)).await;
        mutator.update_row(&ledger, accepted.clone(), &[TestFieldMutation::new(ledger.field("causation_id").unwrap(),
            TestMutationValue::string(requested.clone()))]).await.unwrap();
        assert!(malformed.is_err() || malformed.unwrap().is_none(), "broken accepted causation cannot become a canonical promoted proposal");
        assert_eq!(harness.workspace_read.run(harness.reader_store.crdt_proposal(&posture.proposal.proposal_id)).await.unwrap().unwrap(), promoted);
        assert_eq!(inspector.row_count(&ledger, count()).await.unwrap(), before_count + 2);
    }
    let captured = harness.workspace_read.run(harness.reader_store.consume_context_bundle_for_downstream(
        &posture.run_id, &context_bundle_id, &downstream)).await.unwrap();
    (context_bundle_id, replay, captured)
    };
    harness = harness.reopen().await;
    let reopened_proposal = harness.workspace_read.run(harness.reader_store.crdt_proposal(&posture.proposal.proposal_id)).await.unwrap().unwrap();
    assert_eq!(reopened_proposal.review_state, "promoted");
    assert_eq!(reopened_proposal.recorded_event_id, posture.proposal.recorded_event_id);
    assert_eq!(reopened_proposal.applied_event_id, posture.proposal.applied_event_id);
    let reopened = harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(
        &posture.run_id, &context_bundle_id)).await.unwrap();
    context_bundle_assertions::crdt_assert!(equal, reopened, canonical_handoffs, "same-owner durable reopen preserves exact CRDT/Loom handoffs");
    context_bundle_assertions::crdt_assert!(equal, harness.workspace_read.run(harness.reader_store.consume_context_bundle_for_downstream(
        &posture.run_id, &context_bundle_id, &downstream)).await.unwrap(), captured_context,
        "reopened canonical run, downstream lane, artifact, vector, projection and bundle match the captured tuple");
    context_bundle_assertions::crdt_assert!(condition, harness.workspace_read.run(harness.reader_store.replay_run(&posture.run_id)).await.unwrap().messages.contains(&source),
        "original admitted source remains historically replayable after release, expiry and reopen");
    harness.cleanup().await;
}

#[tokio::test]
async fn context_bundle_record_and_consume_share_one_canonical_lock_order() {
    let harness = Harness::create("handoff-concurrent").await;
    let posture = harness.posture("handoff-concurrent").await;
    let source = harness.workspace_create.run(harness.store.record_message(posture.message.clone()))
        .await
        .expect("record source message");
    let downstream = record_downstream_lane(&harness, &posture).await;
    harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(artifact_binding(&posture, &source)))
        .await
        .expect("record exact artifact binding");
    let first = handoff(harness.scope.workspace.as_ref().expect("canonical workspace").as_str(), 
        &posture,
        &source,
        &downstream,
        "concurrent-first",
        ModelLaneHandoffSelectionState::Selected,
        None,
    );
    let context_bundle_id = first.context_bundle_id.clone();
    harness.workspace_create.run(harness.store.record_context_bundle_handoff(first))
        .await
        .expect("seed consumable handoff");

    for round in 0..12 {
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let writer_store = harness.store.clone();
    let writer_operation = harness.workspace_create.clone();
    let writer_barrier = barrier.clone();
    let second = handoff(harness.scope.workspace.as_ref().expect("canonical workspace").as_str(), 
        &posture,
        &source,
        &downstream,
        &format!("concurrent-second-{round}"),
        ModelLaneHandoffSelectionState::Rejected,
        None,
    );
    let writer = tokio::spawn(async move {
        writer_barrier.wait().await;
        writer_operation.run(writer_store.record_context_bundle_handoff(second)).await
    });
    let reader_store = harness.reader_store.clone();
    let reader_operation = harness.workspace_read.clone();
    let reader_barrier = barrier.clone();
    let reader_run = posture.run_id.clone();
    let reader_bundle = context_bundle_id.clone();
    let reader_lane = downstream.clone();
    let reader = tokio::spawn(async move {
        reader_barrier.wait().await;
        reader_operation.run(reader_store.consume_context_bundle_for_downstream(&reader_run, &reader_bundle, &reader_lane))
            .await
    });
    barrier.wait().await;
    let (recorded, consumed) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(writer, reader)
    })
    .await
    .expect("record/consume lock ordering must not deadlock");
    recorded
        .expect("writer joins")
        .expect("concurrent handoff persists");
    let consumed = consumed
        .expect("reader joins")
        .expect("concurrent consumer returns canonical prefix");
    let later_replay = harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id))
        .await.expect("canonical later replay");
    context_bundle_assertions::lock_order_assert!(condition, later_replay.starts_with(&consumed.records),
        "captured canonical snapshot must remain an unchanged ordered prefix of later authority");
    context_bundle_assertions::lock_order_assert!(equal, consumed.allowed_context.get("handoffs"), Some(&serde_json::to_value(&consumed.records).unwrap()),
        "the consumed context uses the same captured canonical records, not a second unrelated replay");
    context_bundle_assertions::lock_order_assert!(equal, consumed.context_hash, hex::encode(Sha256::digest(
        handshake_storage_support::canonical_json::canonical_json_bytes(&consumed.allowed_context))),
        "the immutable context hash covers that captured consistent snapshot");
    context_bundle_assertions::lock_order_assert!(condition, consumed.records.windows(2).all(|pair| pair[0].event_ledger_seq <= pair[1].event_ledger_seq));
    context_bundle_assertions::lock_order_assert!(equal, consumed.records.iter().filter(|record| record.selection_state == ModelLaneHandoffSelectionState::Selected).count(), 1);
    }
    harness.cleanup().await;
}

#[tokio::test]
async fn context_bundle_consumer_blocks_on_run_before_acquiring_downstream_lane() {
    use handshake_core::storage::surreal::observe_context_coordination_busy_for_test;
    use handshake_core::swarm_orchestration::model_lane::ModelLaneError;
    let harness = Harness::create("run-before-lane").await;
    let posture = harness.posture("run-before-lane").await;
    let source = harness.workspace_create.run(harness.store.record_message(posture.message.clone()))
        .await.expect("canonical source message");
    let downstream = record_downstream_lane(&harness, &posture).await;
    harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(artifact_binding(&posture, &source)))
        .await.expect("canonical source artifact");
    let input = handoff(harness.scope.workspace.as_ref().expect("canonical workspace").as_str(), &posture, &source, &downstream, "run-before-lane",
        ModelLaneHandoffSelectionState::Selected, None);
    let bundle_id = input.context_bundle_id.clone();
    harness.workspace_create.run(harness.store.record_context_bundle_handoff(input)).await.expect("canonical handoff");
    let independent_store = || {
        let lifecycle = ResourceAccessLifecycleRegistry::default();
        lifecycle.register_active(ExactResourceScopeAttribution::try_from_resource_scope(&harness.scope)
            .expect("original exact tuple")).expect("independent authenticated lifecycle");
        ModelLaneStore::new(harness.storage.clone(), ResourceAccessContext::for_exact_reader_with_lifecycle(
            ExactResourceScopeAttribution::try_from_resource_scope(&harness.scope).expect("original exact reader tuple"), lifecycle))
            .with_model_catalog(harness.catalog.clone())
    };
    let run_store = independent_store();
    let lane_store = independent_store();
    let reader_store = independent_store();
    let mut run_holder = harness.workspace_read.run(run_store.hold_context_bundle_fence_for_test(&posture.run_id, None))
        .await.expect("hold actual committed run gate");
    let (observer, mut observations) = tokio::sync::mpsc::unbounded_channel();
    let read_operation = harness.workspace_read.clone();
    let reader_run = posture.run_id.clone();
    let reader_lane = downstream.clone();
    let reader_bundle = bundle_id.clone();
    let reader = tokio::spawn(async move {
        observe_context_coordination_busy_for_test(observer,
            read_operation.run(reader_store.consume_context_bundle_for_downstream(&reader_run, &reader_bundle, &reader_lane))).await
    });
    let busy = tokio::time::timeout(std::time::Duration::from_secs(10), observations.recv()).await
        .expect("observe actual database run acquisition").expect("actual consumer busy observation");
    context_bundle_assertions::run_first_assert!(equal, busy.domain_kind, "run"); context_bundle_assertions::run_first_assert!(equal, busy.domain_id, posture.run_id);
    context_bundle_assertions::run_first_assert!(condition, !busy.operation_id.is_nil(), "busy observation binds the actual consumer operation");
    let mut lane_holder = tokio::time::timeout(std::time::Duration::from_secs(10),
        harness.workspace_read.run(lane_store.hold_context_bundle_fence_for_test(&posture.run_id, Some(&downstream))))
        .await.expect("downstream lane remains free while consumer waits on run").expect("actual independent lane gate");
    harness.workspace_read.run(async { lane_holder.release().await.map_err(ModelLaneError::from) })
        .await.expect("release independent lane gate");
    harness.workspace_read.run(async { run_holder.release().await.map_err(ModelLaneError::from) })
        .await.expect("release original run gate");
    let consumed = tokio::time::timeout(std::time::Duration::from_secs(10), reader).await
        .expect("consume succeeds after actual run release").expect("owned reader joins").expect("canonical consume");
    let replay = harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(&posture.run_id, &bundle_id))
        .await.expect("canonical captured handoff");
    context_bundle_assertions::run_first_assert!(equal, consumed.records, replay);
    // Cancellation while the owned supervisor has the run and waits for a lane.
    let lane_blocker_store = independent_store();
    let mut lane_blocker = harness.workspace_read.run(lane_blocker_store.hold_context_bundle_fence_for_test(
        &posture.run_id, Some(&downstream))).await.unwrap();
    let cancel_store = independent_store(); let cancel_operation = harness.workspace_read.clone();
    let cancel_run = posture.run_id.clone(); let cancel_lane = downstream.clone(); let cancel_bundle = bundle_id.clone();
    let (observer, mut observations) = tokio::sync::mpsc::unbounded_channel();
    let cancelled_reader = tokio::spawn(async move {
        observe_context_coordination_busy_for_test(observer, cancel_operation.run(cancel_store.consume_context_bundle_for_downstream(
            &cancel_run, &cancel_bundle, &cancel_lane))).await
    });
    let busy = tokio::time::timeout(std::time::Duration::from_secs(10), observations.recv()).await.unwrap().unwrap();
    assert_eq!(busy.domain_kind, "lane"); assert_eq!(busy.domain_id, downstream);
    cancelled_reader.abort(); assert!(cancelled_reader.await.unwrap_err().is_cancelled());
    harness.workspace_read.run(async { Ok::<_, ModelLaneError>(lane_blocker.release().await) })
        .await.expect("authorize lane blocker release").unwrap();
    let verify_store = independent_store();
    let mut released_run = tokio::time::timeout(std::time::Duration::from_secs(10),
        harness.workspace_read.run(verify_store.hold_context_bundle_fence_for_test(&posture.run_id, None)))
        .await.expect("cancelled lane acquisition supervisor releases its actually acquired run").unwrap();
    harness.workspace_read.run(async { Ok::<_, ModelLaneError>(released_run.release().await) })
        .await.expect("authorize cancelled-acquisition run release").unwrap();
    drop(released_run); drop(lane_blocker); drop(lane_blocker_store); drop(verify_store);

    // Drop the caller after the real authenticated worker has observed commit,
    // before completion delivery: the supervisor still owns final rotated guards.
    let completion = harness.reader_store.context_completion_test_control().await.unwrap();
    let (committed, resume) = completion.pause_next().unwrap();
    let completed_store = harness.reader_store.clone(); let completed_operation = harness.workspace_read.clone();
    let completed_run = posture.run_id.clone(); let completed_lane = downstream.clone(); let completed_bundle = bundle_id.clone();
    let detached_reader = tokio::spawn(async move { completed_operation.run(completed_store.consume_context_bundle_for_downstream(
        &completed_run, &completed_bundle, &completed_lane)).await });
    let operation_id = tokio::time::timeout(std::time::Duration::from_secs(10), committed).await.unwrap().unwrap();
    assert!(!operation_id.is_nil()); detached_reader.abort(); assert!(detached_reader.await.unwrap_err().is_cancelled());
    resume.send(()).unwrap();
    let verified = independent_store();
    let mut final_run = tokio::time::timeout(std::time::Duration::from_secs(10),
        harness.workspace_read.run(verified.hold_context_bundle_fence_for_test(&posture.run_id, None)))
        .await.expect("observed committed ownership is released despite caller drop").unwrap();
    harness.workspace_read.run(async { Ok::<_, ModelLaneError>(final_run.release().await) })
        .await.expect("authorize committed run release").unwrap();
    assert_eq!(harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(&posture.run_id, &bundle_id)).await.unwrap(), replay);
    drop(final_run); drop(verified);
    // The real acquisition transaction has written its candidate but has not
    // settled. Detaching its caller must leave that transaction supervisor-owned.
    let (pending_acquisition, settle_acquisition) = completion.pause_acquisition_after(0).unwrap();
    let pending_store = harness.reader_store.clone(); let pending_operation = harness.workspace_read.clone();
    let pending_run = posture.run_id.clone(); let pending_lane = downstream.clone(); let pending_bundle = bundle_id.clone();
    let pending_reader = tokio::spawn(async move { pending_operation.run(pending_store.consume_context_bundle_for_downstream(
        &pending_run, &pending_bundle, &pending_lane)).await });
    let acquisition_id = tokio::time::timeout(std::time::Duration::from_secs(10), pending_acquisition).await.unwrap().unwrap();
    assert!(!acquisition_id.is_nil()); pending_reader.abort(); assert!(pending_reader.await.unwrap_err().is_cancelled());
    settle_acquisition.send(()).unwrap();
    let verified = independent_store();
    let mut settled_run = tokio::time::timeout(std::time::Duration::from_secs(10),
        harness.workspace_read.run(verified.hold_context_bundle_fence_for_test(&posture.run_id, None)))
        .await.expect("pending acquisition settles before its real acquired guard is released").unwrap();
    harness.workspace_read.run(async { Ok::<_, ModelLaneError>(settled_run.release().await) })
        .await.expect("authorize settled-acquisition run release").unwrap();
    drop(settled_run); drop(verified);
    // Run acquisition commits, then lane acquisition remains in its actual
    // controller callback until the unchanged publication deadline rolls it back.
    let (pending_lane, resume_lane) = completion.pause_acquisition_after(1).unwrap();
    let deadline_store = harness.reader_store.clone(); let deadline_operation = harness.workspace_read.clone();
    let deadline_run = posture.run_id.clone(); let deadline_lane = downstream.clone(); let deadline_bundle = bundle_id.clone();
    let deadline_reader = tokio::spawn(async move {
        handshake_core::storage::surreal::SurrealStorage::with_operation_deadline(
            std::time::Instant::now() + std::time::Duration::from_secs(2),
            deadline_operation.run(deadline_store.consume_context_bundle_for_downstream(
                &deadline_run, &deadline_bundle, &deadline_lane))).await
    });
    let acquisition_id = tokio::time::timeout(std::time::Duration::from_secs(10), pending_lane).await.unwrap().unwrap();
    assert!(!acquisition_id.is_nil());
    let denied = tokio::time::timeout(std::time::Duration::from_secs(10), deadline_reader).await.unwrap().unwrap().unwrap_err();
    assert!(denied.to_string().contains("timeout") || denied.to_string().contains("timed out"), "actual deadline rollback: {denied}");
    drop(resume_lane);
    let verified = independent_store();
    let mut released_run = harness.workspace_read.run(verified.hold_context_bundle_fence_for_test(&posture.run_id, None))
        .await.expect("observed deadline rollback releases committed run using separate finalization budget");
    harness.workspace_read.run(async { Ok::<_, ModelLaneError>(released_run.release().await) })
        .await.expect("authorize deadline-rollback run release").unwrap();
    let mut released_lane = harness.workspace_read.run(verified.hold_context_bundle_fence_for_test(&posture.run_id, Some(&downstream)))
        .await.expect("rolled-back lane acquisition leaves no held generation");
    harness.workspace_read.run(async { Ok::<_, ModelLaneError>(released_lane.release().await) })
        .await.expect("authorize deadline-rollback lane release").unwrap();
    assert_eq!(harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(&posture.run_id, &bundle_id)).await.unwrap(), replay);
    drop(released_run); drop(released_lane); drop(verified); drop(completion);
    drop(run_holder); drop(lane_holder); drop(run_store); drop(lane_store);
    harness.cleanup().await;
}

#[tokio::test]
async fn context_bundle_derivation_rejects_run_lane_and_non_crdt_message_ledger_drift() {
    let harness = Harness::create("handoff-fabricated").await;
    let posture = harness.posture("handoff-fabricated").await;
    let source = harness.workspace_create.run(harness.store.record_message(posture.message.clone()))
        .await
        .expect("record source message");
    let downstream = record_downstream_lane(&harness, &posture).await;
    harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(artifact_binding(&posture, &source)))
        .await
        .expect("record exact artifact binding");

    let mut drifted = handoff(harness.scope.workspace.as_ref().expect("canonical workspace").as_str(), 
        &posture,
        &source,
        &downstream,
        "drifted-source",
        ModelLaneHandoffSelectionState::Selected,
        None,
    );
    drifted.source_lane_id.push_str("-foreign");
    drifted.context_bundle_id =
        model_lane_context_bundle_id_for_handoff(&drifted).expect("derive drifted id");
    let rejected_id = drifted.handoff_id.clone();
    context_bundle_assertions::derivation_assert!(condition, harness.workspace_create.run(harness.store.record_context_bundle_handoff(drifted))
        .await
        .is_err());
    assert_no_context_derivative(&harness, &rejected_id).await;

    let advisory = record_advisory_context_source(&harness, &posture, &posture.lane_id, ModelLaneMessageKind::Proposal, 100).await;
    let retry_binding = artifact_binding(&posture, &advisory);
    let retry_artifact = harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(retry_binding.clone())).await.unwrap();
    {
        use handshake_core::storage::surreal::{RowFilter, TestFieldMutation, TestMutationValue};
        let inspector = harness.storage.test_inspector();
        let table = inspector.table_selector("kernel_event_ledger").await.unwrap();
        let payload_field = table.field("payload").unwrap();
        let hash_field = table.field("payload_hash").unwrap();
        let rows = inspector.project(&table, &[payload_field.clone(), hash_field.clone()],
            RowFilter::IdEquals(retry_artifact.event_ledger_event_id.clone())).await.unwrap();
        context_bundle_assertions::derivation_assert!(equal, rows.len(), 1);
        let original = rows[0].values.clone();
        let mut altered = original["payload"].clone();
        let mut payload: serde_json::Value = serde_json::from_str(altered["event_payload_json"].as_str().unwrap()).unwrap();
        payload["actor_principal_id"] = json!(uuid::Uuid::now_v7().to_string());
        altered["event_payload_json"] = json!(serde_json::to_string(&payload).unwrap());
        let correct_hash = hex::encode(Sha256::digest(altered["event_payload_json"].as_str().unwrap().as_bytes()));
        let mutator = harness.storage.test_mutator();
        mutator.update_row(&table, retry_artifact.event_ledger_event_id.clone(), &[
            TestFieldMutation::new(payload_field.clone(), TestMutationValue::json(altered)),
            TestFieldMutation::new(hash_field.clone(), TestMutationValue::string(correct_hash)),
        ]).await.unwrap();
        let denied = harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(retry_binding.clone())).await;
        mutator.update_row(&table, retry_artifact.event_ledger_event_id.clone(), &[
            TestFieldMutation::new(payload_field.clone(), TestMutationValue::json(original["payload"].clone())),
            TestFieldMutation::new(hash_field.clone(), TestMutationValue::json(original["payload_hash"].clone())),
        ]).await.unwrap();
        context_bundle_assertions::derivation_assert!(condition, denied.is_err(), "identical artifact retry must reconcile immutable receipt scope before returning canonical success");
        context_bundle_assertions::derivation_assert!(equal, inspector.project(&table, &[payload_field, hash_field],
            RowFilter::IdEquals(retry_artifact.event_ledger_event_id.clone())).await.unwrap()[0].values, original);
        context_bundle_assertions::derivation_assert!(equal, harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(retry_binding)).await.unwrap(), retry_artifact);
    }
    assert_source_scope_counterfactual(&harness, &posture, &advisory, &downstream, "run", &posture.run_id, "actor_principal_id").await;
    assert_source_scope_counterfactual(&harness, &posture, &advisory, &downstream, "lane", &posture.lane_id, "actor_principal_id").await;
    assert_mixed_source_denial(&harness, &posture, &advisory, &downstream, "actor_principal_id").await;
    for (kind, identity) in [("run", posture.run_id.as_str()), ("lane", posture.lane_id.as_str()),
        ("message", advisory.message_id.as_str())] {
        assert_source_receipt_scope_counterfactual(&harness, &posture, &advisory, &downstream, kind, identity).await;
    }
    assert_forged_run_scope_cannot_authorize_artifact(&harness, &posture, &advisory).await;
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_context_bundle_rejects_fabricated_crdt_authority_before_persistence() {
    let harness = Harness::create("fabricated-crdt-authority").await;
    let posture = harness.posture("fabricated-crdt-authority").await;
    let source = harness.workspace_create.run(harness.store.record_message(posture.message.clone())).await.unwrap();
    let downstream = record_downstream_lane(&harness, &posture).await;
    harness.workspace_create.run(harness.store.record_context_bundle_artifact_binding(artifact_binding(&posture, &source))).await.unwrap();
    let mut fabricated_crdt = crdt_metadata(&posture, &source);
    fabricated_crdt.update_bytes_ref.push_str("-fabricated");
    let fabricated = handoff(harness.scope.workspace.as_ref().unwrap().as_str(), &posture, &source,
        &downstream, "fabricated-crdt", ModelLaneHandoffSelectionState::Selected, Some(fabricated_crdt));
    let rejected_id = fabricated.handoff_id.clone(); let derived_id = fabricated.context_bundle_id.clone();
    context_bundle_assertions::fabricated_assert!(condition, harness.workspace_create.run(harness.store.record_context_bundle_handoff(fabricated)).await.is_err());
    assert_no_context_derivative(&harness, &rejected_id).await;
    context_bundle_assertions::fabricated_assert!(condition, harness.workspace_read.run(harness.reader_store.replay_context_bundle_handoffs(
        &posture.run_id, &derived_id)).await.unwrap().is_empty());
    use handshake_core::storage::surreal::{RowFilter, ScalarValue};
    let inspector = harness.storage.test_inspector(); let table = inspector.table_selector("model_lane_authority").await.unwrap();
    let rows = inspector.project(&table, &[table.field("record_json").unwrap()], RowFilter::FieldEquals {
        field: table.field("record_kind").unwrap(), value: ScalarValue::String("context_handoff".into()),
    }).await.unwrap();
    context_bundle_assertions::fabricated_assert!(condition, rows.iter().all(|row| serde_json::from_str::<serde_json::Value>(row.values["record_json"].as_str().unwrap())
        .unwrap().get("context_bundle_id") != Some(&json!(derived_id))), "the actual derived bundle identity remains absent");
    harness.cleanup().await;
}

async fn record_cloud_source_lane(harness: &Harness, posture: &SurrealAdmissibleCrdtPosture) -> String {
    use handshake_core::swarm_orchestration::model_lane::{ModelLaneKind, RuntimeBinding, LaunchAuthority, ModelLaneProviderKind};
    let replay = harness.workspace_read.run(harness.reader_store.replay_run(&posture.run_id))
        .await.expect("canonical source lane");
    let mut lane = replay.lanes.iter().find(|lane| lane.lane_id == posture.lane_id)
        .expect("exact original local lane").inner.clone();
    lane.lane_id = format!("{}-cloud", posture.lane_id);
    lane.lane_span_id = format!("span-{}", lane.lane_id);
    lane.kind = ModelLaneKind::CloudModel;
    lane.runtime_binding = RuntimeBinding::Cloud;
    lane.launch_authority = LaunchAuthority::CloudLane;
    lane.provider_kind = ModelLaneProviderKind::OpenAi;
    lane.role = "cloud-reviewer".into();
    lane.backend = "openai-byok".into();
    lane.adapter_id = "provider://openai/byok".into();
    lane.session_id = format!("session-{}", lane.lane_id);
    lane.model_session_id = format!("model-session-{}", lane.lane_id);
    lane.model_id = Some(format!("model://mt005/{}", lane.lane_id));
    lane.projection_plan_ref = Some(format!("projection-plan://{}", lane.lane_id));
    lane.consent_receipt_ref = Some(format!("consent://{}", lane.lane_id));
    if let Some(locus) = lane.locus_binding.as_mut() {
        locus.session_id = lane.session_id.clone();
        locus.model_session_id = lane.model_session_id.clone();
        locus.locus_binding_ref = format!("locus://wp1/mt005/{}", lane.lane_id);
    }
    let (projection, consent) = harness.workspace_create.run(async { Ok::<_, ModelLaneError>(model_lane_cloud_support::seed_cloud_lane_authority(&harness.store,
        model_lane_cloud_support::CloudLaneAuthoritySpec {
            run_id: &lane.run_id, lane_id: &lane.lane_id, model_session_id: &lane.model_session_id,
            provider_kind: lane.provider_kind.as_str(), requested_model_id: lane.model_id.as_deref().unwrap(),
            projection_plan_id: lane.projection_plan_ref.as_deref().unwrap(),
            consent_receipt_id: lane.consent_receipt_ref.as_deref().unwrap(),
            event_ledger_stream_id: &lane.event_ledger_stream_id,
            work_packet_id: lane.work_packet_id.as_deref().unwrap(), micro_task_id: "MT-005",
            task_board_id: lane.task_board_id.as_deref().unwrap(), owner_session: &lane.owner_session,
        }).await) }).await.expect("authorize canonical cloud-lane authority seed");
    assert_cloud_canonical_receipt(harness, "projection_plan", &projection.projection_plan_id,
        &projection.event_ledger_event_id, projection.event_ledger_seq, &serde_json::to_value(&projection).unwrap()).await;
    assert_cloud_canonical_receipt(harness, "consent_receipt", &consent.consent_receipt_id,
        &consent.event_ledger_event_id, consent.event_ledger_seq, &serde_json::to_value(&consent).unwrap()).await;
    let cloud_authority = harness.storage.test_inspector().table_selector("model_lane_cloud_authority").await.unwrap();
    let cloud_ledger = harness.storage.test_inspector().table_selector("model_lane_cloud_event_ledger").await.unwrap();
    let before_authority = harness.storage.test_inspector().row_count(&cloud_authority, handshake_core::storage::surreal::RowFilter::All).await.unwrap();
    let before_ledger = harness.storage.test_inspector().row_count(&cloud_ledger, handshake_core::storage::surreal::RowFilter::All).await.unwrap();
    for case in [
        handshake_core::swarm_orchestration::model_lane::CloudAdmissionCounterfactual::CallerSequence,
        handshake_core::swarm_orchestration::model_lane::CloudAdmissionCounterfactual::CallerFinalMetadata,
        handshake_core::swarm_orchestration::model_lane::CloudAdmissionCounterfactual::FalseTargetBindingsHash,
    ] {
        let error = harness.workspace_create.run(harness.store.test_cloud_admission_counterfactual(projection.clone(), case)).await
            .expect_err("authenticated held-token raw cloud counterfactual must be rejected");
        let marker = match case {
            handshake_core::swarm_orchestration::model_lane::CloudAdmissionCounterfactual::CallerSequence => "HSK-CLOUD-CALLER-SEQUENCE-DENIED",
            handshake_core::swarm_orchestration::model_lane::CloudAdmissionCounterfactual::CallerFinalMetadata => "HSK-CLOUD-CALLER-METADATA-DENIED",
            handshake_core::swarm_orchestration::model_lane::CloudAdmissionCounterfactual::FalseTargetBindingsHash => "HSK-CLOUD-PROBE-AUTHORITY-DENIED",
        };
        assert!(error.to_string().contains(marker), "counterfactual {case:?} reached its exact database admission guard: {error}");
        assert_eq!(harness.storage.test_inspector().row_count(&cloud_authority, handshake_core::storage::surreal::RowFilter::All).await.unwrap(), before_authority);
        assert_eq!(harness.storage.test_inspector().row_count(&cloud_ledger, handshake_core::storage::surreal::RowFilter::All).await.unwrap(), before_ledger,
            "failed cloud admission must roll back the allocated receipt too");
    }
    assert!(projection.event_ledger_seq < consent.event_ledger_seq,
        "canonical projection receipt is allocated before its approving consent receipt");
    let id = lane.lane_id.clone();
    harness.workspace_create.run(harness.store.record_lane(lane)).await.expect("real cloud lane authority");
    id
}

async fn assert_cloud_canonical_receipt(harness: &Harness, kind: &str, aggregate_id: &str,
    event_id: &str, sequence: i64, canonical_record: &serde_json::Value,
) {
    use handshake_core::storage::surreal::{RowFilter, ScalarValue};
    let inspector = harness.storage.test_inspector();
    let authority = inspector.table_selector("model_lane_cloud_authority").await.unwrap();
    let rows = inspector.project(&authority, &[
        authority.field("kind").unwrap(), authority.field("record_json").unwrap(),
        authority.field("event_id").unwrap(), authority.field("event_seq").unwrap(),
        authority.field("event_payload_json").unwrap(),
    ], RowFilter::FieldEquals { field: authority.field("aggregate_id").unwrap(),
        value: ScalarValue::String(aggregate_id.to_owned()) }).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].values["kind"], kind);
    assert_eq!(rows[0].values["event_id"], event_id);
    assert_eq!(rows[0].values["event_seq"], sequence);
    assert_eq!(serde_json::from_str::<serde_json::Value>(rows[0].values["record_json"].as_str().unwrap()).unwrap(),
        *canonical_record);
    let ledger = inspector.table_selector("model_lane_cloud_event_ledger").await.unwrap();
    let receipt = inspector.project(&ledger, &[
        ledger.field("event_id").unwrap(), ledger.field("aggregate_kind").unwrap(),
        ledger.field("aggregate_id").unwrap(), ledger.field("run_id").unwrap(),
        ledger.field("event_seq").unwrap(), ledger.field("event_ledger_stream_id").unwrap(),
        ledger.field("payload_json").unwrap(), ledger.field("payload_hash").unwrap(),
        ledger.field("owner_account_id").unwrap(), ledger.field("actor_principal_id").unwrap(),
        ledger.field("authenticated_session_id").unwrap(), ledger.field("access_space_id").unwrap(),
        ledger.field("workspace_id").unwrap(),
    ], RowFilter::IdEquals(event_id.to_owned())).await.unwrap();
    assert_eq!(receipt.len(), 1, "independently read the actual dedicated cloud receipt, not its copied JSON");
    let actual = &receipt[0].values;
    assert!(sequence > 0);
    assert_eq!(actual["event_id"], event_id); assert_eq!(actual["event_seq"], sequence);
    assert_eq!(actual["aggregate_kind"], kind); assert_eq!(actual["aggregate_id"], aggregate_id);
    assert_eq!(actual.get("run_id"), canonical_record.get("run_id"));
    assert_eq!(actual.get("event_ledger_stream_id"), canonical_record.get("event_ledger_stream_id"));
    let payload_text = actual["payload_json"].as_str().unwrap();
    assert_eq!(actual["payload_hash"], hex::encode(Sha256::digest(payload_text.as_bytes())));
    assert_eq!(rows[0].values["event_payload_json"], actual["payload_json"]);
    let payload: serde_json::Value = serde_json::from_str(payload_text).unwrap();
    assert_eq!(payload.get("record"), Some(canonical_record));
    assert_eq!(canonical_record.get("event_ledger_event_id"), Some(&json!(event_id)));
    assert_eq!(canonical_record.get("event_ledger_seq"), Some(&json!(sequence)));
    assert_eq!(canonical_record.get("event_stream_version"), Some(&json!(sequence)));
    assert_eq!(canonical_record.get("transaction_seq"), Some(&json!(sequence)));
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&harness.scope).unwrap();
    for (field, expected) in [
        ("owner_account_id", exact.owner_account_id.to_string()),
        ("actor_principal_id", exact.actor_principal_id.to_string()),
        ("authenticated_session_id", exact.authenticated_session_id.to_string()),
        ("access_space_id", exact.access_space_id.to_string()), ("workspace_id", exact.workspace_id.to_string()),
    ] { assert_eq!(actual.get(field), Some(&json!(expected)), "actual cloud receipt exact scope {field}"); }
}

async fn record_advisory_context_source(harness: &Harness, posture: &SurrealAdmissibleCrdtPosture,
    lane_id: &str, kind: ModelLaneMessageKind, index: usize,
) -> handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord {
    use handshake_core::swarm_orchestration::model_lane::ModelLaneTarget;
    let replay = harness.workspace_read.run(harness.reader_store.replay_run(&posture.run_id)).await.unwrap();
    let lane = replay.lanes.iter().find(|lane| lane.lane_id == lane_id).expect("canonical source identity");
    let mut message = posture.message.clone();
    message.message_id = format!("context-source-{}-{index}", posture.run_id);
    message.idempotency_key = format!("idem-{}", message.message_id);
    message.replay_order_key = format!("0010-{index}");
    message.message_span_id = format!("span-{}", message.message_id);
    message.from_lane_id = lane_id.to_owned();
    message.locus_binding = lane.locus_binding.clone();
    message.to_lane = ModelLaneTarget::Coordinator;
    message.routing = None;
    message.kind = kind;
    message.authority = ModelLaneAuthority::Advisory;
    message.proposal_ref = None;
    message.crdt_update_ref = None;
    message.crdt_base_snapshot_ref = None;
    message.crdt_state_vector = None;
    message.crdt_proposal_ref = None;
    message.crdt_stale_base_ref = None;
    message.payload_ref = format!("artifact-store://mt005/{}/context.json", message.message_id);
    message.payload_sha256 = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(handshake_storage_support::canonical_json::canonical_json_bytes(&json!({
            "schema_id": "hsk.model_lane_context_payload@1", "message_id": message.message_id, "summary": message.summary,
        }))))
    };
    harness.workspace_create.run(harness.store.record_message(message)).await.expect("canonical advisory source")
}

async fn assert_no_context_derivative(harness: &Harness, aggregate_id: &str) {
    use handshake_core::storage::surreal::{RowFilter, ScalarValue};
    let inspector = harness.storage.test_inspector();
    for name in ["model_lane_authority", "kernel_event_ledger"] {
        let table = inspector.table_selector(name).await.unwrap();
        assert_eq!(inspector.row_count(&table, RowFilter::FieldEquals {
            field: table.field("aggregate_id").unwrap(), value: ScalarValue::String(aggregate_id.to_owned()),
        }).await.unwrap(), 0, "actual rejected identity {aggregate_id} leaves neither derivative nor receipt");
    }
}

async fn assert_handoff_denial(harness: &Harness, input: NewModelLaneContextBundleHandoff, diagnostic: &str) {
    let identity = input.handoff_id.clone();
    let denied = harness.workspace_create.run(harness.store.record_context_bundle_handoff(input)).await
        .expect_err("invalid handoff must be rejected");
    assert!(denied.to_string().contains(diagnostic), "expected {diagnostic}: {denied}");
    assert_no_context_derivative(harness, &identity).await;
}

async fn assert_final_context_receipt(harness: &Harness, kind: &str, aggregate_id: &str,
    event_id: &str, sequence: i64, canonical_record: &serde_json::Value,
) {
    use handshake_core::storage::surreal::RowFilter;
    let inspector = harness.storage.test_inspector();
    let table = inspector.table_selector("kernel_event_ledger").await.unwrap();
    let rows = inspector.project(&table, &[table.field("payload").unwrap(), table.field("event_sequence").unwrap(),
        table.field("aggregate_id").unwrap(), table.field("aggregate_type").unwrap(),
        table.field("session_run_id").unwrap(), table.field("kernel_task_run_id").unwrap(),
        table.field("owner_account_id").unwrap(), table.field("actor_principal_id").unwrap(),
        table.field("authenticated_session_id").unwrap(), table.field("access_space_id").unwrap(),
        table.field("workspace_id").unwrap(), table.field("source_component").unwrap(),
        table.field("event_type").unwrap(), table.field("payload_hash").unwrap()],
        RowFilter::IdEquals(event_id.to_owned())).await.unwrap();
    assert_eq!(rows.len(), 1, "one independently read canonical receipt");
    let row = &rows[0].values;
    assert_eq!(row.get("aggregate_id"), Some(&json!(aggregate_id)));
    assert_eq!(row.get("aggregate_type"), Some(&json!(kind)));
    assert_eq!(row.get("event_sequence"), Some(&json!(sequence)));
    assert_eq!(row.get("session_run_id"), canonical_record.get("event_ledger_stream_id"));
    let payload: serde_json::Value = serde_json::from_str(row["payload"]["event_payload_json"].as_str().unwrap()).unwrap();
    assert_eq!(row.get("kernel_task_run_id"), canonical_record.get("run_id"));
    assert_eq!(row.get("source_component"), Some(&json!("model_lane")));
    assert_eq!(row.get("event_type"), Some(&json!("MODEL_LANE_AUTHORITY_RECORDED")));
    let schema_id = match kind {
        "context_artifact" => "hsk.model_lane_context_bundle_artifact@1",
        "context_handoff" => "hsk.model_lane_context_bundle_handoff@1",
        other => panic!("not a context receipt kind: {other}"),
    };
    assert_eq!(payload.get("schema_id"), Some(&json!(schema_id)));
    assert_eq!(payload.get("dexterity_kernel"), Some(&json!("Dexterity")));
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&harness.scope).unwrap();
    for (field, expected) in [
        ("owner_account_id", exact.owner_account_id.to_string()),
        ("actor_principal_id", exact.actor_principal_id.to_string()),
        ("authenticated_session_id", exact.authenticated_session_id.to_string()),
        ("access_space_id", exact.access_space_id.to_string()),
        ("workspace_id", exact.workspace_id.to_string()),
    ] {
        assert_eq!(row.get(field), Some(&json!(expected)), "actual receipt scope {field}");
        assert_eq!(payload.get(field), row.get(field), "inner immutable payload scope {field}");
    }
    use sha2::{Digest, Sha256};
    assert_eq!(row.get("payload_hash"), Some(&json!(hex::encode(Sha256::digest(
        row["payload"]["event_payload_json"].as_str().unwrap().as_bytes())))));
    assert_eq!(payload["record"], *canonical_record, "final event ID and sequence are in the immutable receipt");
    assert_eq!(payload["record"]["event_ledger_event_id"], event_id);
    assert_eq!(payload["record"]["event_ledger_seq"], sequence);
}

async fn record_downstream_lane(
    harness: &Harness,
    posture: &SurrealAdmissibleCrdtPosture,
) -> String {
    let replay = harness.workspace_read.run(harness.reader_store.replay_run(&posture.run_id))
        .await
        .expect("read source lane for downstream derivation");
    let mut lane = replay.lanes[0].inner.clone();
    lane.lane_id = format!("{}-downstream", posture.lane_id);
    lane.role = "context-consumer".into();
    lane.lane_span_id = format!("{}-downstream", lane.lane_span_id);
    lane.session_id = format!("{}-downstream", lane.session_id);
    lane.model_session_id = format!("{}-downstream", lane.model_session_id);
    lane.adapter_id = "local-runtime-downstream".into();
    lane.process_ownership_ref = Some(format!("process://mt005/{}/downstream", posture.run_id));
    if let Some(locus) = lane.locus_binding.as_mut() {
        locus.session_id = lane.session_id.clone();
        locus.model_session_id = lane.model_session_id.clone();
        locus.locus_binding_ref = format!("locus://wp1/mt005/{}/downstream", posture.run_id);
    }
    let lane_id = lane.lane_id.clone();
    harness.workspace_create.run(harness.store.record_lane(lane))
        .await
        .expect("record downstream lane through production faÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¾Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¾ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Â¦Ãƒâ€šÃ‚Â¡ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Â¦Ãƒâ€šÃ‚Â¾ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¾Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Â¦Ãƒâ€šÃ‚Â¡ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â¦ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¡ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¦ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¡ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Â¦Ãƒâ€šÃ‚Â¡ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§ade");
    lane_id
}

fn artifact_binding(
    posture: &SurrealAdmissibleCrdtPosture,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> NewModelLaneContextBundleArtifactBinding {
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-{}", source.message_id),
        run_id: posture.run_id.clone(),
        trace_id: posture.trace_id.clone(),
        artifact_ref: source.payload_ref.clone(),
        artifact_sha256: source.payload_sha256.clone(),
        content_hash: source.payload_sha256.clone(),
        artifact_kind: "model_lane_message_payload".into(),
        artifact_manifest_ref: format!("artifact-manifest://{}", source.message_id),
        artifact_payload_ref: source.payload_ref.clone(),
        payload_json: json!({
            "schema_id": "hsk.model_lane_context_payload@1",
            "message_id": source.message_id,
            "summary": source.summary,
        }),
        event_ledger_stream_id: source.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-005".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: source.owner_session.clone(),
        idempotency_key: format!("artifact-binding-{}", source.message_id),
        created_at_utc: "2026-09-02T00:03:00Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    }
}

fn crdt_metadata(
    posture: &SurrealAdmissibleCrdtPosture,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> ModelLaneCrdtHandoffMetadata {
    let binding = source
        .crdt_authority_binding
        .as_ref()
        .expect("source has exact CRDT binding");
    ModelLaneCrdtHandoffMetadata {
        schema_id: "hsk.model_lane_crdt_payload@1".into(),
        document_id: binding.document_id.clone(),
        workspace_id: binding.workspace_id.clone(),
        actor_id: binding.actor_id.clone(),
        actor_kind: binding.actor_kind.clone(),
        lane_id: binding.lane_id.clone(),
        crdt_site_id: binding.crdt_site_id.clone(),
        update_seq: binding.update_seq,
        update_bytes_ref: binding.update_bytes_ref.clone(),
        update_sha256: posture.update.update_sha256.clone(),
        state_vector: binding.state_vector.clone(),
        base_snapshot_ref: binding.base_snapshot_ref.clone(),
        materialized_projection_hash: binding.materialized_projection_hash.clone(),
        replay_metadata: json!({
            "format": "yjs_update_v1",
            "yjs_compatible": true,
            "replay_order_key": posture.update.replay_order_key,
            "dependency_update_ids": posture.update.dependency_update_ids,
            "schema_version": posture.update.replay_schema_version,
        }),
        promotion_gate_ref: format!("promotion-gate://model-lane-message/{}", source.message_id),
        promotion_receipt_ref: None,
        validation_runner_ref: format!("eventledger://{}", posture.update.event_ledger_event_id),
        authority_effect: "advisory_only".into(),
    }
}

fn handoff(
    workspace_id: &str,
    posture: &SurrealAdmissibleCrdtPosture,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
    downstream_lane_id: &str,
    suffix: &str,
    selection_state: ModelLaneHandoffSelectionState,
    crdt_payload: Option<ModelLaneCrdtHandoffMetadata>,
) -> NewModelLaneContextBundleHandoff {
    let mut input = NewModelLaneContextBundleHandoff {
        handoff_id: format!("handoff-{}-{suffix}", source.message_id),
        context_bundle_id: "pending-derived-id".into(),
        run_id: posture.run_id.clone(),
        trace_id: posture.trace_id.clone(),
        handoff_span_id: format!("span-handoff-{}-{suffix}", source.message_id),
        parent_span_id: Some(source.message_span_id.clone()),
        linked_span_contexts: vec![source.trace_id.clone()],
        downstream_lane_id: downstream_lane_id.into(),
        source_lane_id: source.from_lane_id.clone(),
        source_message_id: source.message_id.clone(),
        artifact_ref: source.payload_ref.clone(),
        artifact_sha256: source.payload_sha256.clone(),
        content_hash: source.payload_sha256.clone(),
        source_kind: match source.kind {
            ModelLaneMessageKind::Proposal => ModelLaneHandoffSourceKind::Proposal,
            ModelLaneMessageKind::Critique => ModelLaneHandoffSourceKind::Critique,
            ModelLaneMessageKind::Status => ModelLaneHandoffSourceKind::Status,
            ModelLaneMessageKind::Recovery => ModelLaneHandoffSourceKind::Recovery,
            _ => panic!("unsupported ContextBundle fixture source kind"),
        },
        authority_state: ModelLaneAuthority::PromotionCandidate,
        selection_state,
        reason_code: format!("mt005_{suffix}"),
        decision_ref: Some(format!("decision://mt005/{suffix}")),
        reviewer_ref: Some(format!("reviewer://mt005/{suffix}")),
        replay_hint: "load typed handoff and exact artifact ref".into(),
        crdt_payload,
        loom_refs: vec![ModelLaneLoomHandoffRef {
            workspace_id: source
                .crdt_authority_binding
                .as_ref()
                .map(|binding| binding.workspace_id.clone())
                .unwrap_or_else(|| workspace_id.to_owned()),
            block_id: format!("loom-block-{}", source.message_id),
            source_block_id: Some(format!("loom-source-{}", source.message_id)),
            target_block_id: Some(format!("loom-target-{}", source.message_id)),
            artifact_ref: Some(source.payload_ref.clone()),
            content_hash: source.payload_sha256.clone(),
            version: "1".into(),
            event_ledger_evidence_ref: format!("eventledger://{}", source.event_ledger_event_id),
            flight_recorder_evidence_ref: format!("flight-recorder://{}", source.message_id),
        }],
        memory_pack_refs: vec![ModelLaneMemoryPackHandoffRef {
            memory_pack_ref: format!("memory-pack://mt005/{}", source.message_id),
            memory_pack_hash: "d".repeat(64),
            scope_tag: "exact_resource_scope".into(),
            review_status: "reviewed".into(),
            cloud_safe: false,
            classification: "local_only_context".into(),
            projection_ref: None,
            evidence_ref: format!("eventledger://{}", source.event_ledger_event_id),
        }],
        event_ledger_stream_id: source.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-005".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: source.owner_session.clone(),
        idempotency_key: format!("handoff-{}-{suffix}", source.message_id),
        replay_order_key: format!("0040-{suffix}"),
        created_at_utc: "2026-09-02T00:04:00Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    };
    input.context_bundle_id = model_lane_context_bundle_id_for_handoff(&input)
        .expect("derive canonical ContextBundle id");
    input
}

struct ContextBundleOnlyFactory;

#[async_trait::async_trait]
impl handshake_core::swarm_orchestration::ModelSessionFactory for ContextBundleOnlyFactory {
    async fn create(&self, _request: &handshake_core::swarm_orchestration::SpawnRequest)
        -> handshake_core::swarm_orchestration::SwarmResult<handshake_core::swarm_orchestration::LiveSession> {
        Err(handshake_core::swarm_orchestration::SwarmError::FactoryFailed(
            "MT-005 ContextBundle proof must not create a model session".into()))
    }
}

fn context_bundle_coordinator(store: ModelLaneStore) -> handshake_core::swarm_orchestration::SwarmCoordinator {
    use handshake_core::process_ledger::{LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink};
    use handshake_core::swarm_orchestration::{RecordingSwarmSink, RunBudget, SwarmConfig, SwarmCoordinator};
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig { capacity: 4096, ..LedgerBatcherConfig::default() }, Arc::new(NoopOverflowSink),
    ).expect("manual process ledger for the original no-session ContextBundle adapter proof");
    SwarmCoordinator::new_with_model_lane_store(SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(ContextBundleOnlyFactory), Arc::new(RecordingSwarmSink::new()), ledger, store)
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope {
        owner_account_id: OwnerAccountId::from_uuid(label_uuid(&(format!("account-{label}")))),
        actor_principal_id: ActorPrincipalId::from_uuid(label_uuid(&(format!("actor-{label}")))),
        authenticated_session: Some(AuthenticatedSessionRef::from_uuid(label_uuid(&(format!("session-{label}"))))),
        access_space: Some(AccessSpaceRef::from_uuid(label_uuid(&(format!("access-{label}"))))),
        workspace: Some(WorkspaceScopeRef::new(format!("workspace-{label}")).expect("workspace")),
    }
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<ResourceScope> {
    let mut owner = scope.clone();
    owner.owner_account_id = OwnerAccountId::from_uuid(label_uuid(&("account-foreign")));
    let mut actor = scope.clone();
    actor.actor_principal_id = ActorPrincipalId::from_uuid(label_uuid(&("actor-foreign")));
    let mut session = scope.clone();
    session.authenticated_session =
        Some(AuthenticatedSessionRef::from_uuid(label_uuid(&("session-foreign"))));
    let mut access = scope.clone();
    access.access_space = Some(AccessSpaceRef::from_uuid(label_uuid(&("access-foreign"))));
    let mut workspace = scope.clone();
    workspace.workspace =
        Some(WorkspaceScopeRef::new("workspace-foreign").expect("foreign workspace"));
    vec![owner, actor, session, access, workspace]
}

/// Deterministic identifier for a test label so the same label resolves to the
/// same exact scope across reopen phases of one proof.
fn label_uuid(label: &str) -> uuid::Uuid {
    let digest = <sha2::Sha256 as sha2::Digest>::digest(label.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    uuid::Uuid::from_bytes(bytes)
}

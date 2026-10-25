use handshake_core::preferences::{
    work_profile_preference_entry, PreferenceRecord, PreferenceScope,
    PREF_WORK_PROFILE_REGISTRY, PREF_WORK_PROFILE_SELECTION,
};
use serde_json::{json, Value};

fn registry() -> Value {
    json!({
        "schema_id":"hsk.work_profiles@1", "registry_id":"profiles", "folded_stub_ids":["WP-1-Work-Profiles-v1"],
        "profile_storage_ref":"work-profiles.registry", "selected_profile_id":"local",
        "profiles":[{"profile_id":"local", "profile_version":1, "profile_id_is_immutable":true,
            "display_name":"Local", "created_at_utc":"2026-10-23T00:00:00Z",
            "role_routes":[{"role_id":"coder", "model_ref":"model", "provider_ref":"local_runtime", "capability_profile_ref":"capabilities"}],
            "autonomy":{"max_auto_actions":1,"requires_operator_approval_for_promotion":true,"allow_parallel_agents":false,"allow_network":false}}],
        "profile_receipts":[], "action_requests":[],
        "product_authority_refs":["kernel.action_catalog","kernel.role_turn_isolation","flight_recorder.profile_events","kernel.workflow_transition_registry"],
        "folded_source_refs":["WP-1-Work-Profiles-v1"]
    })
}

#[test]
fn registry_accepts_domain_data_without_client_authority_claims() {
    let entry = work_profile_preference_entry(PREF_WORK_PROFILE_REGISTRY).unwrap();
    assert!(entry.validate(&registry()).is_ok());
    let mut forged = registry();
    forged["profile_receipts"] = json!([{"receipt_ref":"forged", "profile_id":"local", "profile_version":1,"action_request_id":"forged", "event_ref":"forged"}]);
    assert!(entry.validate(&forged).is_err());
    let mut forged = registry();
    forged["action_requests"] = json!([{"action_request_id":"forged", "action_id":"run", "role_id":"coder", "selected_profile_id":"local", "selected_route_model_ref":"model", "receipt_ref":"forged", "job_metadata_work_profile_id":"local"}]);
    assert!(entry.validate(&forged).is_err());
}

#[test]
fn registry_preserves_existing_profile_domain_checks() {
    let entry = work_profile_preference_entry(PREF_WORK_PROFILE_REGISTRY).unwrap();
    for invalid in ["profile_version", "profile_id_is_immutable", "provider_ref", "role_id", "promotion"] {
        let mut value = registry();
        match invalid {
            "profile_version" => value["profiles"][0]["profile_version"] = json!(0),
            "profile_id_is_immutable" => value["profiles"][0]["profile_id_is_immutable"] = json!(false),
            "provider_ref" => value["profiles"][0]["role_routes"][0]["provider_ref"] = json!("unknown"),
            "role_id" => value["profiles"][0]["role_routes"][0]["role_id"] = json!(""),
            _ => value["profiles"][0]["autonomy"]["requires_operator_approval_for_promotion"] = json!(false),
        }
        assert!(entry.validate(&value).is_err(), "{invalid}");
    }
}

#[test]
fn selection_requires_exact_positive_versions_and_rejects_extra_authority() {
    let entry = work_profile_preference_entry(PREF_WORK_PROFILE_SELECTION).unwrap();
    let valid = json!({"profile_id":"local","profile_version":1,"role_id":"coder","registry_revision":1});
    assert!(entry.validate(&valid).is_ok());
    for field in ["profile_version", "registry_revision"] {
        let mut value = valid.clone(); value[field] = json!(0);
        assert!(entry.validate(&value).is_err());
    }
    let mut forged = valid; forged["receipt_id"] = json!("forged");
    assert!(entry.validate(&forged).is_err());
}

#[test]
fn unresolved_defaults_are_not_configured_domain_values() {
    for id in [PREF_WORK_PROFILE_REGISTRY, PREF_WORK_PROFILE_SELECTION] {
        let entry = work_profile_preference_entry(id).unwrap();
        let record = PreferenceRecord::resolved_default(&entry, &PreferenceScope::workspace("workspace"));
        assert_eq!(record.revision, 0);
        assert_eq!(record.value, json!({}));
        assert!(entry.validate(&record.value).is_err());
    }
}


macro_rules! authenticated_store_contract_tests {
    () => {
        use crate::storage::surreal::{SurrealStorage, resource_authority::{RecordUserScope, ResourceAction, ResourceGrantSpec}};
        use crate::preferences::{LocalExecutionAuthorityRequest, PreferenceSource};
        use std::time::Duration;

        use sha2::Digest;
#[derive(Default)]
struct NoopRecorder;

#[async_trait::async_trait]
impl crate::flight_recorder::FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: crate::flight_recorder::FlightRecorderEvent) -> Result<(), crate::flight_recorder::RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, crate::flight_recorder::RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: crate::flight_recorder::EventFilter,
    ) -> Result<Vec<crate::flight_recorder::FlightRecorderEvent>, crate::flight_recorder::RecorderError> {
        Ok(Vec::new())
    }
}

#[async_trait::async_trait]
impl crate::diagnostics::DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diag: crate::diagnostics::Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }

    async fn list_problems(
        &self,
        _filter: crate::diagnostics::DiagFilter,
    ) -> Result<Vec<crate::diagnostics::ProblemGroup>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(
        &self,
        _id: uuid::Uuid,
    ) -> Result<crate::diagnostics::Diagnostic, handshake_core::storage::StorageError> {
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
    }

    async fn list_diagnostics(
        &self,
        _filter: crate::diagnostics::DiagFilter,
    ) -> Result<Vec<crate::diagnostics::Diagnostic>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
}

    pub(crate) struct WorkspaceBindingFixture {
        _lock: std::sync::MutexGuard<'static, ()>,
        _directory: tempfile::TempDir,
        previous: Option<std::ffi::OsString>,
        token: String,
    }
    impl WorkspaceBindingFixture {
        pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let lock = crate::api::stage::NATIVE_BINDING_ENV_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let directory = tempfile::Builder::new().prefix("mt196-http-binding-").tempdir_in(crate::storage::tests::test_store_root()?)?;
            let path = directory.path().join("workspace-binding.json");
            let token = "c7".repeat(32);
            std::fs::write(
                &path,
                serde_json::to_vec(&crate::api::stage::current_process_native_binding(&token))?,
            )?;
            let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
            Ok(Self {
                _lock: lock,
                _directory: directory,
                previous,
                token,
            })
        }
    }
    impl Drop for WorkspaceBindingFixture {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous);
            } else {
                std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE");
            }
        }
    }
    pub(crate) async fn workspace_test_principal(
        state: &crate::AppState,
        binding: &WorkspaceBindingFixture,
        key: &str,
    ) -> Result<
        (
            crate::storage::surreal::resource_authority::ProvisionedPrincipal,
            axum::http::HeaderMap,
        ),
        Box<dyn std::error::Error>,
    > {
        if !state
            .surreal
            .reconciliation_principal_is_provisioned()
            .await?
        {
            state
                .surreal
                .provision_reconciliation_principal(&[], None)
                .await?;
        }
        let capabilities = [
            "fs.read",
            "fs.write",
            "fr.read",
            "fr.ingest.runtime_chat",
            "fr.ingest.native_editor",
            "memory.read",
            "memory.propose",
        ]
        .map(str::to_owned)
        .to_vec();
        let principal = state
            .surreal
            .provision_principal(
                key,
                key,
                "human_account",
                key,
                "Operator",
                &capabilities,
                key,
                Some(&hex::encode(sha2::Sha256::digest(binding.token.as_bytes()))),
                std::time::Duration::from_secs(3600),
            )
            .await?;
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-hsk-session-token", principal.session.token.parse()?);
        headers.insert("x-hsk-channel-binding-token", binding.token.parse()?);
        Ok((principal, headers))
    }
    async fn create_owned_test_workspace(state: &crate::AppState, headers: &axum::http::HeaderMap) -> Result<TestWorkspace, String> {
        let (status, response) = preference_http(state, headers, "POST", "/workspaces", json!({"name":"mt196-http-workspace"})).await;
        if status != axum::http::StatusCode::CREATED { return Err(format!("{status}: {response}")); }
        let id = response["id"].as_str().ok_or_else(|| format!("workspace id missing: {response}"))?.to_owned();
        Ok(TestWorkspace { id })
    }
    struct TestWorkspace { id: String }

        async fn authenticated_body(storage: &SurrealStorage) {
            let capabilities = vec!["fs.read".to_owned(), "fs.write".to_owned()];
            let principal = storage.provision_principal("mt196-account", "mt196-principal", "human_account", "mt196-actor", "Operator",
                &capabilities, "mt196-space", Some("mt196-binding"), Duration::from_secs(3600)).await.unwrap();
            let workspace = uuid::Uuid::now_v7().to_string();
            let resource = storage.register_workspace_resource(&principal.identity, &workspace).await.unwrap();
            let grant = storage.grant_resource(&principal.identity.account_id, &principal.identity.access_space_id, ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(), resource_id: resource.resource_id.clone(),
                actions: vec![ResourceAction::Read, ResourceAction::Create, ResourceAction::Update], capability_ids: capabilities,
                expires_at: Some(principal.session.expires_at), delegation_chain: vec![],
            }).await.unwrap();
            let authenticated = RecordUserScope { grant_id: Some(grant.grant_id), workspace_id: Some(workspace.clone()),
                session_token: principal.session.token.clone(), channel_binding_hash: Some("mt196-binding".into()),
                resource_id: resource.resource_id, session_id: principal.session.session_id.clone(), capability_id: "fs.write".into(), action: ResourceAction::Update };
            crate::storage::surreal::event_ledger::with_loom_session_receipt(crate::kernel::KernelActor::Operator("mt196-actor".into()), workspace.clone(), async {
            let scope = PreferenceScope::workspace(&workspace);
            let registry_entry = work_profile_preference_entry(PREF_WORK_PROFILE_REGISTRY).unwrap();
            let selection_entry = work_profile_preference_entry(PREF_WORK_PROFILE_SELECTION).unwrap();
            assert!(storage.with_record_user_scope(authenticated.clone(), storage.preference_set(&scope, &registry_entry, registry(), PreferenceSource::Operator, "mt196-actor")).await.is_err());
            assert!(storage.with_record_user_scope(authenticated.clone(), storage.preference_reset(&scope, &registry_entry, "mt196-actor")).await.is_err());
            assert!(storage.with_record_user_scope(authenticated.clone(), storage.preference_set_expected(&scope, &registry_entry, registry(), PreferenceSource::Operator, "forged-actor", Some(0))).await.is_err());
            storage.with_record_user_scope(authenticated.clone(), storage.preference_set_expected(&scope, &registry_entry, registry(), PreferenceSource::Operator, "mt196-actor", Some(0))).await.unwrap();
            let selection = json!({"profile_id":"local","profile_version":1,"role_id":"coder","registry_revision":1});
            storage.with_record_user_scope(authenticated.clone(), storage.preference_set_expected(&scope, &selection_entry, selection.clone(), PreferenceSource::Operator, "mt196-actor", Some(0))).await.unwrap();
            let request = LocalExecutionAuthorityRequest { workspace_id: workspace.clone(), expected_registry_revision:1, expected_selection_revision:1 };
            let lease = storage.with_record_user_scope(authenticated.clone(), storage.local_execution_authority_snapshot(&request)).await.unwrap();
            assert_eq!(lease.snapshot().actor_principal_id, principal.identity.principal_id);
            assert_eq!(lease.snapshot().actor_id, "mt196-actor");
            assert_eq!(lease.snapshot().registry_receipt.new_value, registry());
            let fresh_session = storage.issue_authenticated_session(&principal.identity.account_id, &principal.identity.principal_id,
                &principal.identity.access_space_id, Some("mt196-binding"), Duration::from_secs(3600)).await.unwrap();
            let mut reauthenticated = authenticated.clone();
            reauthenticated.session_id = fresh_session.session_id;
            reauthenticated.session_token = fresh_session.token;
            let before_registry = lease.snapshot().registry_record.clone();
            let before_selection = lease.snapshot().selection_record.clone();
            let before_registry_receipt = lease.snapshot().registry_receipt.clone();
            let before_selection_receipt = lease.snapshot().selection_receipt.clone();
            drop(lease);
            let fresh = storage.with_record_user_scope(reauthenticated, storage.local_execution_authority_snapshot(&request)).await.unwrap();
            assert_eq!(serde_json::to_value(&fresh.snapshot().registry_record).unwrap(), serde_json::to_value(&before_registry).unwrap());
            assert_eq!(serde_json::to_value(&fresh.snapshot().selection_record).unwrap(), serde_json::to_value(&before_selection).unwrap());
            assert_eq!(fresh.snapshot().registry_receipt.receipt_id, before_registry_receipt.receipt_id);
            assert_eq!(fresh.snapshot().selection_receipt.receipt_id, before_selection_receipt.receipt_id);
            drop(fresh);
            let lease = storage.with_record_user_scope(authenticated.clone(), storage.local_execution_authority_snapshot(&request)).await.unwrap();
            for entry in [&registry_entry, &selection_entry] {
                assert!(tokio::time::timeout(Duration::from_millis(20), storage.with_record_user_scope(authenticated.clone(),
                    storage.preference_set_expected(&scope, entry, if entry.preference_id == PREF_WORK_PROFILE_REGISTRY { registry() } else { selection.clone() },
                        PreferenceSource::Operator, "mt196-actor", Some(1)))).await.is_err());
                assert!(tokio::time::timeout(Duration::from_millis(20), storage.with_record_user_scope(authenticated.clone(),
                    storage.preference_reset_expected(&scope, entry, "mt196-actor", Some(1)))).await.is_err());
            }
            let clone = storage.clone();
            let mutation = clone.with_record_user_scope(authenticated.clone(), clone.preference_set_expected(&scope, &registry_entry, registry(), PreferenceSource::Operator, "mt196-actor", Some(1)));
            tokio::pin!(mutation);
            assert!(tokio::time::timeout(Duration::from_millis(30), &mut mutation).await.is_err(), "canonical clone must share held snapshot gate");
            // Cancelling a queued acquisition must not keep a permit alive after release.
            assert!(tokio::time::timeout(Duration::from_millis(30), storage.with_record_user_scope(authenticated.clone(), storage.local_execution_authority_snapshot(&request))).await.is_err());
            let immutable = lease.release();
            assert_eq!(immutable.registry_record.revision, 1);
            // An indefinitely retained native boundary is represented only by this uncompleted future.
            let (_native_release, native_blocked) = tokio::sync::oneshot::channel::<()>();
            tokio::pin!(native_blocked);
            let changed = tokio::time::timeout(Duration::from_secs(2), &mut mutation).await.unwrap().unwrap();
            assert_eq!(changed.0.revision, 2, "write completes while simulated native boundary remains blocked");
            assert!(tokio::time::timeout(Duration::from_millis(1), &mut native_blocked).await.is_err());
            assert!(storage.with_record_user_scope(authenticated.clone(), storage.local_execution_authority_snapshot(&request)).await.is_err(), "mutation-first makes captured revisions stale");
            assert!(storage.with_record_user_scope(authenticated.clone(), storage.preference_set_expected(&scope, &registry_entry, registry(), PreferenceSource::Operator, "mt196-actor", Some(1))).await.is_err(), "CAS must not rebase");
            let after = storage.with_record_user_scope(authenticated.clone(), storage.preference_get(&scope, &registry_entry)).await.unwrap();
            assert_eq!(after.revision,2);
            let mut selected = selection; selected["registry_revision"] = json!(2);
            storage.with_record_user_scope(authenticated.clone(), storage.preference_set_expected(&scope, &selection_entry, selected, PreferenceSource::Operator, "mt196-actor", Some(1))).await.unwrap();
            let current = LocalExecutionAuthorityRequest { workspace_id:workspace.clone(), expected_registry_revision:2, expected_selection_revision:2 };
            let lease = storage.with_record_user_scope(authenticated.clone(), storage.local_execution_authority_snapshot(&current)).await.unwrap();
            let event_id = lease.snapshot().selection_record.event_ledger_event_id.clone();
            drop(lease);
            let mut wrong_scope = authenticated.clone(); wrong_scope.workspace_id = Some(uuid::Uuid::now_v7().to_string());
            assert!(storage.with_record_user_scope(wrong_scope, storage.local_execution_authority_snapshot(&current)).await.is_err());
            storage.with_data_operation(move |database| Box::pin(async move {
                database.client.query("UPDATE $event SET payload.receipt_id = 'forged-receipt';")
                    .bind(("event", surrealdb::types::RecordId::new("kernel_event_ledger", event_id))).await?.check()?;
                Ok(())
            })).await.unwrap();
            assert!(storage.with_record_user_scope(authenticated, storage.local_execution_authority_snapshot(&current)).await.is_err(), "forged persisted receipt/event mapping cannot authorize");
            }).await;
        }

        #[cfg(feature = "test-utils")]
        async fn submitted_write_settlement_body(storage: &SurrealStorage) {
            let capabilities = vec!["fs.read".to_owned(), "fs.write".to_owned()];
            let principal = storage.provision_principal("mt196-settlement-account", "mt196-settlement-principal", "human_account", "mt196-settlement-actor", "Operator",
                &capabilities, "mt196-settlement-space", Some("mt196-binding"), Duration::from_secs(3600)).await.unwrap();
            let workspace = uuid::Uuid::now_v7().to_string();
            let resource = storage.register_workspace_resource(&principal.identity, &workspace).await.unwrap();
            let grant = storage.grant_resource(&principal.identity.account_id, &principal.identity.access_space_id, ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(), resource_id: resource.resource_id.clone(),
                actions: vec![ResourceAction::Read, ResourceAction::Create, ResourceAction::Update], capability_ids: capabilities,
                expires_at: Some(principal.session.expires_at), delegation_chain: vec![],
            }).await.unwrap();
            let authenticated = RecordUserScope { grant_id: Some(grant.grant_id), workspace_id: Some(workspace.clone()),
                session_token: principal.session.token.clone(), channel_binding_hash: Some("mt196-binding".into()),
                resource_id: resource.resource_id, session_id: principal.session.session_id.clone(), capability_id: "fs.write".into(), action: ResourceAction::Update };
            crate::storage::surreal::event_ledger::with_loom_session_receipt(crate::kernel::KernelActor::Operator("mt196-settlement-actor".into()), workspace.clone(), async {
                let scope = PreferenceScope::workspace(&workspace);
                let registry_entry = work_profile_preference_entry(PREF_WORK_PROFILE_REGISTRY).unwrap();
                let selection_entry = work_profile_preference_entry(PREF_WORK_PROFILE_SELECTION).unwrap();
                storage.with_record_user_scope(authenticated.clone(), storage.preference_set_expected(&scope, &registry_entry,
                    registry(), PreferenceSource::Operator, "mt196-settlement-actor", Some(0))).await.unwrap();
                let mut revision = 1;
                let mut selection_revision = 0;
                for rollback in [false, true] {
                    for caller_drop in [false, true] {
                        let selected = json!({"profile_id":"local","profile_version":1,"role_id":"coder","registry_revision":revision});
                        storage.with_record_user_scope(authenticated.clone(), storage.preference_set_expected(&scope, &selection_entry,
                            selected.clone(), PreferenceSource::Operator, "mt196-settlement-actor", Some(selection_revision))).await.unwrap();
                        selection_revision += 1;
                        let request = LocalExecutionAuthorityRequest { workspace_id:workspace.clone(), expected_registry_revision:revision, expected_selection_revision:selection_revision };
                        let proof = super::WorkProfileWriteProof::new(rollback);
                        let operation = super::WORK_PROFILE_WRITE_PROOF.scope(proof.clone(), storage.with_record_user_scope(authenticated.clone(),
                            storage.preference_set_expected(&scope, &registry_entry, registry(), PreferenceSource::Operator, "mt196-settlement-actor", Some(revision))));
                        let deadline = std::time::Instant::now() + Duration::from_millis(80);
                        let mut writer = Box::pin(SurrealStorage::with_operation_deadline(deadline, operation));
                        tokio::select! {
                            biased;
                            _ = proof.dispatched.notified() => {},
                            result = &mut writer => panic!("write completed before real SDK query pending: {result:?}"),
                            _ = tokio::time::sleep(Duration::from_secs(2)) => panic!("SDK query did not dispatch"),
                        }
                        if caller_drop { drop(writer); }
                        else { assert!(writer.await.is_err(), "caller deadline must expire during original SDK query"); }
                        let competing_snapshot = storage.with_record_user_scope(authenticated.clone(), storage.local_execution_authority_snapshot(&request));
                        let competing_writer = storage.with_record_user_scope(authenticated.clone(), storage.preference_reset_expected(&scope, &selection_entry, "mt196-settlement-actor", Some(selection_revision)));
                        assert!(tokio::time::timeout(Duration::from_millis(25), competing_snapshot).await.is_err());
                        assert!(tokio::time::timeout(Duration::from_millis(25), competing_writer).await.is_err());
                        tokio::time::timeout(Duration::from_secs(2), proof.responded.notified()).await.expect("original SDK response must remain owned");
                        // Still fenced after the engine response until its owner consumes actual settlement.
                        assert!(tokio::time::timeout(Duration::from_millis(25), storage.authority_publication_read()).await.is_err());
                        proof.release.add_permits(1);
                        let publication = tokio::time::timeout(Duration::from_secs(2), storage.authority_publication_read()).await.unwrap().unwrap();
                        drop(publication);
                        if !rollback { revision += 1; }
                        let record = storage.with_record_user_scope(authenticated.clone(), storage.preference_get(&scope, &registry_entry)).await.unwrap();
                        assert_eq!(record.revision, revision);
                        let selected_record = storage.with_record_user_scope(authenticated.clone(), storage.preference_get(&scope, &selection_entry)).await.unwrap();
                        assert_eq!(selected_record.revision, selection_revision, "cancelled queued reset must not mutate");
                        assert!(storage.with_record_user_scope(authenticated.clone(), storage.preference_set_expected(&scope, &registry_entry,
                            registry(), PreferenceSource::Operator, "mt196-settlement-actor", Some(revision - 1))).await.is_err(), "CAS must not rebase after settlement");
                    }
                }
            }).await;
        }

        #[cfg(feature = "test-utils")]
        pub async fn submitted_write_settlement_contract() {
            let backend = crate::storage::tests::embedded_test_backend().await.unwrap();
            let storage = backend.storage.clone();
            let outcome = tokio::spawn(async move { submitted_write_settlement_body(&storage).await }).await;
            backend.close_and_remove().await.unwrap();
            if let Err(panic) = outcome { std::panic::resume_unwind(panic.into_panic()); }
        }

        async fn preference_http(state: &crate::AppState, headers: &axum::http::HeaderMap, method: &str, uri: &str, body: Value)
            -> (axum::http::StatusCode, Value) {
            async fn invoke<S>(mut service: S, request: axum::http::Request<axum::body::Body>) -> Result<S::Response, S::Error>
            where S: axum::ServiceExt<axum::http::Request<axum::body::Body>> {
                std::future::poll_fn(|cx| service.poll_ready(cx)).await?;
                service.call(request).await
            }
            let mut request = axum::http::Request::builder().method(method).uri(uri).header("content-type", "application/json")
                .header("x-hsk-actor-id", "forged-header-actor");
            for (name, value) in headers { request = request.header(name, value); }
            let response = invoke(crate::api::routes(state.clone()), request.body(axum::body::Body::from(serde_json::to_vec(&body).unwrap())).unwrap()).await.unwrap();
            let status = response.status();
            let bytes = axum::body::to_bytes(response.into_body(), 1024*1024).await.unwrap();
            (status, serde_json::from_slice(&bytes).unwrap())
        }

        pub async fn mounted_preferences_http_cas_receipts_and_reset() {
            use std::sync::Arc;
            use axum::http::StatusCode;
            let backend = crate::storage::tests::embedded_test_backend().await.unwrap();
            let recorder = Arc::new(NoopRecorder);
            let state = crate::AppState {
                storage: backend.database.clone(), surreal: backend.storage.clone(), flight_recorder: recorder.clone(), diagnostics: recorder,
                llm_client: Arc::new(crate::llm::ollama::InMemoryLlmClient::new("unused".into())),
                capability_registry: Arc::new(crate::capabilities::CapabilityRegistry::new()),
                session_registry: Arc::new(crate::workflows::SessionRegistry::new(crate::workflows::SessionSchedulerConfig::default())),
            };
            let binding = WorkspaceBindingFixture::new().unwrap();
            let (_owner, headers) = workspace_test_principal(&state, &binding, "mt196-http-owner").await.unwrap();
            let (_, other_headers) = workspace_test_principal(&state, &binding, "mt196-http-other").await.unwrap();
            let workspace = create_owned_test_workspace(&state, &headers).await.unwrap();
            let registry_url = format!("/workspaces/{}/preferences/{}", workspace.id, PREF_WORK_PROFILE_REGISTRY);
            let selection_url = format!("/workspaces/{}/preferences/{}", workspace.id, PREF_WORK_PROFILE_SELECTION);
            let (status, _) = preference_http(&state, &headers, "PUT", &registry_url, json!({"value":registry()})).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "special key requires explicit CAS");
            let (status, configured) = preference_http(&state, &headers, "PUT", &registry_url, json!({"value":registry(),"expected_revision":0})).await;
            assert_eq!(status, StatusCode::OK, "{configured}");
            assert_eq!(configured["record"]["revision"],1);
            assert_eq!(configured["receipt"]["actor"], "mt196-http-owner");
            assert_ne!(configured["receipt"]["receipt_id"], "forged-header-actor");
            let (status, selection) = preference_http(&state, &headers, "PUT", &selection_url,
                json!({"value":{"profile_id":"local","profile_version":1,"role_id":"coder","registry_revision":1},"expected_revision":0})).await;
            assert_eq!(status,StatusCode::OK,"{selection}");
            let (status, reread) = preference_http(&state, &headers, "GET", &registry_url, Value::Null).await;
            assert_eq!(status,StatusCode::OK);
            assert_eq!(reread["record"],configured["record"]);
            let (status, history) = preference_http(&state, &headers, "GET", &format!("{registry_url}/history"), Value::Null).await;
            assert_eq!(status,StatusCode::OK);
            assert_eq!(history["receipts"][0]["receipt_id"], configured["receipt"]["receipt_id"]);
            let (status, _) = preference_http(&state, &headers, "PUT", &registry_url, json!({"value":registry(),"expected_revision":0})).await;
            assert_eq!(status,StatusCode::CONFLICT);
            let (status, _) = preference_http(&state, &other_headers, "PUT", &registry_url, json!({"value":registry(),"expected_revision":1})).await;
            assert_eq!(status,StatusCode::FORBIDDEN);
            let (status, _) = preference_http(&state, &axum::http::HeaderMap::new(), "PUT", &registry_url, json!({"value":registry(),"expected_revision":1})).await;
            assert_eq!(status,StatusCode::FORBIDDEN);
            let (status, reset) = preference_http(&state, &headers, "POST", &format!("{registry_url}/reset"), json!({"expected_revision":1})).await;
            assert_eq!(status,StatusCode::OK,"{reset}");
            assert_eq!(reset["record"]["revision"],2);
            assert_eq!(reset["record"]["value"],json!({}));
            assert_eq!(reset["record"]["source"],"default");
            drop(binding); drop(state);
            backend.close_and_remove().await.unwrap();
        }

        pub async fn authenticated_snapshot_cas_and_lease_contract() {
            let backend = crate::storage::tests::embedded_test_backend().await.expect("isolated embedded Preferences fixture");
            let storage = backend.storage.clone();
            let outcome = tokio::spawn(async move { authenticated_body(&storage).await }).await;
            backend.close_and_remove().await.expect("close isolated Preferences fixture");
            if let Err(panic) = outcome { std::panic::resume_unwind(panic.into_panic()); }
        }
    };
}
pub(crate) use authenticated_store_contract_tests;

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn authenticated_store_integration_contract() {
    handshake_core::storage::surreal::work_profile_contract_test_support::authenticated_snapshot_cas_and_lease_contract().await;
}

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn mounted_store_integration_contract() {
    handshake_core::storage::surreal::work_profile_contract_test_support::mounted_preferences_http_cas_receipts_and_reset().await;
}

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn submitted_store_settlement_integration_contract() {
    handshake_core::storage::surreal::work_profile_contract_test_support::submitted_write_settlement_contract().await;
}

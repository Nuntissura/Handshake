//! Mounted production indexing, source-scoped navigation and owned cleanup.
#![cfg(all(feature = "duckdb-flight-recorder", feature = "os-keychain"))]

use std::sync::Arc;
use axum::{body::{to_bytes, Body}, http::{HeaderMap, Request, StatusCode}, Router};
use serde_json::{json, Value};
use surrealdb::types::SurrealValue;
use crate::storage::knowledge::{KnowledgeEdgeType, KnowledgeStore};
use crate::api::MountedRequestExt;
use crate::AppState;

struct Binding {
    _lock: std::sync::MutexGuard<'static, ()>,
    _directory: tempfile::TempDir,
    previous: Option<std::ffi::OsString>,
    channel: String,
}
impl Binding {
    fn new() -> Self {
        let lock = crate::api::stage::NATIVE_BINDING_ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let directory = tempfile::tempdir_in(crate::storage::tests::test_store_root().unwrap()).unwrap();
        let path = directory.path().join("index-binding.json");
        let channel = "d6".repeat(32);
        std::fs::write(&path, serde_json::to_vec(&crate::api::stage::current_process_native_binding(&channel)).unwrap()).unwrap();
        let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
        Self { _lock: lock, _directory: directory, previous, channel }
    }
}
impl Drop for Binding {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous { std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous); }
        else { std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE"); }
    }
}

async fn request(router: &Router, method: &str, uri: &str, headers: &HeaderMap, body: Value) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri).header("content-type", "application/json");
    for (name, value) in headers { request = request.header(name, value); }
    let response = router.clone().oneshot(request.body(Body::from(serde_json::to_vec(&body).unwrap())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024).await.unwrap();
    (status, if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() })
}

#[tokio::test]
async fn mounted_index_account_source_witness_revocation_and_workspace_delete() {
    let binding = Binding::new();
    let backend = crate::storage::tests::embedded_test_backend().await.unwrap();
    let recorder = Arc::new(crate::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7).unwrap());
    let state = AppState {
        storage: backend.database.clone(), surreal: backend.storage.clone(),
        flight_recorder: recorder.clone(), diagnostics: recorder,
        llm_client: Arc::new(crate::llm::ollama::InMemoryLlmClient::new("unused".into())),
        capability_registry: Arc::new(crate::capabilities::CapabilityRegistry::new()),
        session_registry: Arc::new(crate::workflows::SessionRegistry::new(crate::workflows::SessionSchedulerConfig::default())),
    };
    let router = crate::api::authority::routes(state.clone())
        .merge(crate::api::workspaces::routes(state.clone()))
        .merge(super::routes(state.clone()))
        .merge(crate::api::knowledge_code_nav::routes(state.clone()));
    let mut headers = HeaderMap::new();
    headers.insert("x-hsk-channel-binding-token", binding.channel.parse().unwrap());
    let password = json!({"account_name":"Index owner", "password":"index owner password for runtime proof"});
    assert_eq!(request(&router,"POST","/authority/setup",&headers,password.clone()).await.0,StatusCode::OK);
    let (status, credential) = request(&router,"POST","/authority/login",&headers,password).await;
    assert_eq!(status,StatusCode::OK,"real account login failed");
    let (status, session) = request(&router,"POST","/authority/session",&headers,json!({
        "account_id":credential["account_id"],"principal_id":credential["principal_id"],
        "access_space_id":credential["access_space_id"],"authentication_token":credential["token"]
    })).await;
    assert_eq!(status,StatusCode::OK,"real session exchange failed");
    headers.insert("x-hsk-session-token",session["session_token"].as_str().unwrap().parse().unwrap());
    headers.insert("x-hsk-actor-id",session["principal_id"].as_str().unwrap().parse().unwrap());
    headers.insert("x-hsk-actor-kind","operator".parse().unwrap());
    headers.insert("x-hsk-kernel-task-run-id","mounted-index-proof".parse().unwrap());
    headers.insert("x-hsk-session-run-id","mounted-index-proof".parse().unwrap());
    let (status, workspace) = request(&router,"POST","/workspaces",&headers,json!({"name":"Owned indexed workspace"})).await;
    assert_eq!(status,StatusCode::CREATED,"owned workspace create: {workspace}");
    let workspace_id = workspace["id"].as_str().unwrap();
    let files = tempfile::tempdir_in(crate::storage::tests::test_store_root().unwrap()).unwrap();
    std::fs::write(
        files.path().join("probe.rs"),
        "pub fn account_index_target() -> u32 { 7 }\npub fn account_index_probe() -> u32 { account_index_target() }\n",
    ).unwrap();
    let (status, indexed) = request(&router,"POST",&format!("/workspaces/{workspace_id}/code-nav/index"),&headers,
        json!({"root_path":files.path()})).await;
    assert_eq!(status,StatusCode::OK,"production index failed: {indexed}");
    assert_eq!(indexed["files_indexed"],1);
    assert_eq!(indexed["files_failed"],0);
    assert!(indexed["symbol_count"].as_u64().unwrap() > 0);
    let lookup = format!("/knowledge/code/symbols?workspace_id={workspace_id}&name=account_index_probe");
    let (status, nav) = request(&router,"GET",&lookup,&headers,Value::Null).await;
    assert_eq!(status,StatusCode::OK,"indexed navigation: {nav}");
    let matches = nav["matches"].as_array().unwrap();
    assert_eq!(matches.len(),1);
    let entity_id = matches[0]["symbol_entity_id"].as_str().unwrap().to_owned();
    let source_id = matches[0]["primary_source_id"].as_str().unwrap().to_owned();
    let references_uri = format!("/knowledge/code/symbols/{entity_id}/references");
    let (status, references) = request(&router,"GET",&references_uri,&headers,Value::Null).await;
    assert_eq!(status,StatusCode::OK,"production symbol references: {references}");
    assert_eq!(references["symbol_entity_id"].as_str(),Some(entity_id.as_str()));
    assert!(references["callees"].as_array().unwrap().iter().any(|callee| {
        callee["display_name"].as_str() == Some("account_index_target")
    }),"real same-source call must produce a navigable reference edge: {references}");
    let reference_receipt_id = references["nav_receipt_event_id"].as_str().unwrap();
    let mut canonical = state.surreal.test_admin_query(format!(
        "SELECT payload FROM kernel_event_ledger WHERE event_id = '{reference_receipt_id}';"
    )).await.unwrap();
    let reference_receipts: Vec<Value> = canonical.take(0).unwrap();
    assert_eq!(reference_receipts.len(),1);
    let witnessed_edge_ids = reference_receipts[0]["payload"]["read_witnesses"]["edge_ids"]
        .as_array().unwrap();
    assert!(!witnessed_edge_ids.is_empty(),"symbol reference receipt must witness its real edge");
    let edge_read_scope = crate::api::authority::authorize_request(
        &state,
        &headers,
        "memory.read",
        crate::storage::surreal::resource_authority::ResourceKind::Workspace,
        workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await
    .expect("real account read scope for edge policy")
    .record_user_scope;
    let edge_reader = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
    let visible_edges = state
        .surreal
        .with_record_user_scope(
            edge_read_scope.clone(),
            edge_reader.list_knowledge_edges_for_entity(&entity_id),
        )
        .await
        .expect("record-user read must reach the mounted edge policy");
    assert!(visible_edges.iter().any(|edge| {
        edge.edge_type == KnowledgeEdgeType::References
            && witnessed_edge_ids.iter().any(|id| id.as_str() == Some(edge.edge_id.as_str()))
    }),"record-user edge read must return the same real reference edge witnessed by the route");
    #[derive(SurrealValue)]
    struct SourceMutationBindings {
        source: String,
        relative_path: String,
    }
    let source_scope = crate::api::authority::authorize_request(
        &state,
        &headers,
        "memory.propose",
        crate::storage::surreal::resource_authority::ResourceKind::Workspace,
        workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await
    .expect("real index workspace scope")
    .record_user_scope;
    let valid_source_update = state
        .surreal
        .with_record_user_scope(
            source_scope.clone(),
            state.surreal.with_data_operation({
                let source = source_id.clone();
                move |database| {
                    Box::pin(async move {
                        database
                            .execute_returning(
                                "UPDATE type::record('knowledge_sources', $source) SET updated_at = time::now() RETURN AFTER;",
                                SourceMutationBindings {
                                    source,
                                    relative_path: String::new(),
                                },
                            )
                            .await
                    })
                }
            }),
        )
        .await
        .expect("authorized source update must pass its record-user event guard");
    assert_eq!(valid_source_update, 1);
    let mut canonical_source = state.surreal.test_admin_query_bound(
        "SELECT relative_path FROM type::record('knowledge_sources', $source);".to_owned(),
        json!({"source": source_id.clone()}),
    ).await.unwrap().check().unwrap();
    let source_rows: Vec<Value> = canonical_source.take(0).unwrap();
    assert_eq!(source_rows.len(), 1, "one indexed source before tamper attempt");
    let original_relative_path = source_rows[0]["relative_path"].clone();
    let rejected_source_lineage = state
        .surreal
        .with_record_user_scope(
            source_scope,
            state.surreal.with_data_operation({
                let source = source_id.clone();
                move |database| {
                    Box::pin(async move {
                        database
                            .execute_returning(
                                "UPDATE type::record('knowledge_sources', $source) SET relative_path = $relative_path RETURN AFTER;",
                                SourceMutationBindings {
                                    source,
                                    relative_path: "tampered-probe.rs".to_owned(),
                                },
                            )
                            .await
                    })
                }
            }),
        )
        .await
        .expect_err("source lineage mutation must fire the index update guard");
    assert!(
        rejected_source_lineage
            .to_string()
            .contains("HSK-403-PROTECTED-RESOURCE"),
        "source lineage mutation must be rejected by the update guard: {rejected_source_lineage}"
    );
    let mut canonical_source = state.surreal.test_admin_query_bound(
        "SELECT relative_path FROM type::record('knowledge_sources', $source);".to_owned(),
        json!({"source": source_id.clone()}),
    ).await.unwrap().check().unwrap();
    let source_rows: Vec<Value> = canonical_source.take(0).unwrap();
    assert_eq!(source_rows.len(), 1, "rejected source mutation must retain one row");
    assert_eq!(
        source_rows[0]["relative_path"], original_relative_path,
        "rejected source lineage mutation must not persist"
    );
    // Canonical reads inspect real producer output; they do not create index or ownership rows.
    let mut canonical = state.surreal.test_admin_query(format!(
        "SELECT source_id, indexed_content_hash, parser_version FROM knowledge_code_files WHERE source_id = type::record('knowledge_sources', '{source_id}');"
    )).await.unwrap();
    let code_files: Vec<Value> = canonical.take(0).unwrap();
    assert_eq!(code_files.len(),1);
    let lens_uri = format!("/knowledge/code/files/probe.rs/lens?workspace_id={workspace_id}&content_hash={}&parser_version={}",
        code_files[0]["indexed_content_hash"].as_str().unwrap(),code_files[0]["parser_version"].as_str().unwrap());
    let (status, lens) = request(&router,"GET",&lens_uri,&headers,Value::Null).await;
    assert_eq!(status,StatusCode::OK,"production file lens: {lens}");
    assert!(!lens["entries"].as_array().unwrap().is_empty());
    let receipt_id = lens["nav_receipt_event_id"].as_str().unwrap();
    let mut canonical = state.surreal.test_admin_query(format!(
        "SELECT payload, actor_id, session_run_id FROM kernel_event_ledger WHERE event_id = '{receipt_id}';"
    )).await.unwrap();
    let receipts: Vec<Value> = canonical.take(0).unwrap();
    assert_eq!(receipts.len(),1);
    assert_eq!(receipts[0]["actor_id"],session["principal_id"]);
    assert_eq!(receipts[0]["session_run_id"],session["session_id"]);
    assert!(receipts[0]["payload"]["read_witnesses"]["source_ids"].as_array().unwrap().contains(&json!(source_id)));
    assert!(receipts[0]["payload"]["read_witnesses"]["entity_ids"].as_array().unwrap().contains(&json!(entity_id)));
    let mut grants = state.surreal.test_admin_query(format!(
        "SELECT VALUE record::id(id) FROM resource_grants WHERE resource_id.resource_kind = 'knowledge_source' AND resource_id.external_resource_id = '{source_id}' AND principal_id = type::record('principals', '{}') AND status = 'active';",session["principal_id"].as_str().unwrap()
    )).await.unwrap();
    let grant_ids: Vec<String> = grants.take(0).unwrap();
    assert_eq!(grant_ids.len(),1,"exact source creator grant");
    // Negative setup revokes the actual producer-created source grant; workspace access stays active.
    state.surreal.revoke_grant(&grant_ids[0]).await.unwrap();
    let revoked_edges = state
        .surreal
        .with_record_user_scope(
            edge_read_scope.clone(),
            edge_reader.list_knowledge_edges_for_entity(&entity_id),
        )
        .await
        .expect("revoked record-user edge read stays a bounded successful query");
    assert!(revoked_edges.is_empty(),"revoking the real source grant must hide every edge whose endpoints and evidence intersect that source");
    assert_eq!(request(&router,"GET",&references_uri,&headers,Value::Null).await,
        (StatusCode::FORBIDDEN,json!({"error":"HSK-403-PROTECTED-RESOURCE"})));
    let (status, filtered) = request(&router,"GET",&lookup,&headers,Value::Null).await;
    assert_eq!(status,StatusCode::OK,"workspace remains authorized: {filtered}");
    assert_eq!(filtered["matches"],json!([]),"revoked source cannot leak symbols/counts");
    let denial = json!({"error":"HSK-403-PROTECTED-RESOURCE"});
    assert_eq!(request(&router,"GET",&format!("/knowledge/code/symbols/{entity_id}"),&headers,Value::Null).await,
        (StatusCode::FORBIDDEN,denial.clone()));
    assert_eq!(request(&router,"GET",&lens_uri,&headers,Value::Null).await,(StatusCode::FORBIDDEN,denial.clone()));
    assert_eq!(request(&router,"DELETE",&format!("/workspaces/{workspace_id}"),&headers,Value::Null).await,
        (StatusCode::FORBIDDEN,denial));
    // Undo only the negative fixture mutation; no new grant or wider capability is introduced.
    let mut restored = state.surreal.test_admin_query(format!(
        "UPDATE type::record('resource_grants', '{}') SET status = 'active', revoked_at = NONE, grant_version -= 1, policy_version -= 1 RETURN VALUE record::id(id);",grant_ids[0]
    )).await.unwrap().check().unwrap();
    assert_eq!(restored.take::<Vec<String>>(0).unwrap(), grant_ids);
    assert_eq!(request(&router,"DELETE",&format!("/workspaces/{workspace_id}"),&headers,Value::Null).await,
        (StatusCode::NO_CONTENT,Value::Null));
    let mut canonical = state.surreal.test_admin_query(format!(
        "RETURN {{workspaces: array::len(SELECT id FROM workspaces WHERE id = type::record('workspaces', '{workspace_id}')), sources: array::len(SELECT id FROM knowledge_sources WHERE workspace_id = type::record('workspaces', '{workspace_id}')), files: array::len(SELECT id FROM knowledge_code_files WHERE workspace_id = type::record('workspaces', '{workspace_id}')), entities: array::len(SELECT id FROM knowledge_entities WHERE workspace_id = type::record('workspaces', '{workspace_id}'))}};"
    )).await.unwrap();
    assert_eq!(canonical.take::<Option<Value>>(0).unwrap(),Some(json!({"workspaces":0,"sources":0,"files":0,"entities":0})));
    assert_eq!(request(&router,"POST","/authority/logout",&headers,Value::Null).await.0,StatusCode::OK);
    state.surreal.shutdown().await.unwrap();
}

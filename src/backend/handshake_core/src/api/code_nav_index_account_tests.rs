//! Mounted production indexing, source-scoped navigation and owned cleanup.
#![cfg(all(feature = "duckdb-flight-recorder", feature = "os-keychain"))]

use std::{panic::AssertUnwindSafe, sync::Arc};
use axum::{body::{to_bytes, Body}, http::{HeaderMap, HeaderValue, Request, StatusCode}, response::Response, Router};
use futures::FutureExt;
use serde_json::{json, Value};
use surrealdb::types::{RecordId, SurrealValue};
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
    let response = mounted_response(router, method, uri, headers, body).await;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024).await.unwrap();
    (status, if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() })
}

async fn mounted_response(router: &Router, method: &str, uri: &str, headers: &HeaderMap, body: Value) -> Response {
    let mut request = Request::builder().method(method).uri(uri).header("content-type", "application/json");
    for (name, value) in headers { request = request.header(name, value); }

    router.clone().oneshot(request.body(Body::from(serde_json::to_vec(&body).unwrap())).unwrap()).await.unwrap()
}

struct MountedIndexAccountFixture {
    _binding: Binding,
    backend: crate::storage::tests::EmbeddedTestBackend,
    state: AppState,
    router: Router,
    headers: HeaderMap,
    session: Value,
}

async fn mounted_index_account_fixture(compute_only: bool) -> MountedIndexAccountFixture {
    let binding = Binding::new();
    let backend = if compute_only {
        crate::storage::tests::embedded_test_backend_compute_only().await
    } else {
        crate::storage::tests::embedded_test_backend().await
    }
    .unwrap();
    let (state, router) = mount_index_account(&backend);
    let mut headers = index_account_channel_headers(&binding);
    let session = issue_index_account_session(&router, &mut headers).await
        .unwrap_or_else(|phase| panic!("mounted index account fixture phase={phase}"));
    MountedIndexAccountFixture { _binding: binding, backend, state, router, headers, session }
}

fn mount_index_account(backend: &crate::storage::tests::EmbeddedTestBackend) -> (AppState, Router) {
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
    (state, router)
}

fn index_account_channel_headers(binding: &Binding) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let mut channel: HeaderValue = binding.channel.parse().unwrap();
    channel.set_sensitive(true);
    headers.insert("x-hsk-channel-binding-token", channel);
    headers
}

async fn issue_index_account_session(router: &Router, headers: &mut HeaderMap) -> Result<Value, &'static str> {
    let password = json!({"account_name":"Index owner", "password":"index owner password for runtime proof"});
    if request(router, "POST", "/authority/setup", headers, password.clone()).await.0 != StatusCode::OK {
        return Err("fixture_setup_denied");
    }
    let (status, credential) = request(router, "POST", "/authority/login", headers, password).await;
    if status != StatusCode::OK {
        return Err("fixture_login_denied");
    }
    let response = mounted_response(router, "POST", "/authority/session", headers, json!({
        "account_id":credential["account_id"],"principal_id":credential["principal_id"],
        "access_space_id":credential["access_space_id"],"authentication_token":credential["token"]
    })).await;
    if response.status() != StatusCode::OK {
        return Err("session_exchange_unconfirmed");
    }
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024).await
        .map_err(|_| "session_exchange_body_unconfirmed")?;
    let mut session: Value = serde_json::from_slice(&bytes)
        .map_err(|_| "session_exchange_body_unconfirmed")?;
    let token = session.get_mut("session_token").map(Value::take)
        .ok_or("session_exchange_token_unconfirmed")?;
    let mut token: HeaderValue = token.as_str().and_then(|value| value.parse().ok())
        .ok_or("session_exchange_token_unconfirmed")?;
    token.set_sensitive(true);
    // Retain the issued credential before validating other response fields.
    headers.insert("x-hsk-session-token", token);
    let actor = session["principal_id"].as_str().and_then(|value| value.parse::<HeaderValue>().ok())
        .ok_or("fixture_principal")?;
    headers.insert("x-hsk-actor-id", actor);
    headers.insert("x-hsk-actor-kind", HeaderValue::from_static("operator"));
    headers.insert("x-hsk-kernel-task-run-id", HeaderValue::from_static("mounted-index-proof"));
    headers.insert("x-hsk-session-run-id", HeaderValue::from_static("mounted-index-proof"));
    Ok(session)
}

async fn first_owner_workspace_receipt_scalar_outcome(compute_only: bool) -> &'static str {
    use crate::storage::surreal::resource_authority::{RecordUserScope, ResourceAction};

    let MountedIndexAccountFixture { _binding, backend, state, router, headers, .. } =
        mounted_index_account_fixture(compute_only).await;
    let credentials = crate::api::authority::authenticated_session_credentials(&state, &headers)
        .await
        .unwrap_or_else(|_| panic!("receipt scalar probe account authentication failed"));
    // Same prospective Workspace/Create scope as the real collection handler.
    let scope = RecordUserScope {
        workspace_id: Some(uuid::Uuid::now_v7().to_string()),
        resource_id: uuid::Uuid::now_v7().to_string(),
        session_id: credentials.context.session_id.clone(),
        session_token: credentials.session_token,
        channel_binding_hash: Some(credentials.channel_binding_hash),
        capability_id: "fs.write".to_owned(),
        action: ResourceAction::Create,
        grant_id: None,
    };
    let observed = state
        .surreal
        .probe_first_owner_workspace_receipt_projection(
            &credentials.context,
            &scope,
            crate::storage::NewWorkspace { name: "Owned indexed workspace".to_owned() },
        )
        .await;
    let Ok(outcome) = observed else {
        drop(router);
        drop(state);
        assert!(backend.close_and_preserve().await.is_ok(), "receipt scalar probe fixture close unobserved");
        panic!("receipt scalar probe rollback unobserved");
    };
    let logout_ok = request(&router, "POST", "/authority/logout", &headers, Value::Null).await.0
        == StatusCode::OK;
    drop(router);
    drop(state);
    assert!(backend.close_and_remove().await.is_ok(), "receipt scalar probe fixture cleanup unobserved");
    assert!(logout_ok, "receipt scalar probe logout failed after checked rollback");
    outcome
}

#[tokio::test]
async fn first_owner_workspace_receipt_scalar_projection_observes_rollback() {
    let outcome = first_owner_workspace_receipt_scalar_outcome(false).await;
    // A scalar failure cannot count as receipt acceptance.
    assert_eq!(outcome, "one_visible", "receipt scalar probe after checked rollback");
}

#[tokio::test]
async fn first_owner_workspace_receipt_compute_only_observes_rollback() {
    let outcome = first_owner_workspace_receipt_scalar_outcome(true).await;
    // This compares the fixed outcome with the archived default-planner run.
    assert_eq!(outcome, "one_visible", "receipt compute-only scalar probe after checked rollback");
}

// PT003 observer (R172-02, folded into R172-01): no production code is touched.
const PT003_SETUP_RECEIPT_TERMS: &str = r#"
    $row.purpose = 'prospective_first_owner_administration' AND $row.version = 1
    AND $row.account.account_role = 'Owner'
    AND $row.account.first_owner_administration_origin = first_owner_administration_origins:primary
    AND $row.setup_receipt.event_type = 'LOCAL_ACCOUNT_SETUP'
    AND $row.setup_receipt.event_sequence > 0 AND $row.setup_receipt.created_at = $row.created_at
    AND $row.setup_receipt.actor_kind = 'operator'
    AND $row.setup_receipt.actor_id = record::id($row.operator_principal)
    AND $row.setup_receipt.aggregate_type = 'local_account'
    AND $row.setup_receipt.aggregate_id = record::id($row.account)
    AND $row.setup_receipt.source_component = 'local_account_authority'
    AND $row.setup_receipt.payload.schema_version = 'hsk.local_account_event@1'
    AND $row.setup_receipt.payload.account_id = record::id($row.account)
    AND $row.setup_receipt.payload.principal_id = record::id($row.operator_principal)
    AND $row.setup_receipt.payload.access_space_id = record::id($row.space)
    AND $row.setup_receipt.payload.session_id = NULL
    AND $row.setup_receipt.payload.delegation_chain = [record::id($row.operator_principal)]
    AND $row.setup_receipt.payload.resource_refs = [{ kind: 'local_account', id: record::id($row.account) }]
    AND $row.setup_receipt.payload.action = 'LOCAL_ACCOUNT_SETUP'
    AND $row.setup_receipt.payload.result = 'allow' AND $row.setup_receipt.payload.policy_version = 1
    AND local_account_setup:primary.account_id = $row.account
    AND local_account_setup:primary.principal_id = $row.operator_principal
"#;

const PT003_CONJUNCTS: &str = r#"
SELECT
 record::exists(setup_receipt) AS a_exists_setup_receipt,
 setup_receipt.event_type AS b_event_type,
 purpose = 'prospective_first_owner_administration' AS c01_purpose, version = 1 AS c02_version,
 account.account_role = 'Owner' AS c03_owner_role,
 account.first_owner_administration_origin = first_owner_administration_origins:primary AS c04_origin_link,
 first_workspace_state AS v_first_workspace_state, first_workspace_receipt_nonce AS v_nonce,
 first_workspace_original != NONE AS v_original_present,
 setup_receipt.event_type = 'LOCAL_ACCOUNT_SETUP' AS c05_event_type,
 setup_receipt.event_sequence > 0 AS c06_event_sequence,
 setup_receipt.created_at = created_at AS c07_created_at,
 setup_receipt.actor_kind = 'operator' AS c08_actor_kind,
 setup_receipt.actor_id = record::id(operator_principal) AS c09_actor_id,
 setup_receipt.aggregate_type = 'local_account' AS c10_aggregate_type,
 setup_receipt.aggregate_id = record::id(account) AS c11_aggregate_id,
 setup_receipt.source_component = 'local_account_authority' AS c12_source_component,
 setup_receipt.payload.schema_version = 'hsk.local_account_event@1' AS c13_schema_version,
 setup_receipt.payload.account_id = record::id(account) AS c14_payload_account,
 setup_receipt.payload.principal_id = record::id(operator_principal) AS c15_payload_principal,
 setup_receipt.payload.access_space_id = record::id(space) AS c16_payload_space,
 setup_receipt.payload.session_id = NULL AS c17_payload_session_null,
 setup_receipt.payload.delegation_chain = [record::id(operator_principal)] AS c18_delegation,
 setup_receipt.payload.resource_refs = [{ kind: 'local_account', id: record::id(account) }] AS c19_resource_refs,
 setup_receipt.payload.action = 'LOCAL_ACCOUNT_SETUP' AS c20_payload_action,
 setup_receipt.payload.result = 'allow' AS c21_payload_result,
 setup_receipt.payload.policy_version = 1 AS c22_policy_version,
 local_account_setup:primary.account_id = account AS c23_setup_account,
 local_account_setup:primary.principal_id = operator_principal AS c24_setup_principal
FROM first_owner_administration_origins:primary
WHERE account = type::record('local_accounts', $proof_account);
"#;

const PT003_COPIES: &str = r#"
SELECT fn::pt003_copy_full($this) AS copy_full, fn::pt003_copy_no_exists($this) AS copy_no_exists,
 fn::pt003_copy_param($this, type::record('authenticated_sessions', $proof_session)) AS copy_param
FROM first_owner_administration_origins:primary
WHERE account = type::record('local_accounts', $proof_account);
"#;

const PT003_STRICT_PROJECTION: &str = r#"
SELECT fn::wp012_first_owner_workspace_receipt($this, $proof_op, type::record('workspaces', $proof_op),
 type::record('protected_resources', $proof_resource), type::record('resource_grants', $proof_resource),
 type::record('authenticated_sessions', $proof_session), $proof_channel, $proof_name, time::now()) AS strict_result
FROM first_owner_administration_origins:primary
WHERE account = type::record('local_accounts', $proof_account);
"#;

const PT003_STRICT_WHERE: &str = r#"
SELECT VALUE id FROM first_owner_administration_origins:primary
WHERE fn::wp012_first_owner_workspace_receipt($this, $proof_op, type::record('workspaces', $proof_op),
 type::record('protected_resources', $proof_resource), type::record('resource_grants', $proof_resource),
 type::record('authenticated_sessions', $proof_session), $proof_channel, $proof_name, time::now());
"#;

const PT003_SOURCES: &str = r#"
SELECT fn::wp012_first_owner_workspace_sources($this) AS sources_result
FROM first_owner_administration_origins:primary
WHERE account = type::record('local_accounts', $proof_account);
"#;

async fn pt003_record_user_view(
    state: &AppState, scope: &crate::storage::surreal::resource_authority::RecordUserScope,
    statement: &'static str, bindings: &Value,
) -> String {
    let bindings = bindings.clone();
    let result = state.surreal.with_record_user_scope(scope.clone(), state.surreal.with_data_operation(move |database| {
        Box::pin(async move { database.query_values::<surrealdb::types::Value, _>(statement, bindings).await })
    })).await;
    format!("{result:?}")
}

async fn pt003_root_view(state: &AppState, statement: &'static str, bindings: &Value) -> String {
    match state.surreal.test_admin_query_bound(statement.to_owned(), bindings.clone()).await {
        Ok(mut response) => format!("{:?}", response.take::<surrealdb::types::Value>(0)),
        Err(error) => format!("ADMIN_ERR {error:?}"),
    }
}

#[tokio::test]
async fn pt003_first_owner_setup_receipt_observer() {
    use crate::storage::surreal::resource_authority::{RecordUserScope, ResourceAction};

    let MountedIndexAccountFixture { _binding, backend, state, router, headers, session } =
        mounted_index_account_fixture(false).await;
    let credentials = crate::api::authority::authenticated_session_credentials(&state, &headers)
        .await
        .unwrap_or_else(|_| panic!("pt003 observer account authentication failed"));
    let operation = uuid::Uuid::now_v7().to_string();
    let resource = uuid::Uuid::now_v7().to_string();
    // Same prospective Workspace/Create scope as first_owner_workspace_receipt_scalar_outcome.
    let scope = RecordUserScope {
        workspace_id: Some(operation.clone()),
        resource_id: resource.clone(),
        session_id: credentials.context.session_id.clone(),
        session_token: credentials.session_token,
        channel_binding_hash: Some(credentials.channel_binding_hash.clone()),
        capability_id: "fs.write".to_owned(),
        action: ResourceAction::Create,
        grant_id: None,
    };
    let bindings = json!({
        "proof_account": session["account_id"], "proof_session": session["session_id"],
        "proof_op": operation, "proof_resource": resource,
        "proof_channel": credentials.channel_binding_hash, "proof_name": "Owned indexed workspace",
    });
    // Test-local copies of fn::wp012_first_owner_workspace_receipt (schema.surql 8471-8500): the
    // setup-receipt and row-level terms only, with and without record::exists($row.setup_receipt).
    // The third copy takes a typed record parameter to observe the (UNVERIFIED) parameter candidate.
    let terms = PT003_SETUP_RECEIPT_TERMS;
    let ddl = format!(
        "DEFINE FUNCTION OVERWRITE fn::pt003_copy_full($row: object) {{ RETURN record::exists($row.setup_receipt) AND {terms}; }};
         DEFINE FUNCTION OVERWRITE fn::pt003_copy_no_exists($row: object) {{ RETURN {terms}; }};
         DEFINE FUNCTION OVERWRITE fn::pt003_copy_param($row: object, $sx: record<authenticated_sessions>) {{ RETURN $sx != NONE AND {terms}; }};"
    );
    let mut report = String::new();
    match state.surreal.test_admin_query(ddl).await {
        Ok(mut response) => {
            for index in 0..3 {
                report.push_str(&format!("PT003 ddl[{index}] = {:?}
", response.take::<Option<surrealdb::types::Value>>(index)));
            }
        }
        Err(error) => report.push_str(&format!("PT003 ddl ADMIN_ERR {error:?}
")),
    }
    let statements: [(&str, &'static str); 5] = [
        ("conjuncts", PT003_CONJUNCTS), ("copies", PT003_COPIES), ("strict_projection", PT003_STRICT_PROJECTION),
        ("strict_where", PT003_STRICT_WHERE), ("sources", PT003_SOURCES),
    ];
    for (label, statement) in statements {
        report.push_str(&format!("PT003 record_user {label} = {}
", pt003_record_user_view(&state, &scope, statement, &bindings).await));
        report.push_str(&format!("PT003 root {label} = {}
", pt003_root_view(&state, statement, &bindings).await));
    }
    report.push_str(&pt003_o1_to_o3b(&state, &scope, &bindings).await);
    pt003_write_report("pt003-observer.txt", &report);
    drop(router);
    drop(state);
    assert!(backend.close_and_remove().await.is_ok(), "pt003 observer fixture cleanup unobserved");
}

// R172-02c/02d confirming run: test code only, no production code is touched.
const PT003_REPORT_DIR: &str = "WP-KERNEL-012/MT-172/r172-02d-20261128";

fn pt003_write_report(name: &str, report: &str) {
    eprintln!("{report}");
    if let Some(root) = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT") {
        let dir = std::path::Path::new(&root).join(PT003_REPORT_DIR);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(name), report);
    }
}

async fn pt003_root_owned(state: &AppState, statement: String, bindings: &Value) -> String {
    match state.surreal.test_admin_query_bound(statement, bindings.clone()).await {
        Ok(mut response) => format!("{:?}", response.take::<surrealdb::types::Value>(0)),
        Err(error) => format!("ADMIN_ERR {error:?}"),
    }
}

async fn pt003_root_ddl(state: &AppState, ddl: String, count: usize) -> String {
    match state.surreal.test_admin_query(ddl).await {
        Ok(mut response) => (0..count)
            .map(|index| format!("  ddl[{index}] = {:?}\n", response.take::<Option<surrealdb::types::Value>>(index)))
            .collect(),
        Err(error) => format!("  ddl ADMIN_ERR {error:?}\n"),
    }
}

fn pt003_verdict(view: &str, marker: &str) -> &'static str {
    if !view.starts_with("Ok(") { "ERR" } else if view.contains(marker) { "visible" } else { "hidden" }
}

fn pt003_leak(statement: String) -> &'static str { Box::leak(statement.into_boxed_str()) }

// Same prospective Workspace/Create scope as first_owner_workspace_receipt_scalar_outcome.
macro_rules! pt003_scope_and_bindings {
    ($credentials:expr, $session:expr) => {{
        use crate::storage::surreal::resource_authority::{RecordUserScope, ResourceAction};
        let operation = uuid::Uuid::now_v7().to_string();
        let resource = uuid::Uuid::now_v7().to_string();
        let scope = RecordUserScope {
            workspace_id: Some(operation.clone()), resource_id: resource.clone(),
            session_id: $credentials.context.session_id.clone(), session_token: $credentials.session_token.clone(),
            channel_binding_hash: Some($credentials.channel_binding_hash.clone()), capability_id: "fs.write".to_owned(),
            action: ResourceAction::Create, grant_id: None,
        };
        let bindings = json!({
            "proof_account": $session["account_id"], "proof_session": $session["session_id"],
            "proof_op": operation, "proof_resource": resource,
            "proof_channel": $credentials.channel_binding_hash.clone(), "proof_name": "Owned indexed workspace",
        });
        (scope, bindings)
    }};
}

/// O1 (root INFO), O2 (record user direct reads), O3 (gate tables), O3b (custom function in a WHERE).
async fn pt003_o1_to_o3b(
    state: &AppState, scope: &crate::storage::surreal::resource_authority::RecordUserScope, bindings: &Value,
) -> String {
    let mut out = String::new();
    for (label, statement) in [
        ("kernel_event_ledger", "INFO FOR TABLE kernel_event_ledger STRUCTURE;"),
        ("local_account_setup", "INFO FOR TABLE local_account_setup STRUCTURE;"),
        ("first_owner_administration_origins", "INFO FOR TABLE first_owner_administration_origins STRUCTURE;"),
        ("db", "INFO FOR DB;"),
    ] {
        out.push_str(&format!("PT003-O1 root {label} = {}\n", pt003_root_owned(state, statement.to_owned(), bindings).await));
    }

    // O2: the setup ledger row id is read as root, then probed as the record user at top level.
    let rid = match state.surreal.test_admin_query(
        "SELECT VALUE <string>setup_receipt FROM ONLY first_owner_administration_origins:primary;".to_owned(),
    ).await {
        Ok(mut response) => response.take::<Option<String>>(0).ok().flatten(),
        Err(_) => None,
    };
    out.push_str(&format!("PT003-O2 setup_receipt rid (root) = {rid:?}\n"));
    let mut bound = bindings.clone();
    if let Some(map) = bound.as_object_mut() { map.insert("proof_setup".to_owned(), json!(rid)); }
    let setup_statements: [(&str, &'static str); 5] = [
        ("setup_ledger_direct", "SELECT id, event_type FROM type::record($proof_setup);"),
        ("setup_ledger_by_filter", "SELECT id FROM kernel_event_ledger WHERE id = type::record($proof_setup);"),
        ("local_account_setup_primary", "SELECT * FROM local_account_setup:primary;"),
        ("exists_setup_rid", "RETURN record::exists(type::record($proof_setup));"),
        ("event_type_setup_rid", "RETURN type::record($proof_setup).event_type;"),
    ];
    for (label, statement) in setup_statements {
        out.push_str(&format!("PT003-O2 record_user {label} = {}\n", pt003_record_user_view(state, scope, statement, &bound).await));
        out.push_str(&format!("PT003-O2 root {label} = {}\n", pt003_root_owned(state, statement.to_owned(), &bound).await));
    }
    let control_statements: [(&str, &'static str); 6] = [
        ("ctl_origin_direct", "SELECT id FROM first_owner_administration_origins:primary;"),
        ("ctl_exists_origin", "RETURN record::exists(first_owner_administration_origins:primary);"),
        ("ctl_origin_deref", "RETURN first_owner_administration_origins:primary.purpose;"),
        ("ctl_exists_session", "RETURN record::exists(type::record('authenticated_sessions', $proof_session));"),
        ("ctl_exists_account", "RETURN record::exists(type::record('local_accounts', $proof_account));"),
        ("ctl_ledger_sample", "SELECT id, event_type FROM kernel_event_ledger LIMIT 3;"),
    ];
    for (label, statement) in control_statements {
        out.push_str(&format!("PT003-O2 record_user {label} = {}\n", pt003_record_user_view(state, scope, statement, &bound).await));
    }

    // O3: one-term SELECT predicates on test-local gate tables, each holding a copy of the origin row.
    let terms = PT003_SETUP_RECEIPT_TERMS;
    let inline_terms = terms.replace("$row", "$this");
    let full_inline = format!("record::exists($this.setup_receipt) AND {inline_terms}");
    let gates: Vec<(&str, String)> = vec![
        ("control", "true".to_owned()),
        ("exists", "record::exists($this.setup_receipt)".to_owned()),
        ("not_exists", "!record::exists($this.setup_receipt)".to_owned()),
        ("exists_account", "record::exists($this.account)".to_owned()),
        ("exists_origin", "record::exists(first_owner_administration_origins:primary)".to_owned()),
        ("deref", "$this.setup_receipt.event_type = 'LOCAL_ACCOUNT_SETUP'".to_owned()),
        ("las", "local_account_setup:primary.account_id = $this.account".to_owned()),
        ("full_inline", full_inline),
        ("noex_inline", inline_terms),
        ("full_fn", "fn::pt003_copy_full($this)".to_owned()),
        ("noex_fn", "fn::pt003_copy_no_exists($this)".to_owned()),
    ];
    let mut ddl = format!(
        "DEFINE FUNCTION OVERWRITE fn::pt003_copy_full($row: object) {{ RETURN record::exists($row.setup_receipt) AND {terms}; }};
         DEFINE FUNCTION OVERWRITE fn::pt003_copy_no_exists($row: object) {{ RETURN {terms}; }};
         DEFINE FUNCTION OVERWRITE fn::pt003_t_true($row: object) {{ RETURN true; }};
         DEFINE FUNCTION OVERWRITE fn::pt003_t_exists($row: object) {{ RETURN record::exists($row.setup_receipt); }};
         DEFINE FUNCTION OVERWRITE fn::pt003_t_not_exists($row: object) {{ RETURN !record::exists($row.setup_receipt); }};
         DEFINE FUNCTION OVERWRITE fn::pt003_t_exists_account($row: object) {{ RETURN record::exists($row.account); }};
         DEFINE FUNCTION OVERWRITE fn::pt003_t_deref($row: object) {{ RETURN $row.setup_receipt.event_type = 'LOCAL_ACCOUNT_SETUP'; }};
         DEFINE FUNCTION OVERWRITE fn::pt003_t_las($row: object) {{ RETURN local_account_setup:primary.account_id = $row.account; }};
         LET $o = (SELECT * OMIT id FROM ONLY first_owner_administration_origins:primary);
         DEFINE TABLE OVERWRITE pt003_open SCHEMALESS PERMISSIONS FULL;
         CREATE pt003_open:1 CONTENT $o;
         "
    );
    let mut statement_count = 11;
    for (name, predicate) in &gates {
        ddl.push_str(&format!(
            "DEFINE TABLE OVERWRITE pt003_gate_{name} SCHEMALESS PERMISSIONS FOR select WHERE {predicate} FOR create, update, delete NONE;
             CREATE pt003_gate_{name}:1 CONTENT $o;
             "
        ));
        statement_count += 2;
    }
    out.push_str(&format!("PT003-O3 gate DDL\n{}", pt003_root_ddl(state, ddl, statement_count).await));
    for (name, _) in &gates {
        let statement = pt003_leak(format!("SELECT VALUE id FROM pt003_gate_{name};"));
        let view = pt003_record_user_view(state, scope, statement, &bound).await;
        out.push_str(&format!("PT003-O3 gate {name} = {} (raw {view})\n", pt003_verdict(&view, &format!("pt003_gate_{name}"))));
    }
    // O3b: the same terms through a custom function call in a WHERE (legacy executor path).
    for name in ["t_true", "t_exists", "t_not_exists", "t_exists_account", "t_deref", "t_las", "copy_full", "copy_no_exists"] {
        let statement = pt003_leak(format!("SELECT VALUE id FROM pt003_open WHERE fn::pt003_{name}($this);"));
        let view = pt003_record_user_view(state, scope, statement, &bound).await;
        out.push_str(&format!("PT003-O3b where_fn {name} = {} (raw {view})\n", pt003_verdict(&view, "pt003_open")));
    }
    out
}

async fn pt003_probe_run(compute_only: bool, ddl: Option<String>) -> (String, String) {
    let MountedIndexAccountFixture { _binding, backend, state, router, headers, session } =
        mounted_index_account_fixture(compute_only).await;
    let credentials = crate::api::authority::authenticated_session_credentials(&state, &headers)
        .await
        .unwrap_or_else(|_| panic!("pt003 probe account authentication failed"));
    let (scope, _) = pt003_scope_and_bindings!(credentials, session);
    let mut report = String::new();
    if let Some(ddl) = ddl { report.push_str(&pt003_root_ddl(&state, ddl, 1).await); }
    let observed = state
        .surreal
        .probe_first_owner_workspace_receipt_projection(
            &credentials.context, &scope, crate::storage::NewWorkspace { name: "Owned indexed workspace".to_owned() },
        )
        .await;
    let outcome = match observed {
        Ok(outcome) => {
            let _ = request(&router, "POST", "/authority/logout", &headers, Value::Null).await;
            drop(router);
            drop(state);
            assert!(backend.close_and_remove().await.is_ok(), "pt003 probe fixture cleanup unobserved");
            outcome.to_owned()
        }
        Err(error) => {
            drop(router);
            drop(state);
            assert!(backend.close_and_preserve().await.is_ok(), "pt003 probe fixture close unobserved");
            format!("PROBE_ERR {error:?}")
        }
    };
    report.push_str(&format!("outcome = {outcome}\n"));
    (outcome, report)
}

fn pt003_strict_function_text() -> String {
    let schema = include_str!("../storage/surreal/schema.surql");
    let start = schema.find("DEFINE FUNCTION OVERWRITE fn::wp012_first_owner_workspace_receipt(")
        .expect("pt003 strict function definition not found");
    let rest = &schema[start..];
    let end = rest.find("\n};").expect("pt003 strict function end not found") + 3;
    rest[..end].to_owned()
}

#[tokio::test]
async fn pt003_patched_strict_probe() {
    let production = pt003_strict_function_text();
    let conjunct = "record::exists($row.setup_receipt) AND ";
    assert_eq!(production.matches(conjunct).count(), 1, "pt003 conjunct occurrence count");
    let patched = production.replacen(conjunct, "", 1);
    let (outcome, report) = pt003_probe_run(false, Some(patched.clone())).await;
    let report = format!(
        "PT003-O4 patched strict probe (conjunct removed; production text {} bytes, patched {} bytes)\n{report}",
        production.len(), patched.len()
    );
    pt003_write_report("pt003-patched.txt", &report);
    assert_eq!(outcome, "one_visible", "pt003 patched strict probe");
}

fn pt003_clip(text: String) -> String {
    let text = text.replace('\n', " ");
    if text.chars().count() <= 900 { text } else { format!("{}...[clipped]", text.chars().take(900).collect::<String>()) }
}

fn pt003_chain(terms: usize, subject: &str) -> String {
    vec![format!("{subject}.version = 1"); terms].join(" AND ")
}

/// Defines one function in its own admin call, so a parse error hides nothing else.
async fn pt003_depth_define(state: &AppState, label: &str, fname: &str, body: String) -> String {
    let ddl = format!("DEFINE FUNCTION OVERWRITE fn::{fname}($row: object) {{ RETURN {body}; }};");
    format!("{label} define = {}\n", pt003_clip(pt003_root_ddl(state, ddl, 1).await))
}

/// (a) root projection, (b) root WHERE, (c) record-user SELECT permission gate calling the function.
async fn pt003_depth_eval(
    state: &AppState, scope: &crate::storage::surreal::resource_authority::RecordUserScope,
    bindings: &Value, label: &str, fname: &str,
) -> String {
    let projection = format!("SELECT fn::{fname}($this) AS r FROM first_owner_administration_origins:primary;");
    let in_where = format!("SELECT VALUE id FROM first_owner_administration_origins:primary WHERE fn::{fname}($this);");
    let mut out = String::new();
    out.push_str(&format!("{label} (a) root projection = {}\n", pt003_clip(pt003_root_owned(state, projection, bindings).await)));
    out.push_str(&format!("{label} (b) root where_fn = {}\n", pt003_clip(pt003_root_owned(state, in_where, bindings).await)));
    let gate_ddl = format!(
        "LET $o = (SELECT * OMIT id FROM ONLY first_owner_administration_origins:primary);
         DEFINE TABLE OVERWRITE pt003_gate_{label} SCHEMALESS PERMISSIONS FOR select WHERE fn::{fname}($this) FOR create, update, delete NONE;
         CREATE pt003_gate_{label}:1 CONTENT $o;"
    );
    out.push_str(&format!("{label} (c) gate define = {}\n", pt003_clip(pt003_root_ddl(state, gate_ddl, 3).await)));
    let gate = pt003_leak(format!("SELECT VALUE id FROM pt003_gate_{label};"));
    let view = pt003_record_user_view(state, scope, gate, bindings).await;
    let verdict = pt003_verdict(&view, &format!("pt003_gate_{label}"));
    out.push_str(&format!("{label} (c) record_user gate = {verdict} (raw {})\n", pt003_clip(view)));
    out
}

#[tokio::test]
async fn pt003_compute_only_depth_probe() {
    let MountedIndexAccountFixture { _binding, backend, state, router, headers, session } =
        mounted_index_account_fixture(true).await;
    let credentials = crate::api::authority::authenticated_session_credentials(&state, &headers)
        .await
        .unwrap_or_else(|_| panic!("pt003 depth probe account authentication failed"));
    let (scope, bindings) = pt003_scope_and_bindings!(credentials, session);
    let mut report = String::from("PT003-O5 compute-only depth probe (R172-02d, one DDL call per function)\n");
    for n in [20usize, 40, 60, 77, 80, 100, 110, 118] {
        let label = format!("chain_{n}");
        let fname = format!("pt003_chain_{n}");
        report.push_str(&pt003_depth_define(&state, &label, &fname, pt003_chain(n, "$row")).await);
        report.push_str(&pt003_depth_eval(&state, &scope, &bindings, &label, &fname).await);
        let inline = format!(
            "LET $o = (SELECT * OMIT id FROM ONLY first_owner_administration_origins:primary);
             DEFINE TABLE OVERWRITE pt003_inline_{n} SCHEMALESS PERMISSIONS FOR select WHERE {} FOR create, update, delete NONE;
             CREATE pt003_inline_{n}:1 CONTENT $o;", pt003_chain(n, "$this")
        );
        report.push_str(&format!("{label} (d) inline gate define = {}\n", pt003_clip(pt003_root_ddl(&state, inline, 3).await)));
        let gate = pt003_leak(format!("SELECT VALUE id FROM pt003_inline_{n};"));
        let view = pt003_record_user_view(&state, &scope, gate, &bindings).await;
        let verdict = pt003_verdict(&view, "pt003_inline_");
        report.push_str(&format!("{label} (d) record_user inline gate = {verdict} (raw {})\n", pt003_clip(view)));
    }
    // Mirror of the strict shape: 77 terms, the 5th term calls a 28-term chain.
    report.push_str(&pt003_depth_define(&state, "sub_28", "pt003_sub_28", pt003_chain(28, "$row")).await);
    let mirror_body = format!("{} AND fn::pt003_sub_28($row) AND {}", pt003_chain(4, "$row"), pt003_chain(72, "$row"));
    report.push_str(&pt003_depth_define(&state, "mirror_77_with_28", "pt003_mirror_77_with_28", mirror_body).await);
    report.push_str(&pt003_depth_eval(&state, &scope, &bindings, "mirror_77_with_28", "pt003_mirror_77_with_28").await);
    // Textual copy of the production strict function under another name, in its own call.
    let strict_copy = pt003_strict_function_text()
        .replacen("fn::wp012_first_owner_workspace_receipt(", "fn::pt003_mirror_strict(", 1);
    report.push_str(&format!("mirror_strict define = {}\n", pt003_clip(pt003_root_ddl(&state, strict_copy, 1).await)));
    let swap = |text: &str| text.replace("fn::wp012_first_owner_workspace_receipt", "fn::pt003_mirror_strict");
    report.push_str(&format!("mirror_strict root projection = {}\n", pt003_clip(pt003_root_owned(&state, swap(PT003_STRICT_PROJECTION), &bindings).await)));
    report.push_str(&format!("mirror_strict root where_fn = {}\n", pt003_clip(pt003_root_owned(&state, swap(PT003_STRICT_WHERE), &bindings).await)));
    report.push_str(&format!("mirror_strict record_user projection = {}\n",
        pt003_clip(pt003_record_user_view(&state, &scope, pt003_leak(swap(PT003_STRICT_PROJECTION)), &bindings).await)));
    report.push_str(&format!("mirror_strict record_user where_fn = {}\n",
        pt003_clip(pt003_record_user_view(&state, &scope, pt003_leak(swap(PT003_STRICT_WHERE)), &bindings).await)));
    // Baseline: the production strict function evaluated the way the compute-only rollback test evaluates it.
    let (baseline_scope, _) = pt003_scope_and_bindings!(credentials, session);
    let baseline = state.surreal
        .probe_first_owner_workspace_receipt_projection(
            &credentials.context, &baseline_scope, crate::storage::NewWorkspace { name: "Owned indexed workspace".to_owned() },
        )
        .await;
    let baseline_ok = baseline.is_ok();
    report.push_str(&format!("BASELINE production strict function via probe_first_owner_workspace_receipt_projection = {}\n",
        pt003_clip(format!("{baseline:?}"))));
    drop(router);
    drop(state);
    let closed = if baseline_ok { backend.close_and_remove().await.is_ok() } else { backend.close_and_preserve().await.is_ok() };
    report.push_str(&format!("fixture close ok = {closed}\n"));
    pt003_write_report("pt003-depth.txt", &report);
}

#[derive(PartialEq, SurrealValue)]
struct MountedFirstOwnerReceipt {
    setup_operation_id: String,
    operation_id: String,
    workspace: RecordId,
    resource: RecordId,
    creator_grant: RecordId,
}

async fn read_mounted_fixture_rows<R: SurrealValue + Send + 'static>(
    storage: &crate::storage::surreal::SurrealStorage,
    statement: &'static str,
    bindings: &Value,
) -> Result<Vec<R>, crate::storage::surreal::SurrealStorageError> {
    let bindings = bindings.clone();
    storage.with_data_operation(move |database| {
        Box::pin(async move { database.query_values(statement, bindings).await })
    }).await
}

async fn read_mounted_first_owner_receipt(
    storage: &crate::storage::surreal::SurrealStorage,
    bindings: &Value,
) -> Result<MountedFirstOwnerReceipt, &'static str> {
    let rows: Vec<MountedFirstOwnerReceipt> = read_mounted_fixture_rows(storage, r#"
        SELECT setup_operation_id, first_workspace_operation_id AS operation_id,
            first_workspace AS workspace, first_workspace_resource AS resource,
            first_workspace_grant AS creator_grant
        FROM first_owner_administration_origins:primary
        WHERE account = type::record('local_accounts', $proof_account)
          AND operator_principal = type::record('principals', $proof_principal)
          AND space = type::record('access_spaces', $proof_space)
          AND first_workspace_state = 'created' AND first_workspace_receipt_nonce = 1
          AND first_workspace_operation_id = $proof_workspace
          AND first_workspace = type::record('workspaces', $proof_workspace)
          AND first_workspace_session = type::record('authenticated_sessions', $proof_session)
          AND first_workspace_channel_hash = $proof_channel AND first_workspace_name = $proof_name
          AND first_workspace_original.operation_id = first_workspace_operation_id
          AND first_workspace_original.account = account
          AND first_workspace_original.principal = operator_principal
          AND first_workspace_original.space = space
          AND first_workspace_original.account_session = first_workspace_session
          AND first_workspace_original.workspace = first_workspace
          AND first_workspace_original.external = $proof_workspace
          AND first_workspace_original.resource = first_workspace_resource
          AND first_workspace_original.workspace_grant = first_workspace_grant
          AND first_workspace_original.channel_hash = $proof_channel
          AND first_workspace_original.name = $proof_name
          AND first_workspace_original.first_owner_workspace_receipt_read = false
          AND record::exists(first_workspace) AND first_workspace.name = $proof_name
          AND first_workspace.created_in_session_id = first_workspace_session
          AND record::exists(first_workspace_resource)
          AND first_workspace_resource.resource_kind = 'workspace'
          AND first_workspace_resource.external_resource_id = $proof_workspace
          AND first_workspace_resource.owner_account_id = account
          AND first_workspace_resource.created_by_principal_id = operator_principal
          AND first_workspace_resource.access_space_id = space
          AND first_workspace_resource.created_in_session_id = first_workspace_session
          AND first_workspace_resource.lifecycle_state = 'active'
          AND first_workspace_resource.creator_grant_id = first_workspace_grant
          AND record::exists(first_workspace_grant)
          AND first_workspace_grant.resource_id = first_workspace_resource
          AND first_workspace_grant.account_id = account
          AND first_workspace_grant.principal_id = operator_principal
          AND first_workspace_grant.access_space_id = space
          AND first_workspace_grant.status = 'active'
          AND first_workspace_grant.revoked_at = NONE AND first_workspace_grant.expires_at = NONE
          AND first_workspace_grant.grant_version = 1
          AND first_workspace_grant.actions = ['create', 'read', 'update', 'delete']
          AND first_workspace_grant.capability_ids CONTAINS 'fs.write';
    "#, bindings).await.map_err(|_| "receipt_readback")?;
    let [receipt]: [_; 1] = rows.try_into().map_err(|_| "receipt_correlation")?;
    Ok(receipt)
}

#[tokio::test]
async fn mounted_first_owner_workspace_create_persists_original_receipt_and_grant() {
    const NAME: &str = "Mounted first owner receipt proof";
    let binding = Binding::new();
    let mut headers = index_account_channel_headers(&binding);
    let backend = crate::storage::tests::embedded_test_backend().await
        .unwrap_or_else(|_| panic!("mounted first-owner proof phase=backend_open"));
    let mut mounted = None;
    let probe = AssertUnwindSafe(async {
        let (_, router) = mounted.insert(mount_index_account(&backend));
        let session = issue_index_account_session(router, &mut headers).await?;
        let channel = crate::api::stage::capture_channel_binding(&headers)
            .map_err(|_| "fixture_channel_binding")?;
        let mut bindings = json!({
            "proof_account": session["account_id"].as_str().ok_or("fixture_account")?,
            "proof_principal": session["principal_id"].as_str().ok_or("fixture_principal")?,
            "proof_space": session["access_space_id"].as_str().ok_or("fixture_space")?,
            "proof_session": session["session_id"].as_str().ok_or("fixture_session")?,
            "proof_channel": channel.binding_hash,
            "proof_name": NAME,
        });
        let rows: Vec<String> = read_mounted_fixture_rows(&backend.storage, r#"
            SELECT VALUE setup_operation_id FROM first_owner_administration_origins:primary
            WHERE account = type::record('local_accounts', $proof_account)
              AND operator_principal = type::record('principals', $proof_principal)
              AND space = type::record('access_spaces', $proof_space)
              AND first_workspace_state = 'unclaimed' AND first_workspace_receipt_nonce = 0
              AND first_workspace_original = NONE;
        "#, &bindings).await.map_err(|_| "origin_before_post_readback")?;
        let [setup_id]: [_; 1] = rows.try_into().map_err(|_| "origin_before_post_correlation")?;

        let (status, workspace) = request(router, "POST", "/workspaces", &headers, json!({"name": NAME})).await;
        if status != StatusCode::CREATED {
            return Err("workspace_post_not_created");
        }
        bindings["proof_workspace"] = json!(workspace["id"].as_str().ok_or("workspace_response_id")?);
        let receipt = read_mounted_first_owner_receipt(&backend.storage, &bindings).await?;
        if receipt.setup_operation_id != setup_id {
            return Err("setup_origin_identity_changed");
        }
        Ok::<_, &'static str>((bindings, receipt))
    }).catch_unwind().await;

    let logout = if let Some((_, router)) = mounted.as_ref().filter(|_| headers.contains_key("x-hsk-session-token")) {
        let result = AssertUnwindSafe(mounted_response(router, "POST", "/authority/logout", &headers, Value::Null))
            .catch_unwind().await;
        if matches!(result, Ok(response) if response.status() == StatusCode::OK) {
            "checked"
        } else {
            "unconfirmed"
        }
    } else {
        "credential_unavailable"
    };
    drop(mounted);
    let drained = matches!(
        AssertUnwindSafe(backend.storage.drain_member_password_login_operations()).catch_unwind().await,
        Ok(Ok(()))
    );

    let phase = match probe {
        Ok(Ok(_)) if logout != "checked" => "logout_unconfirmed",
        Ok(Ok(_)) if !drained => "owned_drain_unconfirmed",
        Ok(Ok((bindings, receipt))) => {
            let durable = AssertUnwindSafe(async {
                let reopened = backend.reopen_storage().await.map_err(|_| "store_reopen_unconfirmed")?;
                let durable = read_mounted_first_owner_receipt(&reopened, &bindings).await
                    .map_err(|_| "reopened_receipt_readback")?;
                if durable != receipt {
                    return Err("reopened_receipt_identity_changed");
                }
                Ok::<(), &'static str>(())
            }).catch_unwind().await;
            match durable {
                Ok(Ok(())) => {
                    assert!(backend.close_and_remove().await.is_ok(), "mounted first-owner proof phase=store_cleanup");
                    return;
                }
                Ok(Err(phase)) => phase,
                Err(_) => "reopened_receipt_helper_panic",
            }
        }
        Ok(Err(phase)) => phase,
        Err(_) => "fixture_helper_panic",
    };
    let closure = if backend.close_and_preserve().await.is_ok() {
        "closed_store_preserved"
    } else {
        "closure_unconfirmed"
    };
    panic!("mounted first-owner proof failed phase={phase} logout={logout} drained={drained} closure={closure}");
}

#[tokio::test]
async fn mounted_index_account_source_witness_revocation_and_workspace_delete() {
    let MountedIndexAccountFixture { _binding, backend: _backend, state, router, headers, session } =
        mounted_index_account_fixture(false).await;
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

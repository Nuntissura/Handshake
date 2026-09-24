//! Shared authenticated identity and exact protected-resource authorization.
//!
//! Native process binding proves only the local transport channel. Persisted account, principal,
//! session, access-space, capability, and ResourceGrant state are resolved independently by the
//! Surreal-backed ResourceBroker before any protected product query is allowed.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[cfg(test)]
use sha2::{Digest, Sha256};

use crate::kernel::KernelActor;
use crate::storage::surreal::resource_authority::{
    AuthorizationRequest, IssuedSession, ProvisionedPrincipal, RecordUserScope, ResourceAction,
    ResourceGrantSpec, ResourceKind,
};
use crate::AppState;

const HSK_HEADER_SESSION_TOKEN: &str = "x-hsk-session-token";
const LOCAL_SESSION_TTL_HOURS: i64 = 12;
#[cfg(feature = "os-keychain")]
static SESSION_VAULT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(feature = "os-keychain")]
async fn run_session_vault_operation<T: Send + 'static>(
    operation: impl FnOnce() -> T + Send + 'static,
) -> Result<T, (StatusCode, Json<Value>)> {
    let guard = SESSION_VAULT_LOCK.lock().await;
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        operation()
    })
    .await
    .map_err(|error| {
        #[cfg(test)]
        eprintln!("LOCAL_ACCOUNT_TEST_FAILURE phase=vault_worker error={error}");
        let _ = error;
        constant_denial()
    })
}

#[cfg(test)]
struct AccountLifecycleProbe {
    path: std::path::PathBuf,
    phase: &'static str,
    entered: tokio::sync::oneshot::Sender<()>,
    release: tokio::sync::oneshot::Receiver<()>,
    fail: bool,
}
#[cfg(test)]
static ACCOUNT_LIFECYCLE_PROBE: std::sync::Mutex<Option<AccountLifecycleProbe>> =
    std::sync::Mutex::new(None);
#[cfg(test)]
async fn account_lifecycle_probe(
    state: &AppState,
    phase: &'static str,
) -> Result<(), (StatusCode, Json<Value>)> {
    let probe = {
        let mut slot = ACCOUNT_LIFECYCLE_PROBE.lock().unwrap();
        if slot.as_ref().is_some_and(|probe| {
            probe.path == state.surreal.config().path() && probe.phase == phase
        }) {
            slot.take()
        } else {
            None
        }
    };
    if let Some(probe) = probe {
        let _ = probe.entered.send(());
        let _ = probe.release.await;
        if probe.fail {
            return Err(constant_denial());
        }
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub(crate) struct AuthorizedResourceContext {
    pub(crate) decision_id: String,
    pub(crate) account_id: String,
    pub(crate) principal_id: String,
    pub(crate) session_id: String,
    pub(crate) access_space_id: String,
    pub(crate) resource_id: String,
    pub(crate) actor_kind: String,
    pub(crate) actor_id: String,
    pub(crate) capability_profile_id: String,
    pub(crate) capability_id: String,
    pub(crate) delegation_chain: Vec<String>,
    pub(crate) record_user_scope: RecordUserScope,
}

#[derive(Clone, Debug)]
pub(crate) struct ReconciliationAuthority {
    session: IssuedSession,
    pub(crate) record_user_scope: RecordUserScope,
}

pub(crate) async fn reconciliation_authority(
    state: &AppState,
    capability_id: &'static str,
) -> Result<ReconciliationAuthority, String> {
    let principal = state
        .surreal
        .issue_reconciliation_session()
        .await
        .map_err(|error| error.to_string())?;
    let decision = state
        .surreal
        .authorize_protected_resource(AuthorizationRequest {
            session_token: principal.session.token.clone(),
            channel_binding_hash: None,
            capability_id: capability_id.to_owned(),
            resource_kind: ResourceKind::ReconciliationQueue,
            external_resource_id: "mt109-protected-reconciliation".to_owned(),
            action: ResourceAction::Reconcile,
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(ReconciliationAuthority {
        record_user_scope: RecordUserScope {
            grant_id: Some(decision.grant_id.clone()),
            workspace_id: None,
            session_token: principal.session.token.clone(),
            channel_binding_hash: None,
            resource_id: decision.resource_id,
            session_id: decision.session_id,
            capability_id: capability_id.to_owned(),
            action: ResourceAction::Reconcile,
        },
        session: principal.session,
    })
}

pub(crate) async fn authorize_reconciliation_workspace(
    state: &AppState,
    authority: &ReconciliationAuthority,
    workspace_id: &str,
    capability_id: &'static str,
) -> Result<RecordUserScope, String> {
    state
        .surreal
        .authorize_protected_resource(AuthorizationRequest {
            session_token: authority.session.token.clone(),
            channel_binding_hash: None,
            capability_id: capability_id.to_owned(),
            resource_kind: ResourceKind::ReconciliationQueue,
            external_resource_id: format!(
                "{}:{workspace_id}",
                crate::storage::surreal::resource_authority::RECONCILIATION_QUEUE_ID
            ),
            action: ResourceAction::Reconcile,
        })
        .await
        .map(|decision| RecordUserScope {
            grant_id: Some(decision.grant_id.clone()),
            workspace_id: Some(workspace_id.to_owned()),
            session_token: authority.session.token.clone(),
            channel_binding_hash: None,
            resource_id: decision.resource_id,
            session_id: decision.session_id,
            capability_id: capability_id.to_owned(),
            action: ResourceAction::Reconcile,
        })
        .map_err(|error| error.to_string())
}

impl AuthorizedResourceContext {
    pub(crate) fn capture_context(&self) -> crate::api::stage::CaptureContext {
        crate::api::stage::CaptureContext {
            actor_kind: self.actor_kind.clone(),
            actor_id: self.actor_id.clone(),
            limiter_principal: self.principal_id.clone(),
            actor: KernelActor::Operator(self.actor_id.clone()),
            kernel_task_run_id: format!("authority-task:{}", self.decision_id),
            session_run_id: self.session_id.clone(),
            // A channel credential is never copied into the authenticated identity context.
            binding_token: String::new(),
        }
    }
}

#[derive(Serialize)]
struct LocalSessionResponse {
    schema_version: &'static str,
    session_token: String,
    account_id: String,
    principal_id: String,
    session_id: String,
    access_space_id: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalSessionExchange {
    account_id: String,
    principal_id: String,
    access_space_id: String,
    authentication_token: String,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/authority/setup", get(setup_status).post(setup_owner))
        .route("/authority/login", post(login_owner))
        .route(
            "/authority/session",
            get(current_session).post(exchange_local_session),
        )
        .route("/authority/logout", post(logout))
        .with_state(state)
}

#[cfg(all(test, feature = "duckdb-flight-recorder", feature = "os-keychain"))]
mod local_account_http_tests {
    use super::*;
    use crate::api::MountedRequestExt;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn cancelled_vault_request_retains_serialization_until_blocking_exit() {
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, blocked) = std::sync::mpsc::channel();
        let (finished, exited) = tokio::sync::oneshot::channel();
        let request = tokio::spawn(run_session_vault_operation(move || {
            entered.send(()).unwrap();
            blocked.recv().unwrap();
            let _ = finished.send(());
        }));
        started.await.unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert!(SESSION_VAULT_LOCK.try_lock().is_err());
        release.send(()).unwrap();
        exited.await.unwrap();
        let _guard =
            tokio::time::timeout(std::time::Duration::from_secs(2), SESSION_VAULT_LOCK.lock())
                .await
                .expect("blocking exit releases the vault lock");
    }

    fn lifecycle_pause(
        state: &AppState,
        phase: &'static str,
        fail: bool,
    ) -> (
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (entered, observed) = tokio::sync::oneshot::channel();
        let (release, wait) = tokio::sync::oneshot::channel();
        let previous = ACCOUNT_LIFECYCLE_PROBE
            .lock()
            .unwrap()
            .replace(AccountLifecycleProbe {
                path: state.surreal.config().path().to_path_buf(),
                phase,
                entered,
                release: wait,
                fail,
            });
        assert!(previous.is_none());
        (observed, release)
    }

    #[tokio::test]
    async fn local_account_handler_cancellation_and_vault_cleanup_retry() {
        use crate::model_runtime::cloud::secrets_vault::{OsKeychainSecretsVault, SecretsVault};
        let binding = BindingFile::new();
        let backend = crate::storage::tests::embedded_test_backend()
            .await
            .unwrap();
        let recorder = Arc::new(
            crate::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7).unwrap(),
        );
        let state = AppState {
            storage: backend.database.clone(),
            surreal: backend.storage.clone(),
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(crate::llm::ollama::InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(crate::capabilities::CapabilityRegistry::new()),
            session_registry: Arc::new(crate::workflows::SessionRegistry::new(
                crate::workflows::SessionSchedulerConfig::default(),
            )),
        };
        let router = routes(state.clone());
        let config = state.surreal.config();
        let installation = hex::encode(Sha256::digest(
            serde_json::to_vec(&(
                config.path().to_string_lossy(),
                config.namespace(),
                config.database(),
            ))
            .unwrap(),
        ));
        let vault = OsKeychainSecretsVault::new(format!("handshake-local-accounts-{installation}"));
        let password = json!({"account_name":"Owner","password":"correct horse battery staple"});
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                password.clone()
            )
            .await
            .0,
            StatusCode::OK
        );
        for (index, phase) in ["post_session", "post_vault"].into_iter().enumerate() {
            let (_, credential) = request(
                &router,
                "POST",
                "/authority/login",
                Some(&binding.channel),
                None,
                password.clone(),
            )
            .await;
            let principal_id = credential["principal_id"].as_str().unwrap().to_owned();
            let exchange = json!({"account_id":credential["account_id"],"principal_id":credential["principal_id"],
                "access_space_id":credential["access_space_id"],"authentication_token":credential["token"]});
            let (observed, release) = lifecycle_pause(&state, phase, false);
            let app = router.clone();
            let channel = binding.channel.clone();
            let handler = tokio::spawn(async move {
                request(
                    &app,
                    "POST",
                    "/authority/session",
                    Some(&channel),
                    None,
                    exchange,
                )
                .await
            });
            tokio::time::timeout(std::time::Duration::from_secs(10), observed)
                .await
                .unwrap()
                .unwrap();
            handler.abort();
            assert!(handler.await.unwrap_err().is_cancelled());
            release.send(()).unwrap();
            let id = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    let mut response = state.surreal.test_admin_query(format!(
                        "SELECT VALUE record::id(id) FROM authenticated_sessions WHERE revoked_at = NONE AND principal_id = type::record('principals', '{principal_id}');"
                    )).await.unwrap();
                    let ids: Vec<String> = response.take(0).unwrap();
                    let mut audit = state.surreal.test_admin_query("RETURN array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_LOGIN');".into()).await.unwrap();
                    if audit.take::<Option<i64>>(0).unwrap() == Some((index + 1) as i64) && ids.len() == 1 {
                        if vault.get(&ids[0]).is_ok() { break ids[0].clone(); }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
            }).await.unwrap();
            let token = vault.get(&id).unwrap();
            assert_eq!(
                request(
                    &router,
                    "GET",
                    "/authority/session",
                    Some(&binding.channel),
                    Some(&token),
                    Value::Null
                )
                .await
                .0,
                StatusCode::OK
            );
            let logout_phase = if index == 0 {
                "post_revoke"
            } else {
                "vault_delete"
            };
            let (observed, release) = lifecycle_pause(&state, logout_phase, index == 1);
            let app = router.clone();
            let channel = binding.channel.clone();
            let bearer = token.clone();
            let handler = tokio::spawn(async move {
                request(
                    &app,
                    "POST",
                    "/authority/logout",
                    Some(&channel),
                    Some(&bearer),
                    Value::Null,
                )
                .await
            });
            tokio::time::timeout(std::time::Duration::from_secs(10), observed)
                .await
                .unwrap()
                .unwrap();
            if index == 0 {
                handler.abort();
            }
            release.send(()).unwrap();
            if index == 0 {
                assert!(handler.await.unwrap_err().is_cancelled());
            } else {
                assert_eq!(
                    handler.await.unwrap().0,
                    StatusCode::OK,
                    "owned retry repairs injected transient vault deletion failure"
                );
            }
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    match vault.get(&id) {
                        Ok(_) => tokio::time::sleep(std::time::Duration::from_millis(20)).await,
                        Err(crate::model_runtime::cloud::secrets_vault::SecretsVaultError::NoSecretForLane(missing)) if missing == id => break,
                        Err(error) => panic!("vault cleanup lookup failed: {error}"),
                    }
                }
            }).await.unwrap();
            assert_eq!(
                request(
                    &router,
                    "GET",
                    "/authority/session",
                    Some(&binding.channel),
                    Some(&token),
                    Value::Null
                )
                .await
                .0,
                StatusCode::FORBIDDEN
            );
            // Recreate only the retired vault residue to exercise the recovery boundary itself.
            vault.put(&id, token.clone()).unwrap();
            assert_eq!(
                request(
                    &router,
                    "POST",
                    "/authority/logout",
                    Some(&"f4".repeat(32)),
                    Some(&token),
                    Value::Null
                )
                .await
                .0,
                StatusCode::FORBIDDEN
            );
            assert!(vault.get(&id).is_ok());
            assert_eq!(
                request(
                    &router,
                    "POST",
                    "/authority/logout",
                    Some(&binding.channel),
                    Some(&"ab".repeat(32)),
                    Value::Null
                )
                .await
                .0,
                StatusCode::FORBIDDEN
            );
            assert!(vault.get(&id).is_ok());
            assert_eq!(
                request(
                    &router,
                    "POST",
                    "/authority/logout",
                    Some(&binding.channel),
                    Some(&token),
                    Value::Null
                )
                .await
                .0,
                StatusCode::FORBIDDEN
            );
            assert!(
                matches!(vault.get(&id), Err(crate::model_runtime::cloud::secrets_vault::SecretsVaultError::NoSecretForLane(missing)) if missing == id)
            );
        }
        let mut audit = state.surreal.test_admin_query("RETURN array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_LOGOUT');".into()).await.unwrap();
        assert_eq!(
            audit.take::<Option<i64>>(0).unwrap(),
            Some(2),
            "recovery never creates a second logout event"
        );
        state.surreal.shutdown().await.unwrap();
    }
    struct BindingFile {
        _lock: std::sync::MutexGuard<'static, ()>,
        _directory: tempfile::TempDir,
        previous: Option<std::ffi::OsString>,
        channel: String,
    }
    impl BindingFile {
        fn new() -> Self {
            let lock = crate::api::stage::NATIVE_BINDING_ENV_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let directory = tempfile::tempdir().unwrap();
            let channel = "e3".repeat(32);
            let path = directory.path().join("account-binding.json");
            std::fs::write(
                &path,
                serde_json::to_vec(&crate::api::stage::current_process_native_binding(&channel))
                    .unwrap(),
            )
            .unwrap();
            let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
            Self {
                _lock: lock,
                _directory: directory,
                previous,
                channel,
            }
        }
    }
    impl Drop for BindingFile {
        fn drop(&mut self) {
            if let Some(value) = &self.previous {
                std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", value);
            } else {
                std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE");
            }
        }
    }
    async fn request(
        router: &Router,
        method: &str,
        path: &str,
        channel: Option<&str>,
        session: Option<&str>,
        body: Value,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json");
        if let Some(channel) = channel {
            request = request.header("x-hsk-channel-binding-token", channel);
        }
        if let Some(session) = session {
            request = request.header(HSK_HEADER_SESSION_TOKEN, session);
        }
        let response = router
            .clone()
            .oneshot(
                request
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn incomplete_owner_setup_recovery_requires_password_and_preserves_one_time_gate() {
        let binding = BindingFile::new();
        let backend = crate::storage::tests::embedded_test_backend()
            .await
            .unwrap();
        let recorder = Arc::new(
            crate::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7).unwrap(),
        );
        let state = AppState {
            storage: backend.database.clone(),
            surreal: backend.storage.clone(),
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(crate::llm::ollama::InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(crate::capabilities::CapabilityRegistry::new()),
            session_registry: Arc::new(crate::workflows::SessionRegistry::new(
                crate::workflows::SessionSchedulerConfig::default(),
            )),
        };
        let router = routes(state.clone());
        let password =
            json!({"account_name":"Recovery Owner","password":"correct horse battery staple"});
        let capabilities = state
            .capability_registry
            .profile_by_id("Operator")
            .expect("Operator profile")
            .allowed
            .clone();
        let verifier = crate::storage::surreal::local_accounts::new_password_verifier(
            "correct horse battery staple".to_owned(),
        )
        .await
        .expect("test password verifier");
        state
            .surreal
            .setup_local_owner("Recovery Owner", verifier, capabilities)
            .await
            .expect("seed owner marker without reconciliation service");
        assert!(
            !state
                .surreal
                .reconciliation_principal_is_provisioned()
                .await
                .unwrap(),
            "seeded owner setup must be incomplete before recovery"
        );

        let denial = json!({"error":"HSK-403-PROTECTED-RESOURCE"});
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                json!({"account_name":"Recovery Owner","password":"wrong password value"}),
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone()),
            "wrong password must not repair a partial setup"
        );
        assert!(
            !state
                .surreal
                .reconciliation_principal_is_provisioned()
                .await
                .unwrap(),
            "wrong recovery password must leave the service identity absent"
        );

        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                password.clone(),
            )
            .await
            .0,
            StatusCode::OK,
            "the matching owner password repairs only the missing service setup"
        );
        assert!(
            state
                .surreal
                .reconciliation_principal_is_provisioned()
                .await
                .unwrap(),
            "recovery must create the canonical reconciliation authority"
        );
        let mut rows = state
            .surreal
            .test_admin_query(
                "RETURN { principal: array::len(SELECT VALUE id FROM principals WHERE principal_key = 'mt109-reconciliation-service-principal' AND principal_kind = 'service_identity' AND status = 'enabled'), space: array::len(SELECT VALUE id FROM access_spaces WHERE space_key = 'mt109-reconciliation-space' AND status = 'active'), root: array::len(SELECT VALUE id FROM protected_resources WHERE resource_kind = 'reconciliation_queue' AND external_resource_id = 'mt109-protected-reconciliation' AND lifecycle_state = 'active'), root_grant: array::len(SELECT VALUE id FROM resource_grants WHERE resource_id.resource_kind = 'reconciliation_queue' AND resource_id.external_resource_id = 'mt109-protected-reconciliation' AND actions = ['reconcile'] AND capability_ids = ['fr.ingest.native_editor','memory.commit'] AND status = 'active' AND revoked_at = NONE AND expires_at = NONE) };".to_owned(),
            )
            .await
            .unwrap();
        assert_eq!(
            rows.take::<Option<Value>>(0).unwrap(),
            Some(json!({"principal": 1, "space": 1, "root": 1, "root_grant": 1})),
            "recovery must establish the exact service principal, root queue, and root grant"
        );
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                password,
            )
            .await,
            (StatusCode::FORBIDDEN, denial),
            "completed setup remains one-time after recovery"
        );
        state.surreal.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn local_account_http_setup_login_vault_logout_and_constant_denials() {
        let binding = BindingFile::new();
        let backend = crate::storage::tests::embedded_test_backend()
            .await
            .unwrap();
        let recorder = Arc::new(
            crate::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7).unwrap(),
        );
        let state = AppState {
            storage: backend.database.clone(),
            surreal: backend.storage.clone(),
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(crate::llm::ollama::InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(crate::capabilities::CapabilityRegistry::new()),
            session_registry: Arc::new(crate::workflows::SessionRegistry::new(
                crate::workflows::SessionSchedulerConfig::default(),
            )),
        };
        let router = routes(state.clone());
        let denial = json!({"error":"HSK-403-PROTECTED-RESOURCE"});
        assert_eq!(
            request(
                &router,
                "GET",
                "/authority/session",
                None,
                None,
                Value::Null
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone())
        );
        let mut counts = state.surreal.test_admin_query("RETURN array::len(SELECT VALUE id FROM kernel_event_ledger WHERE source_component = 'local_account_authority');".into()).await.unwrap();
        assert_eq!(counts.take::<Option<i64>>(0).unwrap(), Some(0));
        let password = json!({"account_name":"Owner","password":"correct horse battery staple"});
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                password.clone()
            )
            .await
            .0,
            StatusCode::OK
        );
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                password.clone()
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone())
        );
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/login",
                Some(&binding.channel),
                None,
                json!({"account_name":"Owner","password":"wrong password value"})
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone())
        );
        let (status, credential) = request(
            &router,
            "POST",
            "/authority/login",
            Some(&binding.channel),
            None,
            password,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let exchange = json!({"account_id":credential["account_id"],"principal_id":credential["principal_id"],
            "access_space_id":credential["access_space_id"],"authentication_token":credential["token"]});
        let (status, session) = request(
            &router,
            "POST",
            "/authority/session",
            Some(&binding.channel),
            None,
            exchange.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let token = session["session_token"].as_str().unwrap();
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/session",
                Some(&binding.channel),
                None,
                exchange
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone())
        );
        let (status, current) = request(
            &router,
            "GET",
            "/authority/session",
            Some(&binding.channel),
            Some(token),
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(current["account_id"], credential["account_id"]);
        assert_eq!(
            request(
                &router,
                "GET",
                "/authority/session",
                Some(&"f4".repeat(32)),
                Some(token),
                Value::Null
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone())
        );
        use crate::model_runtime::cloud::secrets_vault::{OsKeychainSecretsVault, SecretsVault};
        let config = state.surreal.config();
        let installation = hex::encode(Sha256::digest(
            serde_json::to_vec(&(
                config.path().to_string_lossy(),
                config.namespace(),
                config.database(),
            ))
            .unwrap(),
        ));
        let vault = OsKeychainSecretsVault::new(format!("handshake-local-accounts-{installation}"));
        let session_id = session["session_id"].as_str().unwrap();
        assert!(vault.get(session_id).is_ok_and(|stored| stored == token));
        let other_vault =
            OsKeychainSecretsVault::new(format!("handshake-local-accounts-{installation}-other"));
        assert!(
            matches!(other_vault.get(session_id), Err(crate::model_runtime::cloud::secrets_vault::SecretsVaultError::NoSecretForLane(missing)) if missing == session_id)
        );
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/logout",
                Some(&binding.channel),
                Some(token),
                Value::Null
            )
            .await
            .0,
            StatusCode::OK
        );
        assert!(
            matches!(vault.get(session_id), Err(crate::model_runtime::cloud::secrets_vault::SecretsVaultError::NoSecretForLane(missing)) if missing == session_id)
        );
        for path in ["/authority/session", "/authority/logout"] {
            let method = if path.ends_with("logout") {
                "POST"
            } else {
                "GET"
            };
            assert_eq!(
                request(
                    &router,
                    method,
                    path,
                    Some(&binding.channel),
                    Some(token),
                    Value::Null
                )
                .await,
                (StatusCode::FORBIDDEN, denial.clone())
            );
        }
        let mut counts = state.surreal.test_admin_query("RETURN {setup: array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_SETUP'), login: array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_LOGIN'), logout: array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_LOGOUT')};".into()).await.unwrap();
        assert_eq!(
            counts.take::<Option<Value>>(0).unwrap(),
            Some(json!({"setup":1,"login":1,"logout":1}))
        );
        state.surreal.shutdown().await.unwrap();
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordLogin {
    account_name: String,
    password: String,
}

async fn setup_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    crate::api::stage::capture_channel_binding(&headers).map_err(|_| constant_denial())?;
    let required = state
        .surreal
        .local_owner_setup_required()
        .await
        .map_err(|_| constant_denial())?;
    Ok(Json(
        json!({"schema_version": "hsk.local_account_setup@1", "setup_required": required}),
    ))
}

async fn setup_owner(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<PasswordLogin>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    crate::api::stage::capture_channel_binding(&headers).map_err(|_| constant_denial())?;
    if state
        .surreal
        .local_owner_setup_required()
        .await
        .map_err(|_| constant_denial())?
    {
        let capabilities = state
            .capability_registry
            .profile_by_id("Operator")
            .map_err(|_| constant_denial())?
            .allowed
            .clone();
        let verifier =
            crate::storage::surreal::local_accounts::new_password_verifier(input.password.clone())
                .await
                .map_err(|_| constant_denial())?;
        state
            .surreal
            .setup_local_owner(&input.account_name, verifier, capabilities)
            .await
            .map_err(|_| constant_denial())?;
        state
            .surreal
            .provision_reconciliation_principal(&[], None)
            .await
            .map_err(|_| constant_denial())?;
    } else if !state
        .surreal
        .reconciliation_principal_is_provisioned()
        .await
        .map_err(|_| constant_denial())?
    {
        // Only an interrupted setup can be recovered. A completed setup stays
        // one-time and never mints a session credential on a repeat request.
        state
            .surreal
            .verify_local_owner_password(&input.account_name, input.password)
            .await
            .map_err(|_| constant_denial())?;
        state
            .surreal
            .provision_reconciliation_principal(&[], None)
            .await
            .map_err(|_| constant_denial())?;
    } else {
        return Err(constant_denial());
    }
    Ok(Json(
        json!({"schema_version": "hsk.local_account_setup@1", "setup_required": false}),
    ))
}

async fn login_owner(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<PasswordLogin>,
) -> Result<
    Json<crate::storage::surreal::resource_authority::IssuedSessionCredential>,
    (StatusCode, Json<Value>),
> {
    crate::api::stage::capture_channel_binding(&headers).map_err(|_| constant_denial())?;
    match state
        .surreal
        .login_local_owner(&input.account_name, input.password)
        .await
    {
        Ok(credential) => Ok(Json(credential)),
        Err(_) => {
            let attempt = uuid::Uuid::now_v7().to_string();
            let event = crate::kernel::NewKernelEvent::builder(format!("local-account:{attempt}"), attempt,
                crate::kernel::KernelEventType::LocalAccountLoginDenied,
                KernelActor::System("local_account_authority".into()))
                .aggregate("local_account", "unresolved").source_component("local_account_authority")
                .payload(json!({"schema_version":"hsk.local_account_event@1", "account_id":null,
                    "principal_id":null,"session_id":null,"access_space_id":null,"delegation_chain":[],
                    "resource_refs":[],"action":"LOCAL_ACCOUNT_LOGIN","result":"deny","policy_version":null}))
                .build().map_err(|_| constant_denial())?;
            state
                .storage
                .append_kernel_event(event)
                .await
                .map_err(|_| constant_denial())?;
            Err(constant_denial())
        }
    }
}

pub(crate) async fn authenticated_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::storage::surreal::local_accounts::LocalSessionContext, (StatusCode, Json<Value>)>
{
    authenticated_session_credentials(state, headers)
        .await
        .map(|session| session.context)
}

/// MT-109 C2 (Master Spec 02-system-architecture:2758 deny by default): gate for routes that start
/// processes or touch repository state and have no protected-resource row. Only a persisted account
/// session bound to the live native channel passes; everything else gets the constant denial.
pub(crate) async fn require_authenticated_session(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    match authenticated_session(&state, request.headers()).await {
        Ok(_) => next.run(request).await,
        Err(denial) => denial.into_response(),
    }
}

/// MT-155 (Master Spec 02-system-architecture:2758 deny by default, :2776 capability checks): true
/// only when the authenticated session's delegated capabilities contain exactly `capability_id`.
/// An empty list, an empty id and the `*` wildcard never pass.
pub(crate) fn session_holds_capability(
    context: &crate::storage::surreal::local_accounts::LocalSessionContext,
    capability_id: &str,
) -> bool {
    !capability_id.is_empty()
        && capability_id != "*"
        && context
            .delegated_capabilities
            .iter()
            .any(|held| held == capability_id)
}

pub(crate) struct AuthenticatedLocalSession {
    pub context: crate::storage::surreal::local_accounts::LocalSessionContext,
    pub session_token: String,
    pub channel_binding_hash: String,
}

pub(crate) async fn authenticated_session_credentials(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedLocalSession, (StatusCode, Json<Value>)> {
    let channel =
        crate::api::stage::capture_channel_binding(headers).map_err(|_| constant_denial())?;
    let token = headers
        .get(HSK_HEADER_SESSION_TOKEN)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(constant_denial)?;
    let context = state
        .surreal
        .authenticate_local_session(token, &channel.binding_hash)
        .await
        .map_err(|_| constant_denial())?;
    Ok(AuthenticatedLocalSession {
        context,
        session_token: token.to_owned(),
        channel_binding_hash: channel.binding_hash,
    })
}

/// MT-154 (D-154-3; Master Spec 02-system-architecture.md:2740/:2751/:2758/:2773/:2776): authority
/// for account-owned surfaces that have no workspace and no protected-resource row (Atelier,
/// account-global/surface preferences). The live account session is authenticated through the
/// record access method, the capability is rechecked against the session's delegated capabilities,
/// and the request then runs as the account's record user, so the per-row
/// `owner_account_id = $auth.account_id` table permissions are the data boundary. The scope has no
/// workspace: receipts written inside it carry `wsids = []` and are accepted only by
/// `fn::mt154_account_receipt`.
#[derive(Clone, Debug)]
pub(crate) struct AccountSessionAuthority {
    pub(crate) actor_id: String,
    pub(crate) record_user_scope: RecordUserScope,
}

impl AccountSessionAuthority {
    /// The session principal every receipt written by this request carries.
    pub(crate) fn session_actor(&self) -> KernelActor {
        KernelActor::Operator(self.actor_id.clone())
    }

    /// Runs `operation` as the account's record user (table permissions apply).
    pub(crate) async fn run<T>(
        &self,
        state: &AppState,
        operation: impl std::future::Future<Output = T>,
    ) -> T {
        state
            .surreal
            .with_record_user_scope(self.record_user_scope.clone(), operation)
            .await
    }
}

/// MT-154: authenticates the account session and rechecks `capability_id` for an account-owned
/// surface. Every failure is the constant denial and happens before any product table is touched.
pub(crate) async fn authorize_account_session(
    state: &AppState,
    headers: &HeaderMap,
    capability_id: &'static str,
    action: ResourceAction,
) -> Result<AccountSessionAuthority, (StatusCode, Json<Value>)> {
    let session = authenticated_session_credentials(state, headers).await?;
    if !session_holds_capability(&session.context, capability_id) {
        return Err(constant_denial());
    }
    let record_user_scope = RecordUserScope {
        grant_id: None,
        workspace_id: None,
        // No protected-resource row exists for an account-owned surface; the receipt authority
        // anchor is the owning account (checked by `fn::mt154_account_receipt`).
        resource_id: session.context.identity.account_id.clone(),
        session_id: session.context.session_id.clone(),
        session_token: session.session_token,
        channel_binding_hash: Some(session.channel_binding_hash),
        capability_id: capability_id.to_owned(),
        action,
    };
    Ok(AccountSessionAuthority {
        actor_id: session.context.actor_id,
        record_user_scope,
    })
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Own completion before revocation so cancellation cannot drop vault cleanup.
    tokio::spawn(async move {
        let context = match authenticated_session(&state, &headers).await {
            Ok(context) => context,
            Err(denial) => {
                let channel = crate::api::stage::capture_channel_binding(&headers)
                    .map_err(|_| constant_denial())?;
                let token = headers
                    .get(HSK_HEADER_SESSION_TOKEN)
                    .and_then(|value| value.to_str().ok())
                    .filter(|value| {
                        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
                    .ok_or_else(constant_denial)?;
                if let Some(id) = state
                    .surreal
                    .retired_session_cleanup_id(token, &channel.binding_hash)
                    .await
                    .map_err(|_| constant_denial())?
                {
                    delete_session_secret_with_retry(&state, &id).await?;
                }
                // This authority deletes one retired secret; it never authenticates content.
                return Err(denial);
            }
        };
        state
            .surreal
            .logout_local_account(&context)
            .await
            .map_err(|_| constant_denial())?;
        #[cfg(test)]
        account_lifecycle_probe(&state, "post_revoke").await?;
        delete_session_secret_with_retry(&state, &context.session_id).await?;
        Ok(Json(
            json!({"schema_version": "hsk.local_logout@1", "logged_out": true}),
        ))
    })
    .await
    .map_err(|_| constant_denial())?
}

async fn current_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let context = authenticated_session(&state, &headers).await?;
    Ok(Json(json!({"schema_version": "hsk.current_account@1",
        "account_id": context.identity.account_id, "principal_id": context.identity.principal_id,
        "access_space_id": context.identity.access_space_id, "session_id": context.session_id})))
}

/// Persist bearer material only in the existing OS-bound vault. The database stores hashes.
async fn record_login_event(
    state: &AppState,
    session: &IssuedSession,
    channel_hash: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    let context = state
        .surreal
        .authenticate_local_session(&session.token, channel_hash)
        .await
        .map_err(|error| {
            #[cfg(test)]
            eprintln!("LOCAL_ACCOUNT_TEST_FAILURE phase=login_audit_identity error={error}");
            let _ = error;
            constant_denial()
        })?;
    let event = crate::storage::surreal::local_accounts::account_event(
        crate::kernel::KernelEventType::LocalAccountLogin,
        &context.identity,
        Some(&context.session_id),
        context.policy_version,
        context.delegation_chain,
    )
    .map_err(|_| constant_denial())?;
    state
        .storage
        .append_kernel_event(event)
        .await
        .map_err(|error| {
            #[cfg(test)]
            eprintln!("LOCAL_ACCOUNT_TEST_FAILURE phase=login_audit_append error={error}");
            let _ = error;
            constant_denial()
        })?;
    Ok(())
}

// Revocation stays immediate; transient vault errors do not require a discarded UI context.
// Permanent failure remains an error and the exact retired-session cleanup route remains retryable.
async fn delete_session_secret_with_retry(
    state: &AppState,
    session_id: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    for attempt in 0..3 {
        match store_session_secret(state, session_id, None).await {
            Ok(()) => return Ok(()),
            Err(error) if attempt == 2 => return Err(error),
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
        }
    }
    Err(constant_denial())
}
async fn store_session_secret(
    state: &AppState,
    session_id: &str,
    token: Option<String>,
) -> Result<(), (StatusCode, Json<Value>)> {
    #[cfg(feature = "os-keychain")]
    {
        use crate::model_runtime::cloud::secrets_vault::{OsKeychainSecretsVault, SecretsVault};
        use sha2::{Digest, Sha256};
        // Namespace includes this installation's storage root; independent installations and
        // proof databases cannot overwrite each other's OS credentials.
        let config = state.surreal.config();
        let identity = serde_json::to_vec(&(
            config.path().to_string_lossy(),
            config.namespace(),
            config.database(),
        ))
        .map_err(|_| constant_denial())?;
        let installation = hex::encode(Sha256::digest(identity));
        let namespace = format!("handshake-local-accounts-{installation}");
        #[cfg(test)]
        if token.is_none() {
            account_lifecycle_probe(state, "vault_delete").await?;
        }
        let key = session_id.to_owned();
        // Test builds record every written lane so the store teardown deletes it
        // (storage::tests::shutdown_and_remove_test_store).
        #[cfg(test)]
        if token.is_some() {
            crate::storage::tests::record_test_vault_lane(
                config.path().to_path_buf(),
                namespace.clone(),
                key.clone(),
            );
        }
        run_session_vault_operation(move || {
            let vault = OsKeychainSecretsVault::new(namespace);
            match token {
                Some(token) => vault.put(&key, token),
                None => vault.delete(&key),
            }
        })
        .await?
        .map_err(|error| {
            #[cfg(test)]
            eprintln!("LOCAL_ACCOUNT_TEST_FAILURE phase=os_vault_operation error={error}");
            let _ = error;
            constant_denial()
        })
    }
    #[cfg(not(feature = "os-keychain"))]
    {
        let _ = (state, session_id, token);
        Err(constant_denial())
    }
}

pub(crate) fn constant_denial() -> (StatusCode, Json<Value>) {
    (
        StatusCode::FORBIDDEN,
        Json(json!({"error": "HSK-403-PROTECTED-RESOURCE"})),
    )
}

pub(crate) async fn authorize_request(
    state: &AppState,
    headers: &HeaderMap,
    capability_id: &'static str,
    resource_kind: ResourceKind,
    external_resource_id: &str,
    action: ResourceAction,
) -> Result<AuthorizedResourceContext, (StatusCode, Json<Value>)> {
    let session_token = headers
        .get(HSK_HEADER_SESSION_TOKEN)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(constant_denial)?
        .to_owned();
    let channel =
        crate::api::stage::capture_channel_binding(headers).map_err(|_| constant_denial())?;
    let channel_binding_hash = channel.binding_hash;
    let decision = state
        .surreal
        .authorize_protected_resource(AuthorizationRequest {
            session_token: session_token.clone(),
            channel_binding_hash: Some(channel_binding_hash.clone()),
            capability_id: capability_id.to_owned(),
            resource_kind,
            external_resource_id: external_resource_id.to_owned(),
            action,
        })
        .await
        .map_err(|error| {
            tracing::warn!(
                target: "handshake_core::resource_authority",
                error = %error,
                capability_id,
                "protected_resource_authorization_denied"
            );
            constant_denial()
        })?;

    if !state
        .capability_registry
        .profile_can(&decision.capability_profile_id, capability_id)
        .unwrap_or(false)
    {
        return Err(constant_denial());
    }

    Ok(AuthorizedResourceContext {
        decision_id: decision.decision_id,
        account_id: decision.account_id,
        principal_id: decision.principal_id,
        session_id: decision.session_id.clone(),
        access_space_id: decision.access_space_id,
        resource_id: decision.resource_id.clone(),
        actor_kind: decision.actor_kind,
        actor_id: decision.actor_id,
        capability_profile_id: decision.capability_profile_id,
        capability_id: capability_id.to_owned(),
        delegation_chain: decision.delegation_chain,
        record_user_scope: RecordUserScope {
            grant_id: Some(decision.grant_id.clone()),
            workspace_id: Some(external_resource_id.to_owned()),
            session_token,
            channel_binding_hash: Some(channel_binding_hash),
            resource_id: decision.resource_id,
            session_id: decision.session_id,
            capability_id: capability_id.to_owned(),
            action,
        },
    })
}

async fn exchange_local_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(exchange): Json<LocalSessionExchange>,
) -> Result<Json<LocalSessionResponse>, (StatusCode, Json<Value>)> {
    let channel =
        crate::api::stage::capture_channel_binding(&headers).map_err(|_| constant_denial())?;
    // Own completion before consuming the credential or creating durable authority.
    tokio::spawn(async move {
        let session = state
            .surreal
            .exchange_session_credential(
                &exchange.account_id,
                &exchange.principal_id,
                &exchange.access_space_id,
                &exchange.authentication_token,
                &channel.binding_hash,
                std::time::Duration::from_secs(LOCAL_SESSION_TTL_HOURS as u64 * 60 * 60),
            )
            .await
            .map_err(|error| {
                #[cfg(test)]
                eprintln!("LOCAL_ACCOUNT_TEST_FAILURE phase=session_exchange error={error}");
                tracing::error!(
                    target: "handshake_core::resource_authority",
                    error = %error,
                    "local_authority_session_bootstrap_failed"
                );
                constant_denial()
            })?;
        #[cfg(test)]
        account_lifecycle_probe(&state, "post_session").await?;
        if let Err(error) =
            store_session_secret(&state, &session.session_id, Some(session.token.clone())).await
        {
            let _ = state.surreal.revoke_session(&session.session_id).await;
            return Err(error);
        }
        #[cfg(test)]
        account_lifecycle_probe(&state, "post_vault").await?;
        if let Err(error) = record_login_event(&state, &session, &channel.binding_hash).await {
            let _ = state.surreal.revoke_session(&session.session_id).await;
            let _ = delete_session_secret_with_retry(&state, &session.session_id).await;
            return Err(error);
        }
        Ok(Json(LocalSessionResponse {
            schema_version: "hsk.authenticated_session@1",
            session_token: session.token,
            account_id: session.account_id,
            principal_id: session.principal_id,
            session_id: session.session_id,
            access_space_id: session.access_space_id,
            expires_at: session.expires_at,
        }))
    })
    .await
    .map_err(|_| constant_denial())?
}

#[cfg(test)]
async fn provision_existing_workspace_grants(
    state: &AppState,
    principal: &ProvisionedPrincipal,
) -> Result<(), (StatusCode, Json<Value>)> {
    let workspaces = state.storage.list_workspaces().await.map_err(|error| {
        tracing::error!(target: "handshake_core::resource_authority", error = %error, "workspace_authority_enumeration_failed");
        constant_denial()
    })?;
    for workspace in workspaces {
        let workspace_resource = state
            .surreal
            .register_workspace_resource(&principal.identity, &workspace.id)
            .await
            .map_err(|error| {
                tracing::error!(target: "handshake_core::resource_authority", error = %error, "workspace_resource_registration_failed");
                constant_denial()
            })?;
        grant(
            state,
            principal,
            workspace_resource.resource_id.clone(),
            vec![
                ResourceAction::Read,
                ResourceAction::Create,
                ResourceAction::Update,
            ],
            vec![
                "fr.read",
                "fr.ingest.runtime_chat",
                "fr.ingest.native_editor",
                "memory.read",
                "memory.propose",
                "memory.review",
                "memory.commit",
            ],
        )
        .await?;

        let surfaces = [
            (
                ResourceKind::FlightRecorder,
                vec![ResourceAction::Read, ResourceAction::Create],
                vec![
                    "fr.read",
                    "fr.ingest.runtime_chat",
                    "fr.ingest.native_editor",
                ],
            ),
            (
                ResourceKind::MemoryPack,
                vec![ResourceAction::Read],
                vec!["memory.read"],
            ),
            (
                ResourceKind::MemoryProposal,
                vec![
                    ResourceAction::Read,
                    ResourceAction::Create,
                    ResourceAction::Update,
                ],
                vec![
                    "memory.read",
                    "memory.propose",
                    "memory.review",
                    "memory.commit",
                ],
            ),
            (
                ResourceKind::MemoryCommitReport,
                vec![ResourceAction::Read, ResourceAction::Create],
                vec!["memory.read", "memory.commit"],
            ),
            (
                ResourceKind::MemoryItem,
                vec![
                    ResourceAction::Read,
                    ResourceAction::Create,
                    ResourceAction::Update,
                ],
                vec!["memory.read", "memory.commit"],
            ),
            (
                ResourceKind::MemoryItemCount,
                vec![ResourceAction::Read, ResourceAction::Create],
                vec!["memory.read", "memory.commit"],
            ),
        ];
        for (kind, actions, capabilities) in surfaces {
            let resource = state
                .surreal
                .register_protected_resource(
                    &principal.identity,
                    kind,
                    &workspace.id,
                    Some(&workspace_resource.resource_id),
                    "account_private",
                )
                .await
                .map_err(|error| {
                    tracing::error!(target: "handshake_core::resource_authority", error = %error, "protected_resource_registration_failed");
                    constant_denial()
                })?;
            grant(
                state,
                principal,
                resource.resource_id,
                actions,
                capabilities,
            )
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) async fn test_principal_for_binding(
    state: &AppState,
    binding_token: &str,
) -> Result<ProvisionedPrincipal, String> {
    let binding_hash = hex::encode(Sha256::digest(binding_token.as_bytes()));
    let principal = state
        .surreal
        .provision_local_operator(Some(&binding_hash))
        .await
        .map_err(|error| error.to_string())?;
    provision_existing_workspace_grants(state, &principal)
        .await
        .map_err(|(_, body)| body.0.to_string())?;
    Ok(principal)
}

#[cfg(test)]
pub(crate) async fn test_session_for_binding(
    state: &AppState,
    binding_token: &str,
) -> Result<String, String> {
    Ok(test_principal_for_binding(state, binding_token)
        .await?
        .session
        .token)
}

#[cfg(test)]
async fn grant(
    state: &AppState,
    principal: &ProvisionedPrincipal,
    resource_id: String,
    actions: Vec<ResourceAction>,
    capability_ids: Vec<&str>,
) -> Result<(), (StatusCode, Json<Value>)> {
    state
        .surreal
        .grant_resource(
            &principal.identity.account_id,
            &principal.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(),
                resource_id,
                actions,
                capability_ids: capability_ids.into_iter().map(str::to_owned).collect(),
                expires_at: Some(chrono::Utc::now() + Duration::hours(LOCAL_SESSION_TTL_HOURS)),
                delegation_chain: Vec::new(),
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| {
            tracing::error!(target: "handshake_core::resource_authority", error = %error, "resource_grant_provision_failed");
            constant_denial()
        })
}

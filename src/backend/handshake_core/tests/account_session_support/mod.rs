//! WP-KERNEL-012 MT-109 / Master Spec LM-RLS-002: shared authenticated-record-user fixture for
//! backend route harnesses.
//!
//! Every protected document / Loom / workspace / code-navigation route authorizes ONLY a persisted
//! account session (`x-hsk-session-token`) bound to the live native-MCP channel
//! (`x-hsk-channel-binding-token`) and answers a constant 403 otherwise. Route harnesses therefore
//! run their positive paths as an authenticated record user:
//!
//! * [`NativeBindingEnv`] installs a live native-MCP binding for THIS test process through the
//!   product's own `api::stage::current_process_native_session_binding`;
//! * [`OwnerSession::provision`] provisions an `Owner` account / `human_account` principal / access
//!   space whose session is bound to that binding's hash (`provision_principal`);
//! * [`OwnerSession::create_workspace`] creates the workspace through the real `POST /workspaces`
//!   route as that owner, so it carries its account-owned protected resource and grant;
//! * [`OwnerSession::apply`] / [`OwnerSession::client`] attach the two credential headers.
//!
//! Mirrors the in-crate reference fixture `api::workspaces::tests::{WorkspaceBindingFixture,
//! workspace_test_principal, create_owned_test_workspace}` using only public `handshake_core` API.
//!
//! `HANDSHAKE_STAGE_BINDING_FILE` is process-global: every test in a binary that installs a binding
//! (through this module or a file-local binding fixture) MUST hold [`NATIVE_BINDING_ENV_LOCK`].
#![allow(dead_code)]

use handshake_core::storage::surreal::SurrealStorage;
use handshake_core::AppState;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const SESSION_TOKEN_HEADER: &str = "x-hsk-session-token";
pub const CHANNEL_BINDING_TOKEN_HEADER: &str = "x-hsk-channel-binding-token";

/// Capabilities delegated to the harness Owner principal (same set as the in-crate reference
/// fixture `api::workspaces::tests::workspace_test_principal`).
const OWNER_CAPABILITIES: [&str; 7] = [
    "fs.read",
    "fs.write",
    "fr.read",
    "fr.ingest.runtime_chat",
    "fr.ingest.native_editor",
    "memory.read",
    "memory.propose",
];

/// The single per-binary guard for the process-global `HANDSHAKE_STAGE_BINDING_FILE`.
pub static NATIVE_BINDING_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// A live native-MCP binding for this test process. Install only while holding
/// [`NATIVE_BINDING_ENV_LOCK`]; dropping it restores the previous env value and removes the file.
pub struct NativeBindingEnv {
    token: String,
    path: std::path::PathBuf,
    previous: Option<std::ffi::OsString>,
}

impl NativeBindingEnv {
    pub fn install() -> Self {
        let token = sha256_hex(uuid::Uuid::now_v7().as_bytes());
        let path = std::env::temp_dir().join(format!(
            "handshake-account-session-binding-{}.json",
            uuid::Uuid::now_v7()
        ));
        std::fs::write(
            &path,
            serde_json::to_vec(
                &handshake_core::api::stage::current_process_native_session_binding(&token),
            )
            .expect("serialize account-session native binding"),
        )
        .expect("write account-session native binding");
        let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", &path);
        Self {
            token,
            path,
            previous,
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }
}

impl Drop for NativeBindingEnv {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", value),
            None => std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE"),
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

/// An authenticated Owner record user: persisted account session + the live channel token it is
/// bound to.
#[derive(Clone, Debug)]
pub struct OwnerSession {
    pub session_token: String,
    pub channel_binding_token: String,
    pub account_id: String,
    pub principal_id: String,
    pub access_space_id: String,
    pub session_id: String,
    pub actor_id: String,
}

impl OwnerSession {
    /// Provision an Owner principal/session bound to `channel_binding_token`, which MUST be the
    /// token of the binding currently installed in `HANDSHAKE_STAGE_BINDING_FILE`.
    pub async fn provision(storage: &SurrealStorage, channel_binding_token: &str) -> Self {
        if !storage
            .reconciliation_principal_is_provisioned()
            .await
            .expect("read reconciliation principal provisioning state")
        {
            storage
                .provision_reconciliation_principal(&[], None)
                .await
                .expect("provision reconciliation service principal");
        }
        let key = format!("route-harness-owner-{}", uuid::Uuid::now_v7());
        let capabilities = OWNER_CAPABILITIES.map(str::to_owned).to_vec();
        let principal = storage
            .provision_principal(
                &key,
                &key,
                "human_account",
                &key,
                "Operator",
                &capabilities,
                &key,
                Some(&sha256_hex(channel_binding_token.as_bytes())),
                std::time::Duration::from_secs(3600),
            )
            .await
            .expect("provision Owner principal bound to the live native binding");
        Self {
            session_token: principal.session.token,
            channel_binding_token: channel_binding_token.to_owned(),
            account_id: principal.identity.account_id,
            principal_id: principal.identity.principal_id,
            access_space_id: principal.identity.access_space_id,
            session_id: principal.session.session_id,
            actor_id: key,
        }
    }

    /// The two credential headers every protected route requires.
    pub fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            SESSION_TOKEN_HEADER,
            HeaderValue::from_str(&self.session_token).expect("session token header value"),
        );
        headers.insert(
            CHANNEL_BINDING_TOKEN_HEADER,
            HeaderValue::from_str(&self.channel_binding_token)
                .expect("channel binding token header value"),
        );
        headers
    }

    /// Attach the credential headers to one request.
    pub fn apply(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header(SESSION_TOKEN_HEADER, &self.session_token)
            .header(CHANNEL_BINDING_TOKEN_HEADER, &self.channel_binding_token)
    }

    /// A client that sends the credential headers on every request. A header set explicitly on a
    /// request (for example a deliberately invalid session token) replaces the default.
    pub fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .default_headers(self.headers())
            .build()
            .expect("build account-session reqwest client")
    }

    /// Create a workspace through the real `POST /workspaces` route as this Owner.
    pub async fn create_workspace(&self, state: &AppState) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind workspace-create loopback listener");
        let addr = listener
            .local_addr()
            .expect("workspace-create listener addr");
        let app = handshake_core::api::workspaces::routes(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("workspace-create route server");
        });
        let response = self
            .apply(reqwest::Client::new().post(format!("http://{addr}/workspaces")))
            .json(&json!({ "name": format!("route-harness-ws-{}", uuid::Uuid::now_v7()) }))
            .send()
            .await
            .expect("send owner POST /workspaces");
        let status = response.status();
        let body: Value = response
            .json()
            .await
            .expect("owner POST /workspaces response JSON");
        server.abort();
        let _ = server.await;
        assert_eq!(
            status,
            reqwest::StatusCode::CREATED,
            "owner workspace create through POST /workspaces must succeed: {body}"
        );
        body["id"]
            .as_str()
            .expect("POST /workspaces returns the workspace id")
            .to_owned()
    }
}

/// Lock + installed binding + provisioned Owner, held for one test body.
///
/// Field order is drop order: the Owner handle, then the binding (restores the env), then the lock.
pub struct AccountFixture {
    pub owner: OwnerSession,
    binding: NativeBindingEnv,
    _lock: tokio::sync::MutexGuard<'static, ()>,
}

impl AccountFixture {
    /// Take [`NATIVE_BINDING_ENV_LOCK`], install a fresh live binding and provision an Owner bound
    /// to it. Do NOT call while already holding the lock (use [`OwnerSession::provision`] with the
    /// already-installed binding's token instead).
    pub async fn install(storage: &SurrealStorage) -> Self {
        let lock = NATIVE_BINDING_ENV_LOCK.lock().await;
        let binding = NativeBindingEnv::install();
        let owner = OwnerSession::provision(storage, binding.token()).await;
        Self {
            owner,
            binding,
            _lock: lock,
        }
    }

    pub fn binding_token(&self) -> &str {
        self.binding.token()
    }
}

impl std::ops::Deref for AccountFixture {
    type Target = OwnerSession;

    fn deref(&self) -> &OwnerSession {
        &self.owner
    }
}

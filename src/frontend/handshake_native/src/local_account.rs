//! Immutable account identity for a single authenticated backend connection.

use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct AuthenticatedContext {
    pub account_id: String,
    pub principal_id: String,
    pub session_id: String,
    pub access_space_id: String,
    session_token: String,
    #[serde(skip)]
    channel_token: String,
    #[serde(skip)]
    backend_origin: String,
    #[serde(skip)]
    invalidated: std::sync::Arc<std::sync::atomic::AtomicBool>,
    #[serde(skip)]
    rejected: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl std::fmt::Debug for AuthenticatedContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthenticatedContext").field("account_id", &self.account_id)
            .field("principal_id", &self.principal_id).field("session_id", &self.session_id).finish_non_exhaustive()
    }
}

impl AuthenticatedContext {
    pub fn invalidate(&self) { self.invalidated.store(true, std::sync::atomic::Ordering::Release); }
    pub fn is_active(&self) -> bool { !self.invalidated.load(std::sync::atomic::Ordering::Acquire) }
    pub fn observe_status(&self, status: reqwest::StatusCode) {
        if matches!(status.as_u16(), 401 | 403) { self.rejected.store(true, std::sync::atomic::Ordering::Release); }
    }

    pub fn bind(mut self, backend: &str, channel_token: String) -> Result<Self, String> {
        let origin = reqwest::Url::parse(backend).map_err(|_| "Invalid backend URL")?;
        if !valid_token(&self.session_token) || !valid_token(&channel_token) {
            return Err("Invalid authentication response".into());
        }
        self.backend_origin = origin.origin().ascii_serialization();
        self.channel_token = channel_token;
        Ok(self)
    }

    pub fn authorize_builder(&self, _client: reqwest::Client, request: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder, String> {
        Ok(reqwest::RequestBuilder::from_parts(crate::backend_client::shared_http_client(), self.authorize(request)?))
    }

    pub fn authorize(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Request, String> {
        if !self.is_active() { return Err("Account session is no longer active".into()); }
        self.authorize_inner(request)
    }

    fn authorize_inner(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Request, String> {
        let mut request = request.build().map_err(|_| "Invalid backend request")?;
        if request.url().origin().ascii_serialization() != self.backend_origin {
            return Err("Authenticated backend origin mismatch".into());
        }
        request.headers_mut().insert("x-hsk-session-token", self.session_token.parse()
            .map_err(|_| "Invalid session credential")?);
        request.headers_mut().insert("x-hsk-channel-binding-token", self.channel_token.parse()
            .map_err(|_| "Invalid channel credential")?);
        Ok(request)
    }
}

pub struct AuthenticatedRequest {
    client: reqwest::Client,
    context: Option<std::sync::Arc<AuthenticatedContext>>,
    request: reqwest::RequestBuilder,
}

impl AuthenticatedRequest {
    pub fn new(_client: reqwest::Client, context: Option<std::sync::Arc<AuthenticatedContext>>, request: reqwest::RequestBuilder) -> Self { Self { client: crate::backend_client::shared_http_client(), context, request } }
    pub async fn json(self) -> Result<serde_json::Value, String> {
        let context = self.context.clone();
        let response = self.send().await?;
        if !response.status().is_success() { return Err(format!("Protected request denied ({})", response.status())); }
        let value = response.json().await.map_err(|_| "Invalid backend response".to_owned())?;
        if !context.as_ref().is_some_and(|context| context.is_active()) { return Err("Account session is no longer active".into()); }
        Ok(value)
    }
    pub async fn send(self) -> Result<reqwest::Response, String> {
        let context = self.context.ok_or_else(|| "Account login required".to_owned())?;
        let request = context.authorize(self.request)?;
        let response = self.client.execute(request).await.map_err(|error| error.to_string())?;
        context.observe_status(response.status());
        if !context.is_active() { return Err("Account session is no longer active".into()); }
        Ok(response)
    }
}

fn valid_token(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub async fn setup_required(_client: &reqwest::Client, backend: &str, channel: &str) -> Result<bool, String> {
    let client = &crate::backend_client::shared_http_client();
    let value = channel_request(client.get(format!("{backend}/authority/setup")), channel)?
        .send().await.map_err(|_| "Backend unavailable")?;
    let value = checked_json(value).await?;
    value.get("setup_required").and_then(serde_json::Value::as_bool)
        .ok_or_else(|| "Invalid setup response".into())
}

pub async fn setup_owner(_client: &reqwest::Client, backend: &str, channel: &str, account_name: &str, password: String) -> Result<(), String> {
    let client = &crate::backend_client::shared_http_client();
    let response = channel_request(client.post(format!("{backend}/authority/setup")), channel)?
        .json(&serde_json::json!({"account_name": account_name, "password": password}))
        .send().await.map_err(|_| "Backend unavailable")?;
    checked_json(response).await.map(|_| ())
}

pub async fn login(_client: &reqwest::Client, backend: &str, channel: String, account_name: &str, password: String) -> Result<AuthenticatedContext, String> {
    let client = &crate::backend_client::shared_http_client();
    let credential = channel_request(client.post(format!("{backend}/authority/login")), &channel)?
        .json(&serde_json::json!({"account_name": account_name, "password": password}))
        .send().await.map_err(|_| "Backend unavailable")?;
    let credential = checked_json(credential).await?;
    let response = channel_request(client.post(format!("{backend}/authority/session")), &channel)?
        .json(&serde_json::json!({
            "account_id": credential.get("account_id"),
            "principal_id": credential.get("principal_id"),
            "access_space_id": credential.get("access_space_id"),
            "authentication_token": credential.get("token"),
        })).send().await.map_err(|_| "Backend unavailable")?;
    let context: AuthenticatedContext = serde_json::from_value(checked_json(response).await?)
        .map_err(|_| "Invalid session response")?;
    context.bind(backend, channel)
}

pub async fn logout(_client: &reqwest::Client, backend: &str, context: &AuthenticatedContext) -> Result<(), String> {
    let client = &crate::backend_client::shared_http_client();
    let request = context.authorize_inner(client.post(format!("{backend}/authority/logout")))?;
    context.invalidate();
    let response = client.execute(request).await.map_err(|_| "Backend unavailable")?;
    checked_json(response).await.map(|_| ())
}

pub async fn current_session(_client: &reqwest::Client, backend: &str, context: &AuthenticatedContext) -> Result<bool, String> {
    let client = &crate::backend_client::shared_http_client();
    let request = context.authorize(client.get(format!("{backend}/authority/session")))?;
    let response = client.execute(request).await.map_err(|_| "Backend unavailable")?;
    if matches!(response.status().as_u16(), 401 | 403) { context.invalidate(); return Ok(false); }
    let value = checked_json(response).await?;
    let matches = value["schema_version"].as_str() == Some("hsk.current_account@1")
        && value["session_id"].as_str() == Some(context.session_id.as_str())
        && value["account_id"].as_str() == Some(context.account_id.as_str())
        && value["principal_id"].as_str() == Some(context.principal_id.as_str())
        && value["access_space_id"].as_str() == Some(context.access_space_id.as_str());
    if !matches { context.invalidate(); }
    Ok(matches)
}

fn channel_request(request: reqwest::RequestBuilder, channel: &str) -> Result<reqwest::RequestBuilder, String> {
    if !valid_token(channel) { return Err("Native channel unavailable".into()); }
    Ok(request.header("x-hsk-channel-binding-token", channel))
}

async fn checked_json(response: reqwest::Response) -> Result<serde_json::Value, String> {
    if !response.status().is_success() { return Err("Account authentication denied".into()); }
    response.json().await.map_err(|_| "Invalid authentication response".into())
}

#[derive(Default)]
pub struct AccountUi {
    pub context: Option<std::sync::Arc<AuthenticatedContext>>,
    reset_requested: bool,
    confirm_discard: bool,
    workspace_name: String,
    workspace_created: bool,
    account_name: String,
    password: String,
    setup_required: Option<bool>,
    error: Option<String>,
    pending: Option<std::sync::mpsc::Receiver<Result<AccountResult, String>>>,
}

enum AccountResult { Status(bool), Setup, Login(AuthenticatedContext), Logout, Current(bool), WorkspaceCreated }

impl AccountUi {
    pub fn take_workspace_created(&mut self) -> bool { std::mem::take(&mut self.workspace_created) }
    pub fn take_reset_requested(&mut self) -> bool { std::mem::take(&mut self.reset_requested) }

    pub fn show(&mut self, ctx: &egui::Context, backend: &str, channel: String, runtime: Option<&tokio::runtime::Handle>, allow_actions: bool) {
        if allow_actions {
        if let Some(receiver) = &self.pending {
            let result = match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(Err("Account operation interrupted. Retry.".into())),
            };
            if let Some(result) = result {
                self.pending = None;
                match result {
                    Ok(AccountResult::Status(required)) => self.setup_required = Some(required),
                    Ok(AccountResult::Setup) => self.setup_required = Some(false),
                    Ok(AccountResult::Login(context)) => { self.context = Some(std::sync::Arc::new(context)); self.reset_requested = true; self.confirm_discard = false; },
                    Ok(AccountResult::Logout) => self.context = None,
                    Ok(AccountResult::Current(false)) => { if let Some(context) = self.context.take() { context.invalidate(); } self.error = Some("Session rejected. Log in again.".into()); }
                    Ok(AccountResult::Current(true)) => {}
                    Ok(AccountResult::WorkspaceCreated) => { self.workspace_created = true; self.workspace_name.clear(); }
                    Err(error) => self.error = Some(error),
                }
            }
        }
        }
        let mut action = if allow_actions && self.pending.is_none() && self.context.as_ref().is_some_and(|context| context.rejected.swap(false, std::sync::atomic::Ordering::AcqRel)) { Some(4) } else { None };
        egui::TopBottomPanel::top("local-account").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if let Some(context) = &self.context {
                    let current = ui.label(format!("Account {} · Space {}", context.account_id, context.access_space_id));
                    crate::accessibility::emit_interactive_node(ctx, current.id, "account.current");
                    let check = ui.add_enabled(self.pending.is_none(), egui::Button::new("Check session"));
                    crate::accessibility::emit_interactive_node(ctx, check.id, "account.check-session");
                    if check.clicked() { action = Some(4); }
                    let button = ui.add_enabled(self.pending.is_none(), egui::Button::new("Log out"));
                    crate::accessibility::emit_interactive_node(ctx, button.id, "account.logout");
                    if button.clicked() { self.confirm_discard = true; }
                    let name_label = ui.label("New workspace name");
                    let name = ui.add(egui::TextEdit::singleline(&mut self.workspace_name).desired_width(150.0)).labelled_by(name_label.id);
                    crate::accessibility::emit_interactive_node(ctx, name.id, "account.workspace-name");
                    let create = ui.add_enabled(self.pending.is_none() && !self.workspace_name.trim().is_empty(), egui::Button::new("Create workspace"));
                    crate::accessibility::emit_interactive_node(ctx, create.id, "account.create-workspace");
                    if create.clicked() { action = Some(5); }

                } else {
                    ui.label("Account (Owner setup password: 12–1024 UTF-8 bytes)");
                    let name_label = ui.label("Account name");
                    let name = ui.add(egui::TextEdit::singleline(&mut self.account_name).desired_width(150.0)).labelled_by(name_label.id);
                    crate::accessibility::emit_interactive_node(ctx, name.id, "account.name");
                    let password_label = ui.label("Password");
                    let password = ui.add(egui::TextEdit::singleline(&mut self.password).desired_width(150.0).password(true)).labelled_by(password_label.id);
                    crate::accessibility::emit_interactive_node(ctx, password.id, "account.password");
                    ctx.accesskit_node_builder(password.id, |node| node.set_value(""));
                    let button = ui.add_enabled(self.pending.is_none(), egui::Button::new(if self.setup_required == Some(true) { "Set up Owner" } else { "Log in" }));
                    crate::accessibility::emit_interactive_node(ctx, button.id, "account.submit");
                    if button.clicked() { if self.setup_required == Some(true) { action = Some(1); } else { self.confirm_discard = true; } }
                    let check = ui.add_enabled(self.pending.is_none(), egui::Button::new("Check setup"));
                    crate::accessibility::emit_interactive_node(ctx, check.id, "account.check-setup");
                    if check.clicked() { action = Some(0); }
                }
                if self.confirm_discard {
                    ui.label("Continuing discards unsaved workspace edits. Cancel to keep them.");
                    let confirm = ui.button(if self.context.is_some() { "Discard edits and log out" } else { "Discard edits and log in" });
                    crate::accessibility::emit_interactive_node(ctx, confirm.id, "account.confirm-discard");
                    if confirm.clicked() { action = Some(if self.context.is_some() { 3 } else { 2 }); self.confirm_discard = false; }
                    let cancel = ui.button("Cancel");
                    crate::accessibility::emit_interactive_node(ctx, cancel.id, "account.cancel-discard");
                    if cancel.clicked() { self.confirm_discard = false; }
                }
                if self.pending.is_some() { ui.spinner(); }
                if let Some(error) = &self.error { let label = ui.label(error); crate::accessibility::emit_interactive_node(ctx, label.id, "account.error"); }
            });
        });
        if !allow_actions { return; }
        if let Some(action) = action {
            let Some(runtime) = runtime else { self.error = Some("Backend runtime unavailable".into()); return; };
            let client = crate::backend_client::shared_http_client();
            let backend = backend.to_owned();
            let name = self.account_name.clone();
            let workspace_name = self.workspace_name.clone();
            let password = if matches!(action, 1 | 2) { std::mem::take(&mut self.password) } else { String::new() };
            let context = self.context.clone();
            if action == 3 { if let Some(context) = self.context.take() { context.invalidate(); } self.reset_requested = true; }
            let repaint = ctx.clone();
            let (sender, receiver) = std::sync::mpsc::channel();
            self.pending = Some(receiver);
            self.error = None;
            runtime.spawn(async move {
                let result = match action {
                    0 => setup_required(&client, &backend, &channel).await.map(AccountResult::Status),
                    1 => setup_owner(&client, &backend, &channel, &name, password).await.map(|_| AccountResult::Setup),
                    2 => login(&client, &backend, channel, &name, password).await.map(AccountResult::Login),
                    5 => {
                        match context {
                            Some(context) => {
                                let request = client.post(format!("{backend}/workspaces")).json(&serde_json::json!({"name": workspace_name}));
                                match AuthenticatedRequest::new(client.clone(), Some(context), request).send().await {
                                    Ok(response) => checked_json(response).await.map(|_| AccountResult::WorkspaceCreated),
                                    Err(error) => Err(error),
                                }
                            }
                            None => Err("No active account".into()),
                        }
                    }
                    4 => match context { Some(context) => current_session(&client, &backend, &context).await.map(AccountResult::Current), None => Err("No active account".into()) },
                    _ => match context { Some(context) => logout(&client, &backend, &context).await.map(|_| AccountResult::Logout), None => Err("No active account".into()) },
                };
                let _ = sender.send(result);
                repaint.request_repaint();
            });
        }
    }
}

#[cfg(test)]
pub(crate) fn mock_account_context(base: &str) -> std::sync::Arc<AuthenticatedContext> {
    std::sync::Arc::new(serde_json::from_value::<AuthenticatedContext>(serde_json::json!({
        "account_id": "mock-account", "principal_id": "mock-principal", "session_id": "mock-session", "access_space_id": "mock-space", "session_token": "a".repeat(64)
    })).expect("typed mock account").bind(base, "b".repeat(64)).expect("mock endpoint origin"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(account: &str) -> AuthenticatedContext {
        serde_json::from_value::<AuthenticatedContext>(serde_json::json!({
            "account_id": account, "principal_id": format!("principal-{account}"),
            "session_id": format!("session-{account}"), "access_space_id": format!("space-{account}"),
            "session_token": "a".repeat(64)
        })).unwrap().bind("http://127.0.0.1:9911", "b".repeat(64)).unwrap()
    }

    #[test]
    fn account_session_and_channel_are_distinct_and_origin_bound() {
        let account = context("one");
        let client = crate::backend_client::shared_http_client();
        let request = account.authorize(client.get("http://127.0.0.1:9911/authority/session")).unwrap();
        assert_eq!(request.headers()["x-hsk-session-token"], "a".repeat(64));
        assert_eq!(request.headers()["x-hsk-channel-binding-token"], "b".repeat(64));
        assert!(account.authorize(client.get("http://127.0.0.1:9912/authority/session")).is_err());
        assert!(!format!("{account:?}").contains(&"a".repeat(64)));
    }

    #[test]
    fn invalidation_reaches_captured_context_without_retargeting_another_account() {
        let first = std::sync::Arc::new(context("one"));
        let in_flight = first.clone();
        let second = context("two");
        first.invalidate();
        assert!(!in_flight.is_active());
        assert!(second.is_active());
        assert_eq!(in_flight.account_id, "one");
        assert_eq!(second.account_id, "two");
        assert!(in_flight.authorize(crate::backend_client::shared_http_client().get("http://127.0.0.1:9911/knowledge/documents")).is_err());
    }

    #[test]
    fn disconnected_account_worker_releases_pending_state_without_discarding_workspace() {
        let mut account = AccountUi::default();
        let (sender, receiver) = std::sync::mpsc::channel();
        account.pending = Some(receiver);
        drop(sender);
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| account.show(ctx, "http://127.0.0.1:9911", "b".repeat(64), None, true));
        assert!(account.pending.is_none());
        assert!(account.error.as_deref().unwrap().contains("interrupted"));
        assert!(!account.take_reset_requested());
    }

    #[test]
    fn discard_confirmation_does_not_drop_context_before_confirmation() {
        let mut account = AccountUi { context: Some(std::sync::Arc::new(context("one"))), confirm_discard: true, ..AccountUi::default() };
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| account.show(ctx, "http://127.0.0.1:9911", "b".repeat(64), None, true));
        assert!(account.context.as_ref().unwrap().is_active());
        assert!(!account.take_reset_requested());
    }
    #[test]
    fn account_accesskit_and_argus_snapshot_never_publish_password() {
        let secret = "unique-account-password-DO-NOT-PUBLISH-4389";
        for width in [1200.0, 480.0, 320.0] {
            let mut account = AccountUi { password: secret.into(), setup_required: Some(true), ..AccountUi::default() };
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 800.0))), ..Default::default() };
            let output = ctx.run(input, |ctx| account.show(ctx, "http://127.0.0.1:9911", "b".repeat(64), None, true));
            let update = output.platform_output.accesskit_update.expect("actual account AccessKit frame");
            assert!(crate::accessibility::assert_no_unnamed_interactive(&update) > 0);
            assert!(!format!("{update:?}").contains(secret), "AccessKit must not expose the password");
            let snapshot = crate::accessibility::collect_ui_tree_snapshot(&update);
            assert!(!snapshot.to_json().contains(secret), "Argus snapshot must not expose the password");
            for id in ["account.name", "account.password", "account.submit", "account.check-setup"] { assert!(snapshot.find_by_author_id(id).is_some(), "missing {id} at width {width}"); }
        }
    }

    #[test]
    fn authenticated_executor_does_not_follow_cross_origin_redirect() {
        use std::io::{Read, Write};
        let first = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let destination = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        destination.set_nonblocking(true).unwrap();
        let base = format!("http://{}", first.local_addr().unwrap());
        let location = format!("http://{}/capture", destination.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let (mut socket, _) = first.accept().unwrap();
            socket.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
            let mut buffer = [0; 4096];
            let _ = socket.read(&mut buffer).unwrap();
            write!(socket, "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        let account = std::sync::Arc::new(context("one").bind(&base, "b".repeat(64)).unwrap());
        let custom = reqwest::Client::new();
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let response = runtime.block_on(AuthenticatedRequest::new(custom.clone(), Some(account), custom.get(&base).timeout(std::time::Duration::from_secs(2))).send()).unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        assert_eq!(destination.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
        worker.join().unwrap();
    }

}

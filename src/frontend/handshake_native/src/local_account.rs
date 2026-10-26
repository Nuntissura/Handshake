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
        f.debug_struct("AuthenticatedContext")
            .field("account_id", &self.account_id)
            .field("principal_id", &self.principal_id)
            .field("session_id", &self.session_id)
            .finish_non_exhaustive()
    }
}

impl AuthenticatedContext {
    pub fn invalidate(&self) {
        self.invalidated
            .store(true, std::sync::atomic::Ordering::Release);
    }
    pub fn is_active(&self) -> bool {
        !self.invalidated.load(std::sync::atomic::Ordering::Acquire)
    }
    pub fn observe_status(&self, status: reqwest::StatusCode) {
        if matches!(status.as_u16(), 401 | 403) {
            self.rejected
                .store(true, std::sync::atomic::Ordering::Release);
        }
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

    /// Test seam (MT-154): the SAME session (identity, tokens and shared invalidation/rejection state)
    /// re-bound to another backend origin, for proofs that route a live session through a loopback
    /// proxy or swap the mounted host onto an isolated fault-injection server. Never compiled into a
    /// production build.
    #[cfg(any(test, feature = "integration"))]
    #[doc(hidden)]
    pub fn rebound_to_origin_for_test(&self, backend: &str) -> Result<Self, String> {
        let channel_token = self.channel_token.clone();
        self.clone().bind(backend, channel_token)
    }

    pub fn authorize_builder(
        &self,
        _client: reqwest::Client,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, String> {
        Ok(reqwest::RequestBuilder::from_parts(
            crate::backend_client::shared_http_client(),
            self.authorize(request)?,
        ))
    }

    pub fn authorize(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Request, String> {
        if !self.is_active() {
            return Err("Account session is no longer active".into());
        }
        self.authorize_inner(request)
    }

    fn authorize_inner(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Request, String> {
        let mut request = request.build().map_err(|_| "Invalid backend request")?;
        if request.url().origin().ascii_serialization() != self.backend_origin {
            return Err("Authenticated backend origin mismatch".into());
        }
        request.headers_mut().insert(
            "x-hsk-session-token",
            self.session_token
                .parse()
                .map_err(|_| "Invalid session credential")?,
        );
        request.headers_mut().insert(
            "x-hsk-channel-binding-token",
            self.channel_token
                .parse()
                .map_err(|_| "Invalid channel credential")?,
        );
        Ok(request)
    }
}

pub struct AuthenticatedRequest {
    client: reqwest::Client,
    context: Option<std::sync::Arc<AuthenticatedContext>>,
    request: reqwest::RequestBuilder,
}

impl AuthenticatedRequest {
    pub fn new(
        _client: reqwest::Client,
        context: Option<std::sync::Arc<AuthenticatedContext>>,
        request: reqwest::RequestBuilder,
    ) -> Self {
        Self {
            client: crate::backend_client::shared_http_client(),
            context,
            request,
        }
    }
    pub async fn json(self) -> Result<serde_json::Value, String> {
        let context = self.context.clone();
        let response = self.send().await?;
        if !response.status().is_success() {
            return Err(format!("Protected request denied ({})", response.status()));
        }
        let value = response
            .json()
            .await
            .map_err(|_| "Invalid backend response".to_owned())?;
        if !context.as_ref().is_some_and(|context| context.is_active()) {
            return Err("Account session is no longer active".into());
        }
        Ok(value)
    }
    pub async fn send(self) -> Result<reqwest::Response, String> {
        let context = self
            .context
            .ok_or_else(|| "Account login required".to_owned())?;
        let request = context.authorize(self.request)?;
        let response = self
            .client
            .execute(request)
            .await
            .map_err(|error| send_error_detail(&error))?;
        context.observe_status(response.status());
        if !context.is_active() {
            return Err("Account session is no longer active".into());
        }
        Ok(response)
    }
}

/// MT-109 C2 (diagnosis d): keep the transport failure kind and source chain instead of the bare
/// "error sending request for url (...)" display, so a per-request timeout is distinguishable from a
/// refused or reset connection.
fn send_error_detail(error: &reqwest::Error) -> String {
    let kind = if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connect"
    } else if error.is_body() {
        "body"
    } else if error.is_request() {
        "request"
    } else {
        "other"
    };
    format!("{error} [kind={kind}; {error:?}]")
}

fn valid_token(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub async fn setup_required(
    _client: &reqwest::Client,
    backend: &str,
    channel: &str,
) -> Result<bool, String> {
    let client = &crate::backend_client::shared_http_client();
    let value = channel_request(client.get(format!("{backend}/authority/setup")), channel)?
        .send()
        .await
        .map_err(|_| "Backend unavailable")?;
    let value = checked_json(value).await?;
    value
        .get("setup_required")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| "Invalid setup response".into())
}

pub async fn setup_owner(
    _client: &reqwest::Client,
    backend: &str,
    channel: &str,
    account_name: &str,
    password: String,
) -> Result<(), String> {
    let client = &crate::backend_client::shared_http_client();
    let response = channel_request(client.post(format!("{backend}/authority/setup")), channel)?
        .json(&serde_json::json!({"account_name": account_name, "password": password}))
        .send()
        .await
        .map_err(|_| "Backend unavailable")?;
    checked_json(response).await.map(|_| ())
}

pub async fn login(
    _client: &reqwest::Client,
    backend: &str,
    channel: String,
    account_name: &str,
    password: String,
) -> Result<AuthenticatedContext, String> {
    let client = &crate::backend_client::shared_http_client();
    let credential = channel_request(client.post(format!("{backend}/authority/login")), &channel)?
        .json(&serde_json::json!({"account_name": account_name, "password": password}))
        .send()
        .await
        .map_err(|_| "Backend unavailable")?;
    #[cfg(feature = "integration")]
    if !credential.status().is_success() {
        eprintln!("NATIVE_AUTH_PROOF phase=owner-login status={}", credential.status());
    }
    let credential = checked_json(credential).await?;
    let response = channel_request(
        client.post(format!("{backend}/authority/session")),
        &channel,
    )?
    .json(&serde_json::json!({
        "account_id": credential.get("account_id"),
        "principal_id": credential.get("principal_id"),
        "access_space_id": credential.get("access_space_id"),
        "authentication_token": credential.get("token"),
    }))
    .send()
    .await
    .map_err(|_| "Backend unavailable")?;
    #[cfg(feature = "integration")]
    if !response.status().is_success() {
        eprintln!("NATIVE_AUTH_PROOF phase=session-exchange status={}", response.status());
    }
    let context: AuthenticatedContext = serde_json::from_value(checked_json(response).await?)
        .map_err(|_| "Invalid session response")?;
    context.bind(backend, channel)
}

pub async fn logout(
    _client: &reqwest::Client,
    backend: &str,
    context: &AuthenticatedContext,
) -> Result<(), String> {
    let client = &crate::backend_client::shared_http_client();
    let request = context.authorize_inner(client.post(format!("{backend}/authority/logout")))?;
    context.invalidate();
    let response = client
        .execute(request)
        .await
        .map_err(|_| "Backend unavailable")?;
    checked_json(response).await.map(|_| ())
}

pub async fn current_session(
    _client: &reqwest::Client,
    backend: &str,
    context: &AuthenticatedContext,
) -> Result<bool, String> {
    let client = &crate::backend_client::shared_http_client();
    let request = context.authorize(client.get(format!("{backend}/authority/session")))?;
    let response = client
        .execute(request)
        .await
        .map_err(|_| "Backend unavailable")?;
    if matches!(response.status().as_u16(), 401 | 403) {
        context.invalidate();
        return Ok(false);
    }
    let value = checked_json(response).await?;
    let matches = value["schema_version"].as_str() == Some("hsk.current_account@1")
        && value["session_id"].as_str() == Some(context.session_id.as_str())
        && value["account_id"].as_str() == Some(context.account_id.as_str())
        && value["principal_id"].as_str() == Some(context.principal_id.as_str())
        && value["access_space_id"].as_str() == Some(context.access_space_id.as_str());
    if !matches {
        context.invalidate();
    }
    Ok(matches)
}

pub use handshake_account_activation::WorkspaceActivationAcknowledgement;

#[derive(Debug)]
pub enum WorkspaceActivationError {
    Denied(String),
    Uncertain(String),
}

impl WorkspaceActivationError {
    pub fn is_uncertain(&self) -> bool {
        matches!(self, Self::Uncertain(_))
    }
}

impl std::fmt::Display for WorkspaceActivationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Denied(message) | Self::Uncertain(message) => f.write_str(message),
        }
    }
}

pub async fn activate_workspace(
    _client: &reqwest::Client,
    backend: &str,
    context: std::sync::Arc<AuthenticatedContext>,
    workspace_id: &str,
    operation_id: uuid::Uuid,
) -> Result<WorkspaceActivationAcknowledgement, WorkspaceActivationError> {
    let client = crate::backend_client::shared_http_client();
    let request = context
        .authorize(
            client
                .post(format!("{backend}/authority/session"))
                .json(&serde_json::json!({
                    "workspace_id": workspace_id,
                    "operation_id": operation_id,
                })),
        )
        .map_err(WorkspaceActivationError::Denied)?;
    let response = client
        .execute(request)
        .await
        .map_err(|error| WorkspaceActivationError::Uncertain(send_error_detail(&error)))?;
    let status = response.status();
    context.observe_status(status);
    if matches!(status.as_u16(), 400 | 401 | 403 | 404) {
        if matches!(status.as_u16(), 401 | 403) {
            context.invalidate();
        }
        return Err(WorkspaceActivationError::Denied(format!(
            "Workspace activation denied ({status})"
        )));
    }
    if !status.is_success() {
        return Err(WorkspaceActivationError::Uncertain(format!(
            "Workspace activation acknowledgement unavailable ({status})"
        )));
    }
    let value: serde_json::Value = response.json().await.map_err(|_| {
        WorkspaceActivationError::Uncertain("Invalid workspace activation acknowledgement".into())
    })?;
    let acknowledgement = handshake_account_activation::parse_workspace_activation_ack(
        value,
        &handshake_account_activation::WorkspaceActivationExpected {
            operation_id,
            owner_account_id: &context.account_id,
            actor_principal_id: &context.principal_id,
            authenticated_session_id: &context.session_id,
            access_space_id: &context.access_space_id,
            workspace_id,
        },
    )
    .map_err(|error| {
        use handshake_account_activation::WorkspaceActivationAcknowledgementError;
        let message = match error {
            WorkspaceActivationAcknowledgementError::InvalidSchema => {
                "Invalid workspace activation acknowledgement schema"
            }
            WorkspaceActivationAcknowledgementError::InvalidAcknowledgement => {
                "Invalid workspace activation acknowledgement"
            }
            WorkspaceActivationAcknowledgementError::Mismatch => {
                "Workspace activation acknowledgement mismatch"
            }
        };
        WorkspaceActivationError::Uncertain(message.into())
    })?;
    if !context.is_active() {
        return Err(WorkspaceActivationError::Uncertain(
            "Account session changed during workspace activation".into(),
        ));
    }
    Ok(acknowledgement)
}

fn channel_request(
    request: reqwest::RequestBuilder,
    channel: &str,
) -> Result<reqwest::RequestBuilder, String> {
    if !valid_token(channel) {
        return Err("Native channel unavailable".into());
    }
    Ok(request.header("x-hsk-channel-binding-token", channel))
}

async fn checked_json(response: reqwest::Response) -> Result<serde_json::Value, String> {
    if !response.status().is_success() {
        return Err("Account authentication denied".into());
    }
    response
        .json()
        .await
        .map_err(|_| "Invalid authentication response".into())
}

#[derive(Default)]
struct AccountTextCompletion {
    generation: u64,
    applied: Option<String>,
}

impl AccountTextCompletion {
    fn invalidate(&mut self) {
        if let Some(mut applied) = self.applied.take() {
            zeroize::Zeroize::zeroize(&mut applied);
        }
        self.generation = self.generation.saturating_add(1);
    }

    fn reconcile(&mut self, value: &str) {
        if self.applied.as_deref().is_some_and(|applied| applied != value) {
            self.invalidate();
        }
    }

    fn record_application(&mut self, value: &str) {
        let can_advance = self.generation.checked_add(1).is_some();
        self.invalidate();
        if can_advance { self.applied = Some(value.to_owned()); }
    }
}

fn account_text_edit(
    ui: &mut egui::Ui,
    label: &str,
    author_id: &str,
    value: &mut String,
    completion: &mut AccountTextCompletion,
    allow_actions: bool,
) {
    completion.reconcile(value);
    let id = egui::Id::new(author_id);
    let replacement = allow_actions.then(|| crate::mcp::accesskit_string_set_value(ui, id)).flatten();
    let label = ui.label(label);
    let response = ui.add(egui::TextEdit::singleline(value)
        .id(id).desired_width(150.0)).labelled_by(label.id);
    if let Some(replacement) = replacement {
        *value = replacement;
        completion.record_application(value);
        ui.ctx().request_repaint();
    } else {
        completion.reconcile(value);
    }
    crate::accessibility::emit_interactive_node(ui.ctx(), response.id, author_id);
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.add_action(egui::accesskit::Action::SetValue);
        node.set_value(value.clone());
    });
    let observer = crate::mcp::action::set_value_completion_author_id(author_id);
    let token = crate::mcp::action::serialize_set_value_completion(
        author_id, completion.generation, completion.applied.as_deref());
    ui.ctx().accesskit_node_builder(egui::Id::new(&observer), |node| {
        node.set_role(egui::accesskit::Role::Status);
        node.set_author_id(observer);
        node.set_label("Account input completion");
        if let Some(token) = token { node.set_value(token); }
    });
}

fn account_password_edit(ui: &mut egui::Ui, password: &mut String, allow_actions: bool) {
    let id = egui::Id::new("account.password");
    if allow_actions {
        if let Some(replacement) = crate::mcp::accesskit_string_set_value(ui, id) {
            zeroize::Zeroize::zeroize(password);
            *password = replacement;
            crate::mcp::argus::acknowledge_secret_set_value(
                ui.ctx(), "account.password", id, password);
        }
    }
    let label = ui.label("Password");
    let response = ui.add(egui::TextEdit::singleline(password)
        .id(id).desired_width(150.0).password(true)).labelled_by(label.id);
    crate::accessibility::emit_interactive_node(ui.ctx(), response.id, "account.password");
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.add_action(egui::accesskit::Action::SetValue);
        node.set_value("");
    });
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
    account_name_completion: AccountTextCompletion,
    workspace_name_completion: AccountTextCompletion,
    completion_owner: Option<std::sync::Weak<AuthenticatedContext>>,
    completion_owner_active: bool,
    setup_required: Option<bool>,
    error: Option<String>,
    pending: Option<std::sync::mpsc::Receiver<Result<AccountResult, String>>>,
}

enum AccountResult {
    Status(bool),
    Setup,
    Login(AuthenticatedContext),
    Logout,
    Current(bool),
    WorkspaceCreated,
}

impl AccountUi {
    fn refresh_text_completions(&mut self) {
        let same_owner = match (&self.completion_owner, &self.context) {
            (None, None) => true,
            (Some(previous), Some(current)) =>
                std::sync::Weak::ptr_eq(previous, &std::sync::Arc::downgrade(current)),
            _ => false,
        };
        let active = self.context.as_ref().is_some_and(|context| context.is_active());
        if !same_owner || active != self.completion_owner_active {
            self.account_name_completion.invalidate();
            self.workspace_name_completion.invalidate();
            self.completion_owner = self.context.as_ref().map(std::sync::Arc::downgrade);
            self.completion_owner_active = active;
        }
        self.account_name_completion.reconcile(&self.account_name);
        self.workspace_name_completion.reconcile(&self.workspace_name);
    }

    /// MT-109 C2 (diagnosis e): the bound context state and last account error, for proofs that must
    /// tell an account-gated frame from a swallowed UI intent.
    pub fn diagnostic(&self) -> (Option<bool>, Option<String>) {
        (
            self.context.as_ref().map(|context| context.is_active()),
            self.error.clone(),
        )
    }

    pub fn take_workspace_created(&mut self) -> bool {
        std::mem::take(&mut self.workspace_created)
    }
    pub fn take_reset_requested(&mut self) -> bool {
        std::mem::take(&mut self.reset_requested)
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        backend: &str,
        channel: String,
        runtime: Option<&tokio::runtime::Handle>,
        allow_actions: bool,
    ) {
        self.refresh_text_completions();
        if allow_actions {
            if let Some(receiver) = &self.pending {
                let result = match receiver.try_recv() {
                    Ok(result) => Some(result),
                    Err(std::sync::mpsc::TryRecvError::Empty) => None,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        Some(Err("Account operation interrupted. Retry.".into()))
                    }
                };
                if let Some(result) = result {
                    self.pending = None;
                    match result {
                        Ok(AccountResult::Status(required)) => self.setup_required = Some(required),
                        Ok(AccountResult::Setup) => self.setup_required = Some(false),
                        Ok(AccountResult::Login(context)) => {
                            self.context = Some(std::sync::Arc::new(context));
                            self.reset_requested = true;
                            self.confirm_discard = false;
                        }
                        Ok(AccountResult::Logout) => self.context = None,
                        Ok(AccountResult::Current(false)) => {
                            if let Some(context) = self.context.take() {
                                context.invalidate();
                            }
                            self.error = Some("Session rejected. Log in again.".into());
                        }
                        Ok(AccountResult::Current(true)) => {}
                        Ok(AccountResult::WorkspaceCreated) => {
                            self.workspace_created = true;
                            self.workspace_name.clear();
                            self.workspace_name_completion.invalidate();
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
            }
        }
        self.refresh_text_completions();
        let mut action = if allow_actions
            && self.pending.is_none()
            && self.context.as_ref().is_some_and(|context| {
                context
                    .rejected
                    .swap(false, std::sync::atomic::Ordering::AcqRel)
            }) {
            Some(4)
        } else {
            None
        };
        let mut action_author = None;
        let authenticated = self.context.is_some();
        let account_error = self.error.clone();
        let pending = self.pending.is_some();
        egui::TopBottomPanel::top("local-account").show(ctx, |ui| {
            let mut controls = |ui: &mut egui::Ui| { ui.horizontal_wrapped(|ui| {
                if let Some(context) = &self.context {
                    let current = ui.label("Signed in").on_hover_text(format!(
                        "Account {} · Space {}",
                        context.account_id, context.access_space_id
                    ));
                    crate::accessibility::emit_interactive_node(ctx, current.id, "account.current");
                    let check =
                        ui.add_enabled(self.pending.is_none(), egui::Button::new("Check session"));
                    crate::accessibility::emit_interactive_node(
                        ctx,
                        check.id,
                        "account.check-session",
                    );
                    if allow_actions && check.clicked() {
                        action = Some(4);
                        action_author = Some("account.check-session");
                    }
                    let button =
                        ui.add_enabled(self.pending.is_none(), egui::Button::new("Log out"));
                    crate::accessibility::emit_interactive_node(ctx, button.id, "account.logout");
                    if allow_actions && button.clicked() {
                        self.confirm_discard = true;
                        crate::mcp::argus::acknowledge_action_effect_allow_target_transition(ctx, "account.logout");
                    }
                    account_text_edit(ui, "New workspace name", "account.workspace-name",
                        &mut self.workspace_name, &mut self.workspace_name_completion, allow_actions);
                    let create = ui.add_enabled(
                        self.pending.is_none() && !self.workspace_name.trim().is_empty(),
                        egui::Button::new("Create workspace"),
                    );
                    crate::accessibility::emit_interactive_node(
                        ctx,
                        create.id,
                        "account.create-workspace",
                    );
                    if allow_actions && create.clicked() {
                        action = Some(5);
                        action_author = Some("account.create-workspace");
                    }
                } else {
                    ui.label("Account (Owner setup password: 12–1024 UTF-8 bytes)");
                    account_text_edit(ui, "Account name", "account.name", &mut self.account_name,
                        &mut self.account_name_completion, allow_actions);
                    account_password_edit(ui, &mut self.password, allow_actions);
                    let button = ui.add_enabled(
                        self.pending.is_none(),
                        egui::Button::new(if self.setup_required == Some(true) {
                            "Set up Owner"
                        } else {
                            "Log in"
                        }),
                    );
                    crate::accessibility::emit_interactive_node(ctx, button.id, "account.submit");
                    if allow_actions && button.clicked() {
                        if self.setup_required == Some(true) {
                            action = Some(1);
                            action_author = Some("account.submit");
                        } else {
                            self.confirm_discard = true;
                            crate::mcp::argus::acknowledge_action_effect_allow_target_transition(ctx, "account.submit");
                        }
                    }
                    let check =
                        ui.add_enabled(self.pending.is_none(), egui::Button::new("Check setup"));
                    crate::accessibility::emit_interactive_node(
                        ctx,
                        check.id,
                        "account.check-setup",
                    );
                    if allow_actions && check.clicked() {
                        action = Some(0);
                        action_author = Some("account.check-setup");
                    }
                }
                if self.confirm_discard {
                    ui.label("Continuing discards unsaved workspace edits. Cancel to keep them.");
                    let confirm = ui.button(if self.context.is_some() {
                        "Discard edits and log out"
                    } else {
                        "Discard edits and log in"
                    });
                    crate::accessibility::emit_interactive_node(
                        ctx,
                        confirm.id,
                        "account.confirm-discard",
                    );
                    if allow_actions && confirm.clicked() {
                        action = Some(if self.context.is_some() { 3 } else { 2 });
                        action_author = Some("account.confirm-discard");
                        self.confirm_discard = false;
                    }
                    let cancel = ui.button("Cancel");
                    crate::accessibility::emit_interactive_node(
                        ctx,
                        cancel.id,
                        "account.cancel-discard",
                    );
                    if allow_actions && cancel.clicked() {
                        self.confirm_discard = false;
                        crate::mcp::argus::acknowledge_action_effect_allow_target_transition(ctx, "account.cancel-discard");
                    }
                }
                if self.pending.is_some() {
                    ui.spinner();
                }
                if let Some(error) = &self.error {
                    let label = ui.label(error);
                    crate::accessibility::emit_interactive_node(ctx, label.id, "account.error");
                }
            }); };
            if authenticated {
                ui.horizontal(|ui| {
                    let label = if account_error.is_some() {
                        egui::RichText::new("Account").color(ui.visuals().error_fg_color)
                    } else { egui::RichText::new("Account") };
                    let (response, menu) = egui::containers::menu::MenuButton::new(label)
                        .config(egui::containers::menu::MenuConfig::new()
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside))
                        .ui(ui, |ui| {
                            ui.set_min_width(240.0);
                            ui.set_max_width(360.0);
                            controls(ui);
                        });
                    crate::accessibility::emit_interactive_node(ctx, response.id, "account.menu");
                    if allow_actions && response.clicked() {
                        crate::mcp::argus::acknowledge_action_effect_allow_target_transition(ctx, "account.menu");
                    }
                    if let Some(error) = &account_error {
                        ctx.accesskit_node_builder(response.id, |node| node.set_description(error.clone()));
                        response.on_hover_text(error);
                    }
                    if pending && menu.is_none() { ui.spinner(); }
                });
            } else { controls(ui); }
        });
        if !allow_actions {
            return;
        }
        if let Some(action) = action {
            let Some(runtime) = runtime else {
                self.error = Some("Backend runtime unavailable".into());
                return;
            };
            let client = crate::backend_client::shared_http_client();
            let backend = backend.to_owned();
            let name = self.account_name.clone();
            let workspace_name = self.workspace_name.clone();
            let password = if matches!(action, 1 | 2) {
                std::mem::take(&mut self.password)
            } else {
                String::new()
            };
            let context = self.context.clone();
            if action == 3 {
                if let Some(context) = self.context.take() {
                    context.invalidate();
                }
                self.refresh_text_completions();
                self.reset_requested = true;
            }
            let repaint = ctx.clone();
            let (sender, receiver) = std::sync::mpsc::channel();
            self.pending = Some(receiver);
            self.error = None;
            runtime.spawn(async move {
                let result = match action {
                    0 => setup_required(&client, &backend, &channel)
                        .await
                        .map(AccountResult::Status),
                    1 => setup_owner(&client, &backend, &channel, &name, password)
                        .await
                        .map(|_| AccountResult::Setup),
                    2 => login(&client, &backend, channel, &name, password)
                        .await
                        .map(AccountResult::Login),
                    5 => match context {
                        Some(context) => {
                            let request = client
                                .post(format!("{backend}/workspaces"))
                                .json(&serde_json::json!({"name": workspace_name}));
                            match AuthenticatedRequest::new(client.clone(), Some(context), request)
                                .send()
                                .await
                            {
                                Ok(response) => checked_json(response)
                                    .await
                                    .map(|_| AccountResult::WorkspaceCreated),
                                Err(error) => Err(error),
                            }
                        }
                        None => Err("No active account".into()),
                    },
                    4 => match context {
                        Some(context) => current_session(&client, &backend, &context)
                            .await
                            .map(AccountResult::Current),
                        None => Err("No active account".into()),
                    },
                    _ => match context {
                        Some(context) => logout(&client, &backend, &context)
                            .await
                            .map(|_| AccountResult::Logout),
                        None => Err("No active account".into()),
                    },
                };
                let _ = sender.send(result);
                repaint.request_repaint();
            });
            if let Some(author_id) = action_author {
                crate::mcp::argus::acknowledge_action_effect_allow_target_transition(ctx, author_id);
            }
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

    fn account_argus_frame(
        account: &mut AccountUi,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        allow_actions: bool,
    ) -> crate::accessibility::UiTreeSnapshot {
        let output = ctx.run(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))),
            events,
            ..Default::default()
        }, |ctx| account.show(ctx, "http://127.0.0.1:9911", "b".repeat(64), None, allow_actions));
        crate::accessibility::collect_ui_tree_snapshot(
            &output.platform_output.accesskit_update.expect("actual account frame"))
    }

    fn account_text_argus_frame(
        value: &mut String,
        completion: &mut AccountTextCompletion,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> crate::accessibility::UiTreeSnapshot {
        let output = ctx.run(egui::RawInput { events, focused: true, ..Default::default() }, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                account_text_edit(ui, "New workspace name", "account.workspace-name", value, completion, true);
            });
        });
        crate::accessibility::collect_ui_tree_snapshot(&output.platform_output.accesskit_update.unwrap())
    }

    #[test]
    fn account_text_clear_and_manual_edit_retire_payload_and_cannot_revive_old_receipt() {
        let canary = "prior-workspace-input-completion-canary";
        for change in ["clear", "replace", "typing"] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let mut value = String::new();
            let mut completion = AccountTextCompletion::default();
            let snapshot = account_text_argus_frame(&mut value, &mut completion, &ctx, Vec::new());
            let mut channel = crate::mcp::ActionChannel::new();
            channel.enqueue(&snapshot, "account.workspace-name",
                crate::mcp::UiAction::SetValue { text: canary.to_owned() }).unwrap();
            let events = channel.drain_revalidated_into_live_events(&snapshot, &ctx, egui::ViewportId::ROOT);
            account_text_argus_frame(&mut value, &mut completion, &ctx, events);
            let applied_generation = completion.generation;
            assert_eq!(completion.applied.as_deref(), Some(canary));
            let events = match change {
                "clear" => { value.clear(); Vec::new() },
                "replace" => { value = "Operator replacement".to_owned(); Vec::new() },
                "typing" => {
                    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("account.workspace-name")));
                    vec![egui::Event::Text("-operator-edit".to_owned())]
                }
                _ => unreachable!(),
            };
            let changed = account_text_argus_frame(&mut value, &mut completion, &ctx, events);
            assert!(completion.applied.is_none(), "{change}");
            assert!(completion.generation > applied_generation, "{change}");
            let observer = changed.find_by_author_id("account.workspace-name.set-value-completion").unwrap();
            assert!(!observer.value.as_deref().unwrap().contains(canary), "{change}");
            value = canary.to_owned();
            let later = account_text_argus_frame(&mut value, &mut completion, &ctx, Vec::new());
            channel.acknowledge_after_live_render(&later, Some(&ctx));
            assert_ne!(channel.receipts()[0].status, crate::mcp::ActionReceiptStatus::Applied, "{change}");
        }
    }

    #[test]
    fn account_workspace_created_clears_completion_while_menu_is_closed() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut account = AccountUi {
            context: Some(std::sync::Arc::new(context("one"))),
            ..AccountUi::default()
        };
        account_argus_frame(&mut account, &ctx, Vec::new(), true);
        let canary = "created-workspace-completion-canary";
        account.workspace_name = canary.to_owned();
        account.workspace_name_completion.record_application(canary);
        let generation = account.workspace_name_completion.generation;
        let (sender, receiver) = std::sync::mpsc::channel();
        assert!(sender.send(Ok(AccountResult::WorkspaceCreated)).is_ok());
        account.pending = Some(receiver);
        account_argus_frame(&mut account, &ctx, Vec::new(), true);
        assert!(account.take_workspace_created());
        assert!(account.workspace_name.is_empty());
        assert!(account.workspace_name_completion.applied.is_none());
        assert!(account.workspace_name_completion.generation > generation);
        let later = account_text_argus_frame(&mut account.workspace_name,
            &mut account.workspace_name_completion, &ctx, Vec::new());
        assert!(!later.to_json().contains(canary));
    }

    #[test]
    fn account_owner_transitions_fence_preserved_completion_state_without_reusing_generations() {
        for transition in ["rebind", "login", "logout", "reject", "invalidate"] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let mut account = AccountUi {
                context: Some(std::sync::Arc::new(context("same"))),
                ..AccountUi::default()
            };
            account_argus_frame(&mut account, &ctx, Vec::new(), true);
            account.account_name = "prior-owner-account-completion-canary".to_owned();
            account.workspace_name = "prior-owner-workspace-completion-canary".to_owned();
            account.account_name_completion.record_application(&account.account_name);
            account.workspace_name_completion.record_application(&account.workspace_name);
            let generations = (account.account_name_completion.generation, account.workspace_name_completion.generation);
            match transition {
                "rebind" => account.context = Some(std::sync::Arc::new(context("same"))),
                "invalidate" => account.context.as_ref().unwrap().invalidate(),
                other => {
                    let result = match other {
                        "login" => AccountResult::Login(context("same")),
                        "logout" => AccountResult::Logout,
                        "reject" => AccountResult::Current(false),
                        _ => unreachable!(),
                    };
                    let (sender, receiver) = std::sync::mpsc::channel();
                    assert!(sender.send(Ok(result)).is_ok());
                    account.pending = Some(receiver);
                }
            }
            let mut transferred = std::mem::take(&mut account);
            account_argus_frame(&mut transferred, &ctx, Vec::new(), true);
            assert!(transferred.account_name_completion.applied.is_none(), "{transition}");
            assert!(transferred.workspace_name_completion.applied.is_none(), "{transition}");
            assert!(transferred.account_name_completion.generation > generations.0, "{transition}");
            assert!(transferred.workspace_name_completion.generation > generations.1, "{transition}");
            transferred.refresh_text_completions();
            assert!(transferred.account_name_completion.applied.is_none());
            assert!(transferred.workspace_name_completion.applied.is_none());
        }
        let mut completion = AccountTextCompletion { generation: u64::MAX - 1, applied: None };
        completion.record_application("last-generation");
        assert_eq!(completion.generation, u64::MAX);
        completion.invalidate();
        completion.record_application("must-not-wrap");
        assert_eq!(completion.generation, u64::MAX);
        assert!(completion.applied.is_none());
    }

    #[test]
    fn account_argus_applies_name_and_private_password_through_actual_controls() {
        let mut account = AccountUi::default();
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        for (target, value) in [("account.name", "Owner input"),
            ("account.password", "private-account-input-canary")] {
            let snapshot = account_argus_frame(&mut account, &ctx, Vec::new(), true);
            let mut channel = crate::mcp::ActionChannel::new();
            channel.enqueue(&snapshot, target, crate::mcp::UiAction::SetValue { text: value.to_owned() }).unwrap();
            let events = channel.drain_revalidated_into_live_events(&snapshot, &ctx, egui::ViewportId::ROOT);
            let applied = account_argus_frame(&mut account, &ctx, events, true);
            channel.acknowledge_after_live_render(&applied, Some(&ctx));
            let later = account_argus_frame(&mut account, &ctx, Vec::new(), true);
            channel.acknowledge_after_live_render(&later, Some(&ctx));
            assert_eq!(channel.receipts()[0].status, crate::mcp::ActionReceiptStatus::Applied);
            if target == "account.password" {
                assert_eq!(account.password, value);
                assert_eq!(later.find_by_author_id(target).unwrap().value, None);
                assert!(!later.to_json().contains(value));
                assert!(!serde_json::to_string(&channel.receipts()).unwrap().contains(value));
            } else { assert_eq!(account.account_name, value); }
        }
    }

    #[test]
    fn account_argus_capture_cannot_apply_or_acknowledge_private_input() {
        let mut account = AccountUi::default();
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let snapshot = account_argus_frame(&mut account, &ctx, Vec::new(), true);
        let mut channel = crate::mcp::ActionChannel::new();
        let secret = "capture-must-not-consume-account-password";
        channel.enqueue(&snapshot, "account.password", crate::mcp::UiAction::SetValue { text: secret.to_owned() }).unwrap();
        let events = channel.drain_revalidated_into_live_events(&snapshot, &ctx, egui::ViewportId::ROOT);
        let capture = account_argus_frame(&mut account, &ctx, events, false);
        channel.acknowledge_after_render(&capture);
        let later = account_argus_frame(&mut account, &ctx, Vec::new(), true);
        channel.acknowledge_after_live_render(&later, Some(&ctx));
        assert!(account.password.is_empty());
        assert_ne!(channel.receipts()[0].status, crate::mcp::ActionReceiptStatus::Applied);
        assert!(!later.to_json().contains(secret));
        assert!(!serde_json::to_string(&channel.receipts()).unwrap().contains(secret));
    }

    #[test]
    fn account_argus_confirmation_clicks_acknowledge_actual_local_transitions() {
        let mut account = AccountUi::default();
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        for target in ["account.submit", "account.cancel-discard"] {
            let snapshot = account_argus_frame(&mut account, &ctx, Vec::new(), true);
            let mut channel = crate::mcp::ActionChannel::new();
            channel.enqueue(&snapshot, target, crate::mcp::UiAction::Click).unwrap();
            let events = channel.drain_revalidated_into_live_events(&snapshot, &ctx, egui::ViewportId::ROOT);
            let applied = account_argus_frame(&mut account, &ctx, events, true);
            channel.acknowledge_after_live_render(&applied, Some(&ctx));
            let later = account_argus_frame(&mut account, &ctx, Vec::new(), true);
            channel.acknowledge_after_live_render(&later, Some(&ctx));
            assert_eq!(channel.receipts()[0].status, crate::mcp::ActionReceiptStatus::Applied);
            assert_eq!(account.confirm_discard, target == "account.submit");
        }
        let snapshot = account_argus_frame(&mut account, &ctx, Vec::new(), true);
        let mut channel = crate::mcp::ActionChannel::new();
        channel.enqueue(&snapshot, "account.check-setup", crate::mcp::UiAction::Click).unwrap();
        let events = channel.drain_revalidated_into_live_events(&snapshot, &ctx, egui::ViewportId::ROOT);
        let failed = account_argus_frame(&mut account, &ctx, events, true);
        channel.acknowledge_after_live_render(&failed, Some(&ctx));
        let later = account_argus_frame(&mut account, &ctx, Vec::new(), true);
        channel.acknowledge_after_live_render(&later, Some(&ctx));
        assert_eq!(account.error.as_deref(), Some("Backend runtime unavailable"));
        assert!(account.pending.is_none());
        assert_ne!(channel.receipts()[0].status, crate::mcp::ActionReceiptStatus::Applied);
    }

    #[test]
    fn account_argus_workspace_name_uses_the_same_production_text_binding() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut value = String::new();
        let mut completion = AccountTextCompletion::default();
        let mut frame = |events| {
            let output = ctx.run(egui::RawInput { events, ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    account_text_edit(ui, "New workspace name", "account.workspace-name", &mut value,
                        &mut completion, true);
                });
            });
            crate::accessibility::collect_ui_tree_snapshot(&output.platform_output.accesskit_update.unwrap())
        };
        let snapshot = frame(Vec::new());
        let mut channel = crate::mcp::ActionChannel::new();
        channel.enqueue(&snapshot, "account.workspace-name", crate::mcp::UiAction::SetValue { text: "Owned workspace".to_owned() }).unwrap();
        let events = channel.drain_revalidated_into_live_events(&snapshot, &ctx, egui::ViewportId::ROOT);
        let applied = frame(events);
        channel.acknowledge_after_live_render(&applied, Some(&ctx));
        let later = frame(Vec::new());
        channel.acknowledge_after_live_render(&later, Some(&ctx));
        assert_eq!(channel.receipts()[0].status, crate::mcp::ActionReceiptStatus::Applied);
        assert_eq!(value, "Owned workspace");
    }

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
        let request = account
            .authorize(client.get("http://127.0.0.1:9911/authority/session"))
            .unwrap();
        assert_eq!(request.headers()["x-hsk-session-token"], "a".repeat(64));
        assert_eq!(
            request.headers()["x-hsk-channel-binding-token"],
            "b".repeat(64)
        );
        assert!(account
            .authorize(client.get("http://127.0.0.1:9912/authority/session"))
            .is_err());
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
        assert!(in_flight
            .authorize(
                crate::backend_client::shared_http_client()
                    .get("http://127.0.0.1:9911/knowledge/documents")
            )
            .is_err());
    }

    #[test]
    fn disconnected_account_worker_releases_pending_state_without_discarding_workspace() {
        let mut account = AccountUi::default();
        let (sender, receiver) = std::sync::mpsc::channel();
        account.pending = Some(receiver);
        drop(sender);
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            account.show(ctx, "http://127.0.0.1:9911", "b".repeat(64), None, true)
        });
        assert!(account.pending.is_none());
        assert!(account.error.as_deref().unwrap().contains("interrupted"));
        assert!(!account.take_reset_requested());
    }

    #[test]
    fn authenticated_account_controls_start_inside_a_stable_compact_menu() {
        let mut account = AccountUi {
            context: Some(std::sync::Arc::new(context("one"))),
            ..AccountUi::default()
        };
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            account.show(ctx, "http://127.0.0.1:9911", "b".repeat(64), None, true)
        });
        let update = output.platform_output.accesskit_update.unwrap();
        let snapshot = crate::accessibility::collect_ui_tree_snapshot(&update);
        assert!(snapshot.find_by_author_id("account.menu").is_some());
        for id in ["account.current", "account.check-session", "account.logout",
            "account.workspace-name", "account.create-workspace"] {
            assert!(snapshot.find_by_author_id(id).is_none(), "closed Account menu exposed {id}");
        }
        assert!(account.context.as_ref().unwrap().is_active());
    }

    #[test]
    fn closed_account_menu_does_not_pause_pending_session_rejection() {
        let context = std::sync::Arc::new(context("one"));
        let (sender, receiver) = std::sync::mpsc::channel();
        assert!(sender.send(Ok(AccountResult::Current(false))).is_ok());
        let mut account = AccountUi {
            context: Some(context.clone()), pending: Some(receiver),
            ..AccountUi::default()
        };
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            account.show(ctx, "http://127.0.0.1:9911", "b".repeat(64), None, true)
        });
        assert!(account.pending.is_none());
        assert!(account.context.is_none());
        assert!(!context.is_active());
        assert_eq!(account.error.as_deref(), Some("Session rejected. Log in again."));
    }

    #[test]
    fn discard_confirmation_does_not_drop_context_before_confirmation() {
        let mut account = AccountUi {
            context: Some(std::sync::Arc::new(context("one"))),
            confirm_discard: true,
            ..AccountUi::default()
        };
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            account.show(ctx, "http://127.0.0.1:9911", "b".repeat(64), None, true)
        });
        assert!(account.context.as_ref().unwrap().is_active());
        assert!(!account.take_reset_requested());
    }
    #[test]
    fn account_accesskit_and_argus_snapshot_never_publish_password() {
        let secret = "unique-account-password-DO-NOT-PUBLISH-4389";
        for width in [1200.0, 480.0, 320.0] {
            let mut account = AccountUi {
                password: secret.into(),
                setup_required: Some(true),
                ..AccountUi::default()
            };
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 800.0),
                )),
                ..Default::default()
            };
            let output = ctx.run(input, |ctx| {
                account.show(ctx, "http://127.0.0.1:9911", "b".repeat(64), None, true)
            });
            let update = output
                .platform_output
                .accesskit_update
                .expect("actual account AccessKit frame");
            assert!(crate::accessibility::assert_no_unnamed_interactive(&update) > 0);
            assert!(
                !format!("{update:?}").contains(secret),
                "AccessKit must not expose the password"
            );
            let snapshot = crate::accessibility::collect_ui_tree_snapshot(&update);
            assert!(
                !snapshot.to_json().contains(secret),
                "Argus snapshot must not expose the password"
            );
            for id in [
                "account.name",
                "account.password",
                "account.submit",
                "account.check-setup",
            ] {
                assert!(
                    snapshot.find_by_author_id(id).is_some(),
                    "missing {id} at width {width}"
                );
            }
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
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut buffer = [0; 4096];
            let _ = socket.read(&mut buffer).unwrap();
            write!(socket, "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        let account = std::sync::Arc::new(context("one").bind(&base, "b".repeat(64)).unwrap());
        let custom = reqwest::Client::new();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let response = runtime
            .block_on(
                AuthenticatedRequest::new(
                    custom.clone(),
                    Some(account),
                    custom.get(&base).timeout(std::time::Duration::from_secs(2)),
                )
                .send(),
            )
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        assert_eq!(
            destination.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        worker.join().unwrap();
    }
}

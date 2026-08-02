use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::HsPalette;

/// Stable AccessKit author_id for the Runtime Chat pane container.
pub const RUNTIME_CHAT_PANEL_AUTHOR_ID: &str = "runtime-chat-panel";
/// Stable AccessKit author_id for the Runtime Chat text input.
pub const RUNTIME_CHAT_INPUT_AUTHOR_ID: &str = "runtime-chat-input";
/// Stable AccessKit author_id for the Runtime Chat endpoint/status node.
pub const RUNTIME_CHAT_STATUS_AUTHOR_ID: &str = "runtime-chat-status";
/// Stable AccessKit author_id for the Runtime Chat send button.
pub const RUNTIME_CHAT_SEND_AUTHOR_ID: &str = "runtime-chat-send";
/// Stable AccessKit author_id for cancelling the exact active Runtime Chat request generation.
pub const RUNTIME_CHAT_CANCEL_AUTHOR_ID: &str = "runtime-chat-cancel";
/// Stable author_id for one transcript role label.
pub fn runtime_chat_turn_role_author_id(index: usize) -> String {
    format!("runtime-chat-turn-{index}-role")
}
/// Stable author_id for one transcript body label.
pub fn runtime_chat_turn_body_author_id(index: usize) -> String {
    format!("runtime-chat-turn-{index}-body")
}

/// MT-098 names `POST /chat` as the planned native backend bridge. handshake_core currently has no
/// matching router entry, so a real request to this path receives the router's 404 fallback.
const PRODUCTION_CHAT_ENDPOINT: &str = "/chat";
/// Deliberately unsupported method used to ask the composed HTTP router whether `/chat` is registered.
/// Axum returns 404 when the path is absent and 405 when a path-specific method router exists. Using a
/// method capability signal keeps a real POST handler's own bare 404 distinct from router absence.
const ROUTE_CAPABILITY_METHOD: &[u8] = b"HSK-CAPABILITY";
const ENDPOINT_MISSING_SUMMARY: &str =
    "Runtime Chat endpoint missing. No assistant reply was generated.";

type RuntimeChatDeliveryCell = Arc<Mutex<VecDeque<RuntimeChatDelivery>>>;

#[derive(Debug)]
struct RuntimeChatDelivery {
    generation: u64,
    result: Result<(), ChatSendError>,
}

#[derive(Debug)]
struct ActiveRuntimeChatSend {
    generation: u64,
    task: tokio::task::JoinHandle<()>,
}

/// Role of a chat transcript turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

/// One local chat transcript turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatTurn {
    pub role: ChatRole,
    pub body: String,
}

/// Typed send failure for the production Runtime Chat client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatSendError {
    /// The caller attempted to send an empty/whitespace-only draft.
    EmptyMessage,
    /// A send is already active. Runtime Chat permits exactly one request at a time so a stale or
    /// reordered completion cannot clear or overwrite the current request's visible state.
    AlreadyInFlight { generation: u64 },
    /// The frontend inspected the native backend surface and there is no HTTP chat route to call.
    EndpointMissing { probed_path: String },
    /// The local backend could not be reached or did not complete the bounded request.
    Transport { probed_path: String, detail: String },
    /// The path exists or a non-fallback response was returned, but it rejected the probe.
    HttpStatus { probed_path: String, status: u16 },
    /// A success status cannot be treated as a chat round-trip until handshake_core defines a response
    /// contract. This prevents an unrelated catch-all route from fabricating assistant success.
    ResponseContractMissing { probed_path: String, status: u16 },
}

impl ChatSendError {
    pub fn endpoint_missing() -> Self {
        Self::EndpointMissing {
            probed_path: PRODUCTION_CHAT_ENDPOINT.to_owned(),
        }
    }

    pub fn is_endpoint_missing(&self) -> bool {
        matches!(self, Self::EndpointMissing { .. })
    }

    pub fn is_empty_message(&self) -> bool {
        matches!(self, Self::EmptyMessage)
    }

    pub fn is_already_in_flight(&self) -> bool {
        matches!(self, Self::AlreadyInFlight { .. })
    }

    pub fn probed_path(&self) -> Option<&str> {
        match self {
            Self::EmptyMessage | Self::AlreadyInFlight { .. } => None,
            Self::EndpointMissing { probed_path }
            | Self::Transport { probed_path, .. }
            | Self::HttpStatus { probed_path, .. }
            | Self::ResponseContractMissing { probed_path, .. } => Some(probed_path),
        }
    }
}

impl fmt::Display for ChatSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMessage => write!(f, "EmptyMessage: Runtime Chat draft is empty"),
            Self::AlreadyInFlight { generation } => {
                write!(
                    f,
                    "AlreadyInFlight: Runtime Chat send {generation} is still active"
                )
            }
            Self::EndpointMissing { probed_path } => {
                write!(f, "EndpointMissing: {probed_path}")
            }
            Self::Transport {
                probed_path,
                detail,
            } => write!(f, "Transport: POST {probed_path}: {detail}"),
            Self::HttpStatus {
                probed_path,
                status,
            } => write!(f, "HttpStatus: POST {probed_path} returned {status}"),
            Self::ResponseContractMissing {
                probed_path,
                status,
            } => write!(
                f,
                "ResponseContractMissing: POST {probed_path} returned {status}"
            ),
        }
    }
}

/// Production client for Runtime Chat.
///
/// It intentionally does not target Flight Recorder runtime-chat event ingestion: observability is not
/// an assistant chat send/receive route. `send` performs a real, bounded POST off the UI thread. The
/// current handshake_core router has no `/chat` entry, so its real 404 becomes `EndpointMissing`.
#[derive(Debug, Clone)]
pub struct RuntimeChatClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
    probed_path: String,
}

impl RuntimeChatClient {
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: crate::backend_client::shared_http_client(),
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            runtime,
            probed_path: PRODUCTION_CHAT_ENDPOINT.to_owned(),
        }
    }

    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(crate::backend_client::BACKEND_BASE_URL, runtime)
    }

    pub fn probed_path(&self) -> &str {
        &self.probed_path
    }

    fn endpoint_url(&self) -> String {
        format!("{}{}", self.base_url, self.probed_path)
    }

    /// Dispatch the real local transport probe. The payload uses `prompt`, the authoritative legacy
    /// `kernel_swarm_chat_generate(instance_id, prompt)` message field, but does not claim that the
    /// absent HTTP route has a response schema. Completion is delivered to the panel's frame-drained
    /// queue so the egui thread never blocks on network I/O.
    fn send(
        &self,
        message: &str,
        generation: u64,
        delivery: RuntimeChatDeliveryCell,
        repaint: Option<egui::Context>,
    ) -> Result<tokio::task::JoinHandle<()>, ChatSendError> {
        if message.trim().is_empty() {
            return Err(ChatSendError::EmptyMessage);
        }

        let client = self.client.clone();
        let url = self.endpoint_url();
        let probed_path = self.probed_path.clone();
        let prompt = message.to_owned();
        let task = self.runtime.spawn(async move {
            let result = match client
                .post(&url)
                .json(&serde_json::json!({ "prompt": prompt }))
                .send()
                .await
            {
                Err(error) => Err(ChatSendError::Transport {
                    probed_path: probed_path.clone(),
                    detail: error.to_string(),
                }),
                Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => {
                    // A bare route-handler 404 is byte-for-byte indistinguishable from Axum's
                    // unmatched-route response. Ask the composed router with a deliberately unsupported
                    // method instead: absent paths remain 404, while a registered POST method router
                    // answers 405. Any non-404 probe is conservatively treated as route-present so
                    // Runtime Chat never claims EndpointMissing without an explicit router signal.
                    let capability_method = reqwest::Method::from_bytes(ROUTE_CAPABILITY_METHOD)
                        .expect("static Runtime Chat capability method is valid HTTP");
                    match client.request(capability_method, &url).send().await {
                        Ok(capability) if capability.status() == reqwest::StatusCode::NOT_FOUND => {
                            Err(ChatSendError::EndpointMissing {
                                probed_path: probed_path.clone(),
                            })
                        }
                        Ok(_) => Err(ChatSendError::HttpStatus {
                            probed_path: probed_path.clone(),
                            status: reqwest::StatusCode::NOT_FOUND.as_u16(),
                        }),
                        Err(error) => Err(ChatSendError::Transport {
                            probed_path: probed_path.clone(),
                            detail: format!("route capability probe failed: {error}"),
                        }),
                    }
                }
                Ok(response) if response.status().is_success() => {
                    Err(ChatSendError::ResponseContractMissing {
                        probed_path: probed_path.clone(),
                        status: response.status().as_u16(),
                    })
                }
                Ok(response) => Err(ChatSendError::HttpStatus {
                    probed_path: probed_path.clone(),
                    status: response.status().as_u16(),
                }),
            };
            delivery
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push_back(RuntimeChatDelivery { generation, result });
            if let Some(ctx) = repaint {
                ctx.request_repaint();
            }
        });
        Ok(task)
    }
}

/// The live Runtime Chat pane state.
#[derive(Debug)]
pub struct RuntimeChatPanel {
    client: RuntimeChatClient,
    palette: HsPalette,
    draft: String,
    turns: Vec<ChatTurn>,
    last_error: Option<ChatSendError>,
    last_cancelled_generation: Option<u64>,
    deliveries: RuntimeChatDeliveryCell,
    next_send_generation: u64,
    active_send: Option<ActiveRuntimeChatSend>,
    terminal_send_generation: Option<u64>,
    terminal_send_outcome: Option<String>,
    focus_owner_author_id: Option<String>,
}

impl RuntimeChatPanel {
    pub fn new(client: RuntimeChatClient, palette: HsPalette) -> Self {
        Self {
            client,
            palette,
            draft: String::new(),
            turns: vec![ChatTurn {
                role: ChatRole::System,
                body: "Chat backend route is not available in this build.".to_owned(),
            }],
            last_error: None,
            last_cancelled_generation: None,
            deliveries: Arc::new(Mutex::new(VecDeque::new())),
            next_send_generation: 0,
            active_send: None,
            terminal_send_generation: None,
            terminal_send_outcome: None,
            focus_owner_author_id: None,
        }
    }

    pub fn production(palette: HsPalette, runtime: tokio::runtime::Handle) -> Self {
        Self::new(RuntimeChatClient::production(runtime), palette)
    }

    pub fn set_palette(&mut self, palette: HsPalette) {
        self.palette = palette;
    }

    /// Replace the transport while preserving the mounted panel/factory identity. Used by the live app
    /// test seam and by future explicit backend-base reconfiguration.
    pub fn rebind_client(&mut self, client: RuntimeChatClient) {
        self.cancel_active_send();
        self.client = client;
        self.last_error = None;
        self.last_cancelled_generation = None;
        self.terminal_send_generation = None;
        self.terminal_send_outcome = None;
    }

    pub fn set_draft_for_test(&mut self, draft: impl Into<String>) {
        self.draft = draft.into();
    }

    pub fn turns_for_test(&self) -> &[ChatTurn] {
        &self.turns
    }

    pub fn last_error_for_test(&self) -> Option<&ChatSendError> {
        self.last_error.as_ref()
    }

    pub fn send_current_message_for_test(&mut self) -> Result<(), ChatSendError> {
        self.send_current_message(None)
    }

    pub fn drain_deliveries_for_test(&mut self) {
        self.drain_deliveries();
    }

    pub fn send_in_flight_for_test(&self) -> bool {
        self.active_send.is_some()
    }

    pub fn active_send_generation_for_test(&self) -> Option<u64> {
        self.active_send.as_ref().map(|active| active.generation)
    }

    /// Most recently allocated request generation, retained after terminal delivery/cancellation so
    /// canonical interaction proofs can bind a submitted user turn to the exact request that owned it.
    pub fn last_started_generation_for_test(&self) -> u64 {
        self.next_send_generation
    }

    pub fn inject_delivery_for_test(&mut self, generation: u64, result: Result<(), ChatSendError>) {
        self.deliveries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push_back(RuntimeChatDelivery { generation, result });
    }

    fn cancel_active_send(&mut self) -> Option<u64> {
        let cancelled_generation = self.active_send.take().map(|active| {
            active.task.abort();
            active.generation
        });
        self.deliveries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        cancelled_generation
    }

    fn drain_deliveries(&mut self) {
        loop {
            let delivered = self
                .deliveries
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .pop_front();
            let Some(delivered) = delivered else {
                return;
            };
            let Some(active_generation) = self.active_send.as_ref().map(|active| active.generation)
            else {
                continue;
            };
            if delivered.generation != active_generation {
                continue;
            }
            self.active_send.take();
            self.terminal_send_generation = Some(active_generation);
            match delivered.result {
                Ok(()) => {
                    self.last_error = None;
                    self.terminal_send_outcome = Some("completed".to_owned());
                }
                Err(error) => {
                    self.terminal_send_outcome = Some(
                        match &error {
                            ChatSendError::EndpointMissing { .. } => "endpoint_missing",
                            ChatSendError::EmptyMessage => "input_required",
                            ChatSendError::AlreadyInFlight { .. } => "already_in_flight",
                            ChatSendError::Transport { .. } => "transport_error",
                            ChatSendError::HttpStatus { .. } => "backend_rejected",
                            ChatSendError::ResponseContractMissing { .. } => "contract_missing",
                        }
                        .to_owned(),
                    );
                    self.last_error = Some(error);
                }
            }
        }
    }

    fn send_current_message(
        &mut self,
        repaint: Option<egui::Context>,
    ) -> Result<(), ChatSendError> {
        if let Some(active) = &self.active_send {
            return Err(ChatSendError::AlreadyInFlight {
                generation: active.generation,
            });
        }
        let message = self.draft.trim();
        if message.is_empty() {
            let err = ChatSendError::EmptyMessage;
            self.last_error = Some(err.clone());
            return Err(err);
        }
        self.next_send_generation = self.next_send_generation.wrapping_add(1).max(1);
        let generation = self.next_send_generation;
        self.last_cancelled_generation = None;
        self.terminal_send_generation = None;
        self.terminal_send_outcome = None;
        match self
            .client
            .send(message, generation, Arc::clone(&self.deliveries), repaint)
        {
            Ok(task) => {
                self.turns.push(ChatTurn {
                    role: ChatRole::User,
                    body: message.to_owned(),
                });
                self.draft.clear();
                self.last_error = None;
                self.active_send = Some(ActiveRuntimeChatSend { generation, task });
                Ok(())
            }
            Err(err) => {
                self.last_error = Some(err.clone());
                Err(err)
            }
        }
    }

    fn endpoint_status_text(&self) -> String {
        if self.active_send.is_some() {
            return format!(
                "Probing POST {} through the local handshake_core transport...",
                self.client.probed_path()
            );
        }
        if let Some(generation) = self.last_cancelled_generation {
            return format!(
                "Cancelled: Runtime Chat send generation {generation} was cancelled. No assistant turn was appended."
            );
        }
        match &self.last_error {
            Some(ChatSendError::EndpointMissing { .. }) | None => format!(
                "EndpointMissing: {}; {ENDPOINT_MISSING_SUMMARY}",
                self.client.probed_path()
            ),
            Some(error) => format!("{error}. No assistant reply was generated."),
        }
    }

    fn endpoint_state_label(&self) -> &'static str {
        if self.active_send.is_some() {
            return "Probing";
        }
        if self.last_cancelled_generation.is_some() {
            return "Cancelled";
        }
        match self.last_error.as_ref() {
            None | Some(ChatSendError::EndpointMissing { .. }) => "EndpointMissing",
            Some(ChatSendError::EmptyMessage) => "InputRequired",
            Some(ChatSendError::AlreadyInFlight { .. }) => "Probing",
            Some(ChatSendError::Transport { .. }) => "TransportError",
            Some(ChatSendError::HttpStatus { .. }) => "BackendRejected",
            Some(ChatSendError::ResponseContractMissing { .. }) => "ContractMissing",
        }
    }

    fn action_state_value(&self) -> String {
        let assistant_turn_count = self
            .turns
            .iter()
            .filter(|turn| turn.role == ChatRole::Assistant)
            .count();
        serde_json::json!({
            "schema": "handshake.runtime-chat-action-state/v1",
            "pane_author_id": RUNTIME_CHAT_PANEL_AUTHOR_ID,
            "focus_owner_author_id": self.focus_owner_author_id,
            "active_request_generation": self.active_send.as_ref().map(|active| active.generation),
            "last_started_generation": self.next_send_generation,
            "terminal_request_generation": self.terminal_send_generation,
            "terminal_outcome": self.terminal_send_outcome,
            "turn_count": self.turns.len(),
            "assistant_turn_count": assistant_turn_count,
        })
        .to_string()
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.drain_deliveries();
        let palette = self.palette.clone();
        let endpoint_status = self.endpoint_status_text();
        let endpoint_state = self.endpoint_state_label();
        let region = egui::Frame::new()
            .fill(palette.surface)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_min_height(ui.available_height());
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(egui::RichText::new("Runtime Chat").color(palette.text));
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(endpoint_state)
                                .color(palette.error_text)
                                .background_color(palette.error_bg),
                        );
                    });
                    let status =
                        ui.label(egui::RichText::new(&endpoint_status).color(palette.text_subtle));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let action_width = 64.0;
                        let action_area = if self.active_send.is_some() {
                            action_width * 2.0 + 16.0
                        } else {
                            action_width + 8.0
                        };
                        let input_width = (ui.available_width() - action_area).max(120.0);
                        let draft_ready = !self.draft.trim().is_empty();
                        let input = egui::Frame::new()
                            .fill(palette.bg)
                            .stroke(egui::Stroke::new(1.0, palette.border))
                            .inner_margin(egui::Margin::symmetric(6, 3))
                            .show(ui, |ui| {
                                ui.add_sized(
                                    [input_width, 20.0],
                                    egui::TextEdit::singleline(&mut self.draft)
                                        .id_salt(RUNTIME_CHAT_INPUT_AUTHOR_ID)
                                        .hint_text("Message")
                                        .text_color(palette.text)
                                        .frame(false),
                                )
                            })
                            .inner;
                        ui.ctx().accesskit_node_builder(input.id, |node| {
                            node.set_role(accesskit::Role::TextInput);
                            node.set_author_id(RUNTIME_CHAT_INPUT_AUTHOR_ID.to_owned());
                            node.set_label("Runtime Chat message".to_owned());
                            node.set_value(self.draft.clone());
                            node.add_action(accesskit::Action::Focus);
                            node.add_action(accesskit::Action::SetValue);
                        });
                        let mut native_value = None;
                        let mut native_focus = false;
                        ui.input(|state| {
                            native_focus = state
                                .accesskit_action_requests(input.id, accesskit::Action::Click)
                                .next()
                                .is_some()
                                || state
                                    .accesskit_action_requests(input.id, accesskit::Action::Focus)
                                    .next()
                                    .is_some();
                            for request in state
                                .accesskit_action_requests(input.id, accesskit::Action::SetValue)
                            {
                                if let Some(accesskit::ActionData::Value(value)) = &request.data {
                                    native_value = Some(value.to_string());
                                }
                            }
                        });
                        if native_focus {
                            input.request_focus();
                            self.focus_owner_author_id =
                                Some(RUNTIME_CHAT_INPUT_AUTHOR_ID.to_owned());
                            ui.ctx().request_repaint();
                        }
                        if input.has_focus() {
                            self.focus_owner_author_id =
                                Some(RUNTIME_CHAT_INPUT_AUTHOR_ID.to_owned());
                        }
                        if let Some(value) = native_value {
                            self.draft = value;
                            ui.ctx().request_repaint();
                        }
                        let send = ui.add_enabled(
                            draft_ready && self.active_send.is_none(),
                            egui::Button::new(egui::RichText::new("Send").color(palette.text))
                                .fill(if draft_ready && self.active_send.is_none() {
                                    palette.accent_soft
                                } else {
                                    palette.surface
                                })
                                .min_size(egui::vec2(action_width, 28.0)),
                        );
                        ui.ctx().accesskit_node_builder(send.id, |node| {
                            node.set_author_id(RUNTIME_CHAT_SEND_AUTHOR_ID.to_owned());
                            node.set_label("Send Runtime Chat message".to_owned());
                        });
                        if send.clicked() && draft_ready && self.active_send.is_none() {
                            self.focus_owner_author_id =
                                Some(RUNTIME_CHAT_SEND_AUTHOR_ID.to_owned());
                            let _ = self.send_current_message(Some(ui.ctx().clone()));
                        }
                        if self.active_send.is_some() {
                            let cancel = ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Cancel").color(palette.text),
                                )
                                .fill(palette.surface)
                                .min_size(egui::vec2(action_width, 28.0)),
                            );
                            ui.ctx().accesskit_node_builder(cancel.id, |node| {
                                node.set_author_id(RUNTIME_CHAT_CANCEL_AUTHOR_ID.to_owned());
                                node.set_label("Cancel active Runtime Chat request".to_owned());
                            });
                            if cancel.clicked() {
                                self.focus_owner_author_id =
                                    Some(RUNTIME_CHAT_CANCEL_AUTHOR_ID.to_owned());
                                self.last_cancelled_generation = self.cancel_active_send();
                                self.terminal_send_generation = self.last_cancelled_generation;
                                self.terminal_send_outcome = self
                                    .last_cancelled_generation
                                    .map(|_| "cancelled".to_owned());
                                self.last_error = None;
                                ui.ctx().request_repaint();
                            }
                        }
                    });
                    let action_state = self.action_state_value();
                    ui.ctx().accesskit_node_builder(status.id, |node| {
                        node.set_role(accesskit::Role::Status);
                        node.set_author_id(RUNTIME_CHAT_STATUS_AUTHOR_ID.to_owned());
                        node.set_label(endpoint_status.clone());
                        node.set_value(action_state);
                    });
                    ui.add_space(8.0);

                    for (index, turn) in self.turns.iter().enumerate() {
                        let label = match turn.role {
                            ChatRole::User => "You",
                            ChatRole::Assistant => "Assistant",
                            ChatRole::System => "System",
                        };
                        ui.horizontal_wrapped(|ui| {
                            let role = ui.label(
                                egui::RichText::new(format!("{label}:"))
                                    .strong()
                                    .color(palette.text),
                            );
                            ui.ctx().accesskit_node_builder(role.id, |node| {
                                node.set_author_id(runtime_chat_turn_role_author_id(index));
                                node.set_label(format!("{label}:"));
                            });
                            let body =
                                ui.label(egui::RichText::new(&turn.body).color(palette.text));
                            ui.ctx().accesskit_node_builder(body.id, |node| {
                                node.set_author_id(runtime_chat_turn_body_author_id(index));
                                node.set_label(turn.body.clone());
                            });
                        });
                    }
                    if let Some(err) = &self.last_error {
                        ui.add_space(6.0);
                        let err_text = match err.probed_path() {
                            Some(path) => {
                                format!("{err}; probed {path}. No assistant turn was appended.")
                            }
                            None => err.to_string(),
                        };
                        ui.label(egui::RichText::new(err_text).color(palette.error_text));
                    }
                });
            });

        ui.ctx().accesskit_node_builder(region.response.id, |node| {
            node.set_role(accesskit::Role::Region);
            node.set_author_id(RUNTIME_CHAT_PANEL_AUTHOR_ID.to_owned());
            node.set_label("Runtime Chat".to_owned());
        });
    }
}

impl Drop for RuntimeChatPanel {
    fn drop(&mut self) {
        self.cancel_active_send();
    }
}

/// Pane factory that renders the shared Runtime Chat panel state.
pub struct ChatPaneFactory {
    panel: Arc<Mutex<RuntimeChatPanel>>,
}

impl ChatPaneFactory {
    pub fn new(panel: Arc<Mutex<RuntimeChatPanel>>) -> Self {
        Self { panel }
    }
}

impl PaneFactory for ChatPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::RuntimeChat
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext<'_>) {
        match self.panel.lock() {
            Ok(mut panel) => panel.show(ui),
            Err(_) => {
                ui.label("Runtime Chat unavailable: panel state lock poisoned");
            }
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Region
    }
}

use std::fmt;
use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::{HsPalette, HsTheme};

/// Stable AccessKit author_id for the Runtime Chat pane container.
pub const RUNTIME_CHAT_PANEL_AUTHOR_ID: &str = "runtime-chat-panel";
/// Stable AccessKit author_id for the Runtime Chat text input.
pub const RUNTIME_CHAT_INPUT_AUTHOR_ID: &str = "runtime-chat-input";
/// Stable AccessKit author_id for the Runtime Chat endpoint/status node.
pub const RUNTIME_CHAT_STATUS_AUTHOR_ID: &str = "runtime-chat-status";
/// Stable AccessKit author_id for the Runtime Chat send button.
pub const RUNTIME_CHAT_SEND_AUTHOR_ID: &str = "runtime-chat-send";

const PRODUCTION_CHAT_ENDPOINT: &str = "/api/runtime_chat/messages";
const ENDPOINT_MISSING_SUMMARY: &str =
    "Runtime Chat endpoint missing. No assistant reply was generated.";

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
    /// The frontend inspected the native backend surface and there is no HTTP chat route to call.
    EndpointMissing { probed_path: String },
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

    pub fn probed_path(&self) -> Option<&str> {
        match self {
            Self::EmptyMessage => None,
            Self::EndpointMissing { probed_path } => Some(probed_path),
        }
    }
}

impl fmt::Display for ChatSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMessage => write!(f, "EmptyMessage: Runtime Chat draft is empty"),
            Self::EndpointMissing { probed_path } => {
                write!(f, "EndpointMissing: {probed_path}")
            }
        }
    }
}

/// Production client for Runtime Chat.
///
/// It intentionally does not target `/api/flight_recorder/runtime_chat_event`: that route records
/// observability events and is not an assistant chat send/receive route.
#[derive(Debug, Clone)]
pub struct RuntimeChatClient {
    probed_path: String,
}

impl RuntimeChatClient {
    pub fn production() -> Self {
        Self {
            probed_path: PRODUCTION_CHAT_ENDPOINT.to_owned(),
        }
    }

    pub fn probed_path(&self) -> &str {
        &self.probed_path
    }

    pub fn send(&self, message: &str) -> Result<(), ChatSendError> {
        if message.trim().is_empty() {
            return Err(ChatSendError::EmptyMessage);
        }
        Err(ChatSendError::EndpointMissing {
            probed_path: self.probed_path.clone(),
        })
    }
}

/// The live Runtime Chat pane state.
#[derive(Debug, Clone)]
pub struct RuntimeChatPanel {
    client: RuntimeChatClient,
    palette: HsPalette,
    draft: String,
    turns: Vec<ChatTurn>,
    last_error: Option<ChatSendError>,
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
        }
    }

    pub fn production(palette: HsPalette) -> Self {
        Self::new(RuntimeChatClient::production(), palette)
    }

    pub fn set_palette(&mut self, palette: HsPalette) {
        self.palette = palette;
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
        self.send_current_message()
    }

    fn send_current_message(&mut self) -> Result<(), ChatSendError> {
        let message = self.draft.trim();
        if message.is_empty() {
            let err = ChatSendError::EmptyMessage;
            self.last_error = Some(err.clone());
            return Err(err);
        }
        match self.client.send(message) {
            Ok(()) => {
                self.turns.push(ChatTurn {
                    role: ChatRole::User,
                    body: message.to_owned(),
                });
                self.draft.clear();
                self.last_error = None;
                Ok(())
            }
            Err(err) => {
                self.last_error = Some(err.clone());
                Err(err)
            }
        }
    }

    fn endpoint_status_text(&self) -> String {
        format!(
            "EndpointMissing: {}; {ENDPOINT_MISSING_SUMMARY}",
            self.client.probed_path()
        )
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette.clone();
        let endpoint_status = self.endpoint_status_text();
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
                            egui::RichText::new("EndpointMissing")
                                .color(palette.error_text)
                                .background_color(palette.error_bg),
                        );
                    });
                    let status =
                        ui.label(egui::RichText::new(&endpoint_status).color(palette.text_subtle));
                    ui.ctx().accesskit_node_builder(status.id, |node| {
                        node.set_role(accesskit::Role::Status);
                        node.set_author_id(RUNTIME_CHAT_STATUS_AUTHOR_ID.to_owned());
                        node.set_label(endpoint_status.clone());
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let send_width = 64.0;
                        let input_width = (ui.available_width() - send_width - 8.0).max(120.0);
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
                            node.set_author_id(RUNTIME_CHAT_INPUT_AUTHOR_ID.to_owned());
                            node.set_label("Runtime Chat message".to_owned());
                        });
                        let send = ui.add_enabled(
                            draft_ready,
                            egui::Button::new(egui::RichText::new("Send").color(palette.text))
                                .fill(if draft_ready {
                                    palette.accent_soft
                                } else {
                                    palette.surface
                                })
                                .min_size(egui::vec2(send_width, 28.0)),
                        );
                        ui.ctx().accesskit_node_builder(send.id, |node| {
                            node.set_author_id(RUNTIME_CHAT_SEND_AUTHOR_ID.to_owned());
                            node.set_label("Send Runtime Chat message".to_owned());
                        });
                        if send.clicked() && draft_ready {
                            let _ = self.send_current_message();
                        }
                    });
                    ui.add_space(8.0);

                    for turn in &self.turns {
                        let label = match turn.role {
                            ChatRole::User => "You",
                            ChatRole::Assistant => "Assistant",
                            ChatRole::System => "System",
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{label}:"))
                                    .strong()
                                    .color(palette.text),
                            );
                            ui.label(egui::RichText::new(&turn.body).color(palette.text));
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

impl Default for RuntimeChatPanel {
    fn default() -> Self {
        Self::production(HsTheme::Dark.palette())
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

//! WP-1 MT-012 — native Operator Chat / Launch pane.
//!
//! An interactive operator work-surface: pick a model lane (LOCAL / CLOUD / CLI),
//! pick a folder/worktree working directory, type a prompt, and launch a session
//! through the backend `POST /operator-chat/launch` route (which resolves through
//! `SwarmCoordinator::spawn_session`). The captured conversation / thought /
//! tool-calls land as ModelLaneMessage rows; this pane renders the transcript.
//!
//! HBR-QUIET: every interactive control takes only in-app egui/AccessKit focus
//! (via `accesskit_node_builder`); no OS-window foreground call is ever made.

use std::sync::{Arc, Mutex};

use egui::accesskit;
use serde::Deserialize;

use crate::backend_client::OperatorChatClient;
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};

pub const SURFACE_CONTRACT_ID: &str = "native_operator_chat_launch";
pub const SURFACE_AUTHOR_ID: &str = "operator-chat.surface";
pub const MODEL_PICKER_AUTHOR_ID: &str = "operator-chat.picker.model";
pub const FOLDER_PICKER_AUTHOR_ID: &str = "operator-chat.picker.folder";
pub const PROMPT_INPUT_AUTHOR_ID: &str = "operator-chat.input.prompt";
pub const LAUNCH_AUTHOR_ID: &str = "operator-chat.action.launch";
pub const REFRESH_MODELS_AUTHOR_ID: &str = "operator-chat.action.refresh-models";
pub const TRANSCRIPT_AUTHOR_ID: &str = "operator-chat.transcript";
pub const ERROR_AUTHOR_ID: &str = "operator-chat.error";

/// Stable author_id for one enumerated model row.
pub fn model_row_author_id(model_id: &str) -> String {
    format!("operator-chat.model.{}", token(model_id))
}

/// Stable author_id for one transcript row.
pub fn transcript_row_author_id(index: usize) -> String {
    format!("operator-chat.transcript.row.{index}")
}

/// Fold any character outside `[A-Za-z0-9_-]` to `-` so an author_id is a stable,
/// addressable token (mirrors the swarm-lane-diagnostics `token` helper).
fn token(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Frontend-local mirrors of the backend enumeration/launch JSON (the native
// crate does not depend on handshake_core).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OperatorChatModelRow {
    #[serde(default)]
    pub model_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub runtime_binding: String,
    #[serde(default)]
    pub ready: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OperatorChatCloudRow {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OperatorChatModelInventory {
    #[serde(default)]
    pub local: Vec<OperatorChatModelRow>,
    #[serde(default)]
    pub cloud_byok: Vec<OperatorChatCloudRow>,
    #[serde(default)]
    pub cloud_cli_bridge: Vec<OperatorChatCloudRow>,
    #[serde(default)]
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OperatorChatLaunched {
    #[serde(default)]
    pub instance_id: String,
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub lane_id: String,
}

/// One rendered transcript row.
#[derive(Debug, Clone)]
pub struct TranscriptRow {
    pub role: String,
    pub text: String,
}

pub type ModelsCell = Arc<Mutex<Option<Result<OperatorChatModelInventory, String>>>>;
pub type LaunchCell = Arc<Mutex<Option<Result<OperatorChatLaunched, String>>>>;

#[derive(Default)]
struct OperatorChatUiState {
    inventory: OperatorChatModelInventory,
    selected_model: String,
    folder: String,
    prompt: String,
    transcript: Vec<TranscriptRow>,
    error: Option<String>,
    pending_models: bool,
    pending_launch: bool,
}

/// The Operator Chat / Launch pane factory.
pub struct OperatorChatLaunchPaneFactory {
    state: Arc<Mutex<OperatorChatUiState>>,
    client: Option<Arc<OperatorChatClient>>,
    models_cell: ModelsCell,
    launch_cell: LaunchCell,
}

impl OperatorChatLaunchPaneFactory {
    /// Offline factory (no backend client): renders every control (for Argus
    /// drive and layout) but launch/enumerate report "backend not wired".
    pub fn offline() -> Self {
        Self {
            state: Arc::new(Mutex::new(OperatorChatUiState::default())),
            client: None,
            models_cell: Arc::new(Mutex::new(None)),
            launch_cell: Arc::new(Mutex::new(None)),
        }
    }

    /// Production factory wired to the backend operator-chat routes.
    pub fn with_client(client: Arc<OperatorChatClient>) -> Self {
        Self {
            state: Arc::new(Mutex::new(OperatorChatUiState::default())),
            client: Some(client),
            models_cell: Arc::new(Mutex::new(None)),
            launch_cell: Arc::new(Mutex::new(None)),
        }
    }

    fn drain_cells(&self) {
        if let Ok(mut slot) = self.models_cell.lock() {
            if let Some(result) = slot.take() {
                if let Ok(mut state) = self.state.lock() {
                    state.pending_models = false;
                    match result {
                        Ok(inventory) => state.inventory = inventory,
                        Err(err) => state.error = Some(err),
                    }
                }
            }
        }
        if let Ok(mut slot) = self.launch_cell.lock() {
            if let Some(result) = slot.take() {
                if let Ok(mut state) = self.state.lock() {
                    state.pending_launch = false;
                    match result {
                        Ok(launched) => state.transcript.push(TranscriptRow {
                            role: "system".to_string(),
                            text: format!(
                                "launched run {} lane {} (instance {})",
                                launched.run_id, launched.lane_id, launched.instance_id
                            ),
                        }),
                        Err(err) => state.error = Some(err),
                    }
                }
            }
        }
    }

    fn refresh_models(&self) {
        match &self.client {
            Some(client) => {
                if let Ok(mut state) = self.state.lock() {
                    state.pending_models = true;
                    state.error = None;
                }
                client.fetch_models(self.models_cell.clone());
            }
            None => {
                if let Ok(mut state) = self.state.lock() {
                    state.error = Some("backend not wired (offline pane)".to_string());
                }
            }
        }
    }

    fn launch(&self) {
        let (selected, folder, prompt) = {
            let Ok(state) = self.state.lock() else {
                return;
            };
            (
                state.selected_model.clone(),
                state.folder.clone(),
                state.prompt.clone(),
            )
        };
        if selected.trim().is_empty() || folder.trim().is_empty() || prompt.trim().is_empty() {
            if let Ok(mut state) = self.state.lock() {
                state.error =
                    Some("select a model, a folder/worktree, and enter a prompt".to_string());
            }
            return;
        }
        match &self.client {
            Some(client) => {
                if let Ok(mut state) = self.state.lock() {
                    state.pending_launch = true;
                    state.error = None;
                    state.transcript.push(TranscriptRow {
                        role: "operator".to_string(),
                        text: prompt.clone(),
                    });
                }
                client.launch(&selected, &folder, &prompt, self.launch_cell.clone());
            }
            None => {
                if let Ok(mut state) = self.state.lock() {
                    state.error = Some("backend not wired (offline pane)".to_string());
                }
            }
        }
    }
}

fn labelled_text_edit(
    ui: &mut egui::Ui,
    egui_id: egui::Id,
    author_id: &str,
    label: &str,
    buffer: &mut String,
    multiline: bool,
    width: f32,
) {
    let response = if multiline {
        ui.add(
            egui::TextEdit::multiline(buffer)
                .id_source(egui_id.with(author_id))
                .desired_width(width)
                .desired_rows(3),
        )
    } else {
        ui.add(
            egui::TextEdit::singleline(buffer)
                .id_source(egui_id.with(author_id))
                .desired_width(width),
        )
    };
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_author_id(author_id.to_owned());
        node.set_label(label.to_owned());
    });
}

fn labelled_button(ui: &mut egui::Ui, author_id: &str, label: &str) -> bool {
    let response = ui.button(label);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(accesskit::Role::Button);
        node.add_action(accesskit::Action::Click);
        node.set_author_id(author_id.to_owned());
        node.set_label(label.to_owned());
    });
    response.clicked()
}

impl PaneFactory for OperatorChatLaunchPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::OperatorChatLaunch
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        self.drain_cells();

        let surface_id = ctx.egui_id.with("operator-chat-surface");
        ui.ctx().accesskit_node_builder(surface_id, |node| {
            node.set_role(accesskit::Role::Group);
            node.set_author_id(SURFACE_AUTHOR_ID.to_owned());
            node.set_label("Operator Chat / Launch".to_owned());
        });

        let mut do_refresh = false;
        let mut do_launch = false;
        let mut select_model: Option<String> = None;

        let Ok(mut state) = self.state.lock() else {
            ui.label("Operator chat state unavailable");
            return;
        };

        ui.vertical(|ui| {
            ui.heading("Operator Chat / Launch");

            // Model picker: selected id + refresh + enumerated rows.
            ui.horizontal(|ui| {
                ui.label("Model");
                labelled_text_edit(
                    ui,
                    ctx.egui_id,
                    MODEL_PICKER_AUTHOR_ID,
                    "Selected model",
                    &mut state.selected_model,
                    false,
                    260.0,
                );
                if labelled_button(ui, REFRESH_MODELS_AUTHOR_ID, "Refresh models") {
                    do_refresh = true;
                }
            });
            if state.pending_models {
                ui.label("Enumerating models...");
            }
            for row in &state.inventory.local {
                let author = model_row_author_id(&row.model_id);
                let label = format!("LOCAL  {}  ({})", row.display_name, row.runtime_binding);
                if labelled_button(ui, &author, &label) {
                    select_model = Some(row.model_id.clone());
                }
            }
            for row in &state.inventory.cloud_byok {
                let author = model_row_author_id(&row.provider);
                let label = format!("CLOUD  {}  [{}]", row.label, row.status);
                if labelled_button(ui, &author, &label) {
                    select_model = Some(row.provider.clone());
                }
            }

            // Folder / worktree picker.
            ui.horizontal(|ui| {
                ui.label("Folder / worktree");
                labelled_text_edit(
                    ui,
                    ctx.egui_id,
                    FOLDER_PICKER_AUTHOR_ID,
                    "Working directory",
                    &mut state.folder,
                    false,
                    360.0,
                );
            });

            // Prompt input.
            ui.label("Prompt");
            labelled_text_edit(
                ui,
                ctx.egui_id,
                PROMPT_INPUT_AUTHOR_ID,
                "Operator prompt",
                &mut state.prompt,
                true,
                480.0,
            );

            // Launch.
            if labelled_button(ui, LAUNCH_AUTHOR_ID, "Launch session") {
                do_launch = true;
            }
            if state.pending_launch {
                ui.label("Launching...");
            }
            if let Some(error) = state.error.clone() {
                let resp = ui.colored_label(ui.visuals().error_fg_color, &error);
                ui.ctx().accesskit_node_builder(resp.id, |node| {
                    node.set_author_id(ERROR_AUTHOR_ID.to_owned());
                    node.set_label(error);
                });
            }

            // Transcript.
            ui.separator();
            let transcript_id = ctx.egui_id.with("operator-chat-transcript");
            ui.ctx().accesskit_node_builder(transcript_id, |node| {
                node.set_role(accesskit::Role::Group);
                node.set_author_id(TRANSCRIPT_AUTHOR_ID.to_owned());
                node.set_label("Transcript".to_owned());
            });
            ui.label("Transcript");
            if state.transcript.is_empty() {
                ui.label("No captured turns yet. Launch a session to capture the conversation, thought, and tool calls.");
            }
            for (index, row) in state.transcript.iter().enumerate() {
                let author = transcript_row_author_id(index);
                let resp = ui.label(format!("[{}] {}", row.role, row.text));
                ui.ctx().accesskit_node_builder(resp.id, |node| {
                    node.set_author_id(author);
                    node.set_label(format!("{}: {}", row.role, row.text));
                });
            }
        });

        if let Some(model) = select_model {
            state.selected_model = model;
        }
        drop(state);

        if do_refresh {
            self.refresh_models();
        }
        if do_launch {
            self.launch();
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Pane
    }
}

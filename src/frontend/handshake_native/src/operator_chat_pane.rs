//! WP-1 MT-012 — native Operator Chat / Launch pane.
//!
//! An interactive operator work-surface: pick a model lane (LOCAL / CLOUD / CLI /
//! SUBAGENT), pick a folder/worktree working directory, type a prompt, and
//! launch a session through the backend `POST /operator-chat/launch` route.
//! Process-backed lanes resolve through `SwarmCoordinator::spawn_session`; the
//! subagent lane records a no-OS Dexterity ModelLane. The captured conversation /
//! thought / tool-calls land as ModelLaneMessage rows; this pane renders the
//! transcript.
//!
//! HBR-QUIET: every interactive control takes only in-app egui/AccessKit focus
//! (via `accesskit_node_builder`); no OS-window foreground call is ever made.

use std::sync::{Arc, Mutex};

use egui::accesskit;
use serde::{Deserialize, Serialize};

use crate::backend_client::OperatorChatClient;
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};

pub const SURFACE_CONTRACT_ID: &str = "native_operator_chat_launch";
pub const SURFACE_AUTHOR_ID: &str = "operator-chat.surface";
pub const MODEL_PICKER_AUTHOR_ID: &str = "operator-chat.picker.model";
pub const FOLDER_PICKER_AUTHOR_ID: &str = "operator-chat.picker.folder";
pub const PROMPT_INPUT_AUTHOR_ID: &str = "operator-chat.input.prompt";
pub const LAUNCH_AUTHOR_ID: &str = "operator-chat.action.launch";
pub const REFRESH_MODELS_AUTHOR_ID: &str = "operator-chat.action.refresh-models";
pub const LAUNCH_STATUS_AUTHOR_ID: &str = "operator-chat.launch.status";
pub const TRANSCRIPT_AUTHOR_ID: &str = "operator-chat.transcript";
pub const ERROR_AUTHOR_ID: &str = "operator-chat.error";

/// Stable author_id for one enumerated model row.
pub fn model_row_author_id(model_id: &str) -> String {
    format!("operator-chat.model.{}", token(model_id))
}

/// Stable author_id for one enumerated model row with lane/provider identity.
pub fn model_selection_author_id(
    lane_kind: &str,
    provider: Option<&str>,
    model_id: &str,
) -> String {
    format!(
        "operator-chat.model.{}.{}.{}",
        token(lane_kind),
        token(provider.unwrap_or("none")),
        token(model_id)
    )
}

/// Stable author_id for one transcript row.
pub fn transcript_row_author_id(index: usize) -> String {
    format!("operator-chat.transcript.row.{index}")
}

/// Stable author_id for one transcript row. Backend message ids win over render
/// position so a refresh/reflow does not change the addressable row target.
pub fn transcript_row_author_id_for(index: usize, message_id: Option<&str>) -> String {
    match message_id.filter(|id| !id.trim().is_empty()) {
        Some(message_id) => format!("operator-chat.transcript.message.{}", token(message_id)),
        None => transcript_row_author_id(index),
    }
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
    pub model_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorChatLaunchSelection {
    pub lane_kind: String,
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_provider: Option<String>,
}

impl OperatorChatLaunchSelection {
    pub fn local(model_id: impl Into<String>) -> Self {
        Self {
            lane_kind: "local".to_string(),
            model_id: model_id.into(),
            cloud_provider: None,
            cli_provider: None,
        }
    }

    pub fn cloud(model_id: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            lane_kind: "cloud".to_string(),
            model_id: model_id.into(),
            cloud_provider: Some(provider.into()),
            cli_provider: None,
        }
    }

    pub fn cli(model_id: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            lane_kind: "cli".to_string(),
            model_id: model_id.into(),
            cloud_provider: None,
            cli_provider: Some(provider.into()),
        }
    }

    pub fn subagent(model_id: impl Into<String>) -> Self {
        Self {
            lane_kind: "subagent".to_string(),
            model_id: model_id.into(),
            cloud_provider: None,
            cli_provider: None,
        }
    }
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
    pub subagents: Vec<OperatorChatSubagentRow>,
    #[serde(default)]
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OperatorChatSubagentRow {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub model_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub status: String,
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
    pub message_id: Option<String>,
    pub ordered_index: Option<u64>,
}

pub type ModelsCell = Arc<Mutex<Option<Result<OperatorChatModelInventory, String>>>>;
pub type LaunchCell = Arc<Mutex<Option<Result<OperatorChatLaunched, String>>>>;
/// One-slot cell the async transcript fetch delivers captured rows into (F8).
pub type TranscriptCell = Arc<Mutex<Option<Result<Vec<TranscriptRow>, String>>>>;

/// The backend seam the Operator Chat pane drives. `OperatorChatClient` is the
/// production HTTP implementation; tests inject a recording fake to prove the
/// wiring (F6 selection audit, F8 transcript render) without a live backend.
pub trait OperatorChatBackend: Send + Sync {
    /// Enumerate the picker inventory into `cell` (`GET /operator-chat/models`).
    fn fetch_models(&self, cell: ModelsCell);
    /// Record an operator model selection as an auditable decision (F6,
    /// `POST /operator-chat/selection` -> FR-EVT-MODEL-SELECTION-RECORDED).
    fn record_selection(&self, selection: &OperatorChatLaunchSelection, working_dir: Option<&str>);
    /// Launch a CLI session for the operator selection into `cell`.
    fn launch(
        &self,
        selection: OperatorChatLaunchSelection,
        working_dir: &str,
        prompt: &str,
        cell: LaunchCell,
    );
    /// Fetch the captured transcript rows for a launched run into `cell` (F8,
    /// `GET /operator-chat/transcript/:run_id`).
    fn fetch_transcript(&self, run_id: &str, cell: TranscriptCell);
}

#[derive(Default)]
struct OperatorChatUiState {
    inventory: OperatorChatModelInventory,
    selected_model: String,
    selected: Option<OperatorChatLaunchSelection>,
    folder: String,
    prompt: String,
    launch_status: Option<String>,
    transcript: Vec<TranscriptRow>,
    error: Option<String>,
    pending_models: bool,
    pending_launch: bool,
    pending_transcript: bool,
}

/// The Operator Chat / Launch pane factory.
pub struct OperatorChatLaunchPaneFactory {
    state: Arc<Mutex<OperatorChatUiState>>,
    client: Option<Arc<dyn OperatorChatBackend>>,
    models_cell: ModelsCell,
    launch_cell: LaunchCell,
    transcript_cell: TranscriptCell,
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
            transcript_cell: Arc::new(Mutex::new(None)),
        }
    }

    /// Production factory wired to the backend operator-chat routes.
    pub fn with_client(client: Arc<OperatorChatClient>) -> Self {
        Self::with_backend(client)
    }

    /// Factory wired to any [`OperatorChatBackend`] (production HTTP client, or a
    /// recording fake in tests).
    pub fn with_backend(client: Arc<dyn OperatorChatBackend>) -> Self {
        Self {
            state: Arc::new(Mutex::new(OperatorChatUiState::default())),
            client: Some(client),
            models_cell: Arc::new(Mutex::new(None)),
            launch_cell: Arc::new(Mutex::new(None)),
            transcript_cell: Arc::new(Mutex::new(None)),
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
        // A successful launch triggers a transcript fetch so the pane renders the
        // captured ModelLaneMessage rows (F8), not just a local echo.
        let mut fetch_run: Option<String> = None;
        if let Ok(mut slot) = self.launch_cell.lock() {
            if let Some(result) = slot.take() {
                if let Ok(mut state) = self.state.lock() {
                    state.pending_launch = false;
                    match result {
                        Ok(launched) => {
                            state.launch_status = Some(format!(
                                "launched run {} lane {} (instance {})",
                                launched.run_id, launched.lane_id, launched.instance_id
                            ));
                            if !launched.run_id.trim().is_empty() {
                                fetch_run = Some(launched.run_id.clone());
                            }
                        }
                        Err(err) => state.error = Some(err),
                    }
                }
            }
        }
        if let Some(run_id) = fetch_run {
            if let Some(client) = &self.client {
                if let Ok(mut state) = self.state.lock() {
                    state.pending_transcript = true;
                }
                client.fetch_transcript(&run_id, self.transcript_cell.clone());
            }
        }
        // Drain fetched transcript rows: append each captured turn so it renders.
        if let Ok(mut slot) = self.transcript_cell.lock() {
            if let Some(result) = slot.take() {
                if let Ok(mut state) = self.state.lock() {
                    state.pending_transcript = false;
                    match result {
                        Ok(rows) => {
                            for row in rows {
                                state.transcript.push(row);
                            }
                        }
                        Err(err) => state.error = Some(err),
                    }
                }
            }
        }
    }

    /// Record the operator's model selection as an auditable decision (F6). Wires
    /// the previously-dead `POST /operator-chat/selection` path.
    fn record_selection(&self, selection: &OperatorChatLaunchSelection, working_dir: Option<&str>) {
        if let Some(client) = &self.client {
            client.record_selection(selection, working_dir);
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
                state.selected.clone(),
                state.folder.clone(),
                state.prompt.clone(),
            )
        };
        if selected.is_none() || folder.trim().is_empty() || prompt.trim().is_empty() {
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
                    state.launch_status = None;
                }
                client.launch(
                    selected.expect("checked selected"),
                    &folder,
                    &prompt,
                    self.launch_cell.clone(),
                );
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

fn labelled_disabled_button(ui: &mut egui::Ui, author_id: &str, label: &str) {
    let response = ui.add_enabled(false, egui::Button::new(label));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, false, label));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author_id.to_owned());
        node.set_label(label.to_owned());
    });
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
        let mut select_model: Option<(OperatorChatLaunchSelection, String)> = None;
        let mut audit_selection: Option<(OperatorChatLaunchSelection, Option<String>)> = None;

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
                let author = model_selection_author_id("local", None, &row.model_id);
                let label = format!("LOCAL  {}  ({})", row.display_name, row.runtime_binding);
                if labelled_button(ui, &author, &label) {
                    select_model = Some((
                        OperatorChatLaunchSelection::local(row.model_id.clone()),
                        row.model_id.clone(),
                    ));
                }
            }
            for row in &state.inventory.cloud_byok {
                let author =
                    model_selection_author_id("cloud", Some(&row.provider), &row.model_id);
                let label = format!("CLOUD  {}  [{}]", row.label, row.status);
                if row.status == "configured" && labelled_button(ui, &author, &label) {
                    select_model = Some((
                        OperatorChatLaunchSelection::cloud(
                            row.model_id.clone(),
                            row.provider.clone(),
                        ),
                        format!("{} / {}", row.provider, row.model_id),
                    ));
                } else if row.status != "configured" {
                    labelled_disabled_button(ui, &author, &label);
                }
            }
            for row in &state.inventory.cloud_cli_bridge {
                let author = model_selection_author_id("cli", Some(&row.provider), &row.model_id);
                let label = format!("CLI  {}  [{}]", row.label, row.status);
                if row.status == "configured" && labelled_button(ui, &author, &label) {
                    select_model = Some((
                        OperatorChatLaunchSelection::cli(
                            row.model_id.clone(),
                            row.provider.clone(),
                        ),
                        format!("{} / {}", row.provider, row.model_id),
                    ));
                } else if row.status != "configured" {
                    labelled_disabled_button(ui, &author, &label);
                }
            }
            for row in &state.inventory.subagents {
                let author = model_selection_author_id("subagent", Some(&row.role), &row.model_id);
                let label = format!("SUBAGENT  {}  [{}]", row.label, row.status);
                if row.status == "available" && labelled_button(ui, &author, &label) {
                    select_model = Some((
                        OperatorChatLaunchSelection::subagent(row.model_id.clone()),
                        format!("{} / {}", row.role, row.model_id),
                    ));
                } else if row.status != "available" {
                    labelled_disabled_button(ui, &author, &label);
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
            if let Some(status) = state.launch_status.clone() {
                let resp = ui.label(status.clone());
                ui.ctx().accesskit_node_builder(resp.id, |node| {
                    node.set_author_id(LAUNCH_STATUS_AUTHOR_ID.to_owned());
                    node.set_label(status);
                });
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
                let author = transcript_row_author_id_for(index, row.message_id.as_deref());
                let resp = ui.label(format!("[{}] {}", row.role, row.text));
                ui.ctx().accesskit_node_builder(resp.id, |node| {
                    node.set_author_id(author);
                    node.set_label(format!("{}: {}", row.role, row.text));
                });
            }
        });

        if let Some((selection, display)) = select_model {
            let working_dir = (!state.folder.trim().is_empty()).then(|| state.folder.clone());
            audit_selection = Some((selection.clone(), working_dir));
            state.selected_model = display;
            state.selected = Some(selection);
            // Fire the selection-decision audit for a real operator model pick
            // (F6) — distinct from launch, recorded off the state lock below.
        }
        drop(state);

        if let Some((selection, working_dir)) = audit_selection {
            self.record_selection(&selection, working_dir.as_deref());
        }
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

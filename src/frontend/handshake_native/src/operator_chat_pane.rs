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

use crate::backend_client::{OperatorChatClient, OperatorChatRoutingFailure, EngineProfileSnapshotV1, EngineProfileIdentityV1};
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};

pub const SURFACE_CONTRACT_ID: &str = "native_operator_chat_launch";
pub const SURFACE_AUTHOR_ID: &str = "operator-chat.surface";
pub const MODEL_PICKER_AUTHOR_ID: &str = "operator-chat.picker.model";
/// Focus-safe production worktree selection. Argus can set this text field over
/// the authenticated socket; no native folder picker or foreground activation
/// is required.
pub const WORKTREE_SELECTION_AUTHOR_ID: &str = "operator-chat.picker.folder";
/// Compatibility alias for existing in-process proofs and external clients.
pub const FOLDER_PICKER_AUTHOR_ID: &str = WORKTREE_SELECTION_AUTHOR_ID;
pub const PROMPT_INPUT_AUTHOR_ID: &str = "operator-chat.input.prompt";
pub const LAUNCH_AUTHOR_ID: &str = "operator-chat.action.launch";
pub const REFRESH_MODELS_AUTHOR_ID: &str = "operator-chat.action.refresh-models";
pub const LAUNCH_STATUS_AUTHOR_ID: &str = "operator-chat.launch.status";
pub const TRANSCRIPT_AUTHOR_ID: &str = "operator-chat.transcript";
pub const ERROR_AUTHOR_ID: &str = "operator-chat.error";
pub const ROUTING_REQUEST_AUTHOR_ID: &str = "operator-chat.routing.request";
pub const ROUTING_LIFECYCLE_AUTHOR_ID: &str = "operator-chat.routing.lifecycle";
pub const ROUTING_RECOVER_AUTHOR_ID: &str = "operator-chat.routing.recover";
pub const ROUTING_CANCEL_AUTHOR_ID: &str = "operator-chat.routing.cancel";
pub const ROUTING_AUTHORITY_AUTHOR_ID: &str = "operator-chat.routing.authority";
pub const ROUTING_STATUS_AUTHOR_ID: &str = "operator-chat.routing.status";

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

pub fn profile_selection_author_id(profile: &EngineProfileSnapshotV1) -> String {
    format!("operator-chat.profile.{}", profile.fingerprint)
}

pub fn session_selection_author_id(session_id: &str) -> String {
    format!("operator-chat.session.{}", token(session_id))
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
    #[serde(default)]
    pub engine_profile: Option<EngineProfileSnapshotV1>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OperatorChatSessionRow {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub parent_session_id: Option<String>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub status: String,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorChatLaunchSelection {
    pub lane_kind: String,
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_profile: Option<EngineProfileSnapshotV1>,
}

impl OperatorChatLaunchSelection {
    pub fn local(profile: EngineProfileSnapshotV1) -> Self {
        Self {
            lane_kind: "local".to_string(),
            model_id: profile.profile.model.id.clone(),
            cloud_provider: None,
            cli_provider: None,
            engine_profile: Some(profile),
        }
    }

    pub fn cloud(model_id: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            lane_kind: "cloud".to_string(),
            model_id: model_id.into(),
            cloud_provider: Some(provider.into()),
            cli_provider: None,
            engine_profile: None,
        }
    }

    pub fn cli(model_id: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            lane_kind: "cli".to_string(),
            model_id: model_id.into(),
            cloud_provider: None,
            cli_provider: Some(provider.into()),
            engine_profile: None,
        }
    }

    pub fn subagent(model_id: impl Into<String>) -> Self {
        Self {
            lane_kind: "subagent".to_string(),
            model_id: model_id.into(),
            cloud_provider: None,
            cli_provider: None,
            engine_profile: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OperatorChatModelInventory {
    #[serde(default)]
    pub active_engine_profile: Option<EngineProfileIdentityV1>,
    #[serde(default)]
    pub inventory_source: String,
    #[serde(default)]
    pub sessions: Vec<OperatorChatSessionRow>,
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
    pub engine_profile: Option<EngineProfileSnapshotV1>,
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
pub type SelectionCell = Arc<Mutex<Option<Result<(), String>>>>;
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
    fn record_selection(
        &self,
        selection: OperatorChatLaunchSelection,
        working_dir: Option<String>,
        cell: SelectionCell,
    );
    /// Launch a CLI session for the operator selection into `cell`.
    fn launch(
        &self,
        selection: OperatorChatLaunchSelection,
        owner_session_id: &str,
        working_dir: &str,
        prompt: &str,
        cell: LaunchCell,
    );
    /// Fetch the captured transcript rows for a launched run into `cell` (F8,
    /// `GET /operator-chat/transcript/:run_id`).
    fn fetch_transcript(&self, run_id: &str, cell: TranscriptCell);
    fn record_diagnostic_observation(&self, _run_id: &str, _lane_id: &str) {}
    /// Production checks the captured authenticated context; component backends own their scope.
    fn launch_recovery_scope_is_active(&self) -> bool { true }
    fn routing_action(
        &self,
        action: OperatorChatRoutingAction,
        request_json: String,
        cell: RoutingCell,
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperatorChatRoutingAction {
    Lifecycle,
    Recover,
    Cancel,
    Authority,
}

impl OperatorChatRoutingAction {
    fn label(self) -> &'static str {
        match self {
            Self::Lifecycle => "Execute lifecycle",
            Self::Recover => "Recover lifecycle",
            Self::Cancel => "Cancel execution",
            Self::Authority => "Complete authority",
        }
    }
}

pub type RoutingCell = Arc<Mutex<Option<Result<serde_json::Value, OperatorChatRoutingFailure>>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RoutingExecutionStatus {
    Running, AwaitingAuthority, Succeeded, Failed, Cancelled,
}

impl RoutingExecutionStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::AwaitingAuthority => "awaiting authority",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

// Project only mechanical state and canonical references; never retain graph, output or reasons.
#[derive(Clone, PartialEq, Eq, Deserialize)]
struct RoutingExecutionProjection {
    schema_id: String,
    execution_id: String,
    run_id: String,
    status: RoutingExecutionStatus,
    revision: u64,
    event_ledger_event_id: String,
    event_ledger_seq: i64,
}

#[derive(Deserialize)]
struct RoutingBatchProjection {
    execution: RoutingExecutionProjection,
}

struct PendingRouting {
    action: OperatorChatRoutingAction,
    workspace_id: String,
    request_json: String,
    execution_id: String,
    run_id: Option<String>,
    cell: RoutingCell,
}

impl PendingRouting {
    fn from_request(action: OperatorChatRoutingAction, workspace_id: String, request_json: String) -> Option<Self> {
        let request: serde_json::Value = serde_json::from_str(&request_json).ok()?;
        let execution_id = request.get("execution_id")?.as_str()?.to_owned();
        if execution_id.trim().is_empty() { return None; }
        let run_id = match action {
            OperatorChatRoutingAction::Cancel => None,
            OperatorChatRoutingAction::Authority => {
                let routing = request.get("routing_request")?;
                if routing.get("execution_id")?.as_str()? != execution_id { return None; }
                Some(routing.get("context")?.get("run_id")?.as_str()?.to_owned())
            }
            _ => Some(request.get("context")?.get("run_id")?.as_str()?.to_owned()),
        };
        if run_id.as_ref().is_some_and(|id| id.trim().is_empty()) { return None; }
        Some(Self { action, workspace_id, request_json, execution_id, run_id,
            cell: Arc::new(Mutex::new(None)) })
    }

    fn outcome(&self, value: serde_json::Value) -> Option<RoutingExecutionProjection> {
        let execution = match self.action {
            OperatorChatRoutingAction::Cancel => serde_json::from_value(value).ok()?,
            _ => serde_json::from_value::<RoutingBatchProjection>(value).ok()?.execution,
        };
        if execution.schema_id != "hsk.model_lane_routing_execution@5"
            || execution.execution_id != self.execution_id
            || execution.run_id.trim().is_empty()
            || self.run_id.as_ref().is_some_and(|run| run != &execution.run_id)
            || execution.event_ledger_event_id.trim().is_empty()
        { return None; }
        Some(execution)
    }
}

struct PendingLaunch {
    selection: OperatorChatLaunchSelection,
    display: String,
    workspace_id: String,
    owner_session_id: String,
    working_dir: String,
    prompt: String,
    cell: LaunchCell,
}

struct PendingSelection {
    selection: OperatorChatLaunchSelection,
    display: String,
    workspace_id: String,
    owner_session_id: Option<String>,
    working_dir: String,
    prompt: String,
    cell: SelectionCell,
}

#[derive(Default)]
struct OperatorChatUiState {
    workspace_id: String,
    models_cell: ModelsCell,
    transcript_cell: TranscriptCell,
    inventory: OperatorChatModelInventory,
    selected_model: String,
    selected: Option<OperatorChatLaunchSelection>,
    selected_owner_session_id: Option<String>,
    folder: String,
    prompt: String,
    launch_status: Option<String>,
    transcript: Vec<TranscriptRow>,
    error: Option<String>,
    pending_models: bool,
    pending_selection: Option<PendingSelection>,
    pending_launch: bool,
    pending_launch_request: Option<PendingLaunch>,
    local_candidate: Option<(OperatorChatLaunchSelection, String)>,
    pending_transcript: bool,
    routing_request_json: String,
    routing_results: std::collections::BTreeMap<OperatorChatRoutingAction, RoutingExecutionProjection>,
    routing_errors: std::collections::BTreeMap<OperatorChatRoutingAction, String>,
    routing_observed: std::collections::BTreeMap<(String, String), RoutingExecutionProjection>,
    pending_routing: Vec<PendingRouting>,
}

impl OperatorChatUiState {
    fn prune_routing_observations(&mut self) {
        let pending = &self.pending_routing;
        let visible = &self.routing_results;
        self.routing_observed.retain(|(execution_id, run_id), _| {
            pending.iter().any(|request| &request.execution_id == execution_id
                && request.run_id.as_ref().is_none_or(|run| run == run_id))
                || visible.values().any(|result| &result.execution_id == execution_id && &result.run_id == run_id)
        });
    }

    fn owner_session_is_canonical_and_available(&self) -> bool {
        self.selected_owner_session_id
            .as_deref()
            .is_some_and(|selected| {
                self.inventory
                    .sessions
                    .iter()
                    .any(|row| row.session_id == selected && row.status == "available")
            })
    }

    fn selected_model_is_canonical_and_available(&self) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|selected| self.selection_is_canonical_and_available(selected))
    }

    fn selection_is_canonical_and_available(&self, selected: &OperatorChatLaunchSelection) -> bool {
        match selected.lane_kind.as_str() {
            "local" => selected.engine_profile.as_ref().is_some_and(|profile| {
                self.inventory.local.iter().any(|row| {
                    row.ready && row.model_id == selected.model_id
                        && row.model_id == profile.profile.model.id
                        && row.engine_profile.as_ref() == Some(profile)
                })
            }),
            "cloud" => self.inventory.cloud_byok.iter().any(|row| {
                row.model_id == selected.model_id
                    && selected.cloud_provider.as_deref() == Some(row.provider.as_str())
                    && row.status == "configured"
            }),
            "cli" => self.inventory.cloud_cli_bridge.iter().any(|row| {
                row.model_id == selected.model_id
                    && selected.cli_provider.as_deref() == Some(row.provider.as_str())
                    && row.status == "logged_in"
            }),
            "subagent" => self
                .inventory
                .subagents
                .iter()
                .any(|row| row.model_id == selected.model_id && row.status == "available"),
            _ => false,
        }
    }

    fn reconcile_with_refreshed_inventory(&mut self, inventory: OperatorChatModelInventory) {
        self.inventory = inventory;
        let mut invalidated = Vec::new();
        if self.selected_owner_session_id.is_some()
            && !self.owner_session_is_canonical_and_available()
        {
            self.selected_owner_session_id = None;
            invalidated.push("selected governed session is no longer available");
        }
        if self.selected.is_some() && !self.selected_model_is_canonical_and_available() {
            self.selected = None;
            self.selected_model.clear();
            invalidated.push("selected model is no longer ready or available");
        }
        if self
            .pending_selection
            .as_ref()
            .is_some_and(|pending| !self.selection_is_canonical_and_available(&pending.selection))
        {
            self.pending_selection = None;
            invalidated.push("pending model selection is no longer ready or available");
        }
        if self.local_candidate.as_ref().is_some_and(|(selection, _)|
            !self.selection_is_canonical_and_available(selection))
        {
            self.local_candidate = None;
            self.selected_model.clear();
            invalidated.push("candidate profile is no longer available");
        }
        if !invalidated.is_empty() {
            self.error = Some(format!(
                "Inventory refresh cleared selection: {}. Select an available row before launch.",
                invalidated.join("; ")
            ));
        }
    }

    fn fail_inventory_refresh(&mut self) {
        self.inventory = OperatorChatModelInventory::default();
        self.selected = None;
        self.selected_model.clear();
        self.selected_owner_session_id = None;
        self.pending_selection = None;
        self.local_candidate = None;
        self.error = Some(
            "Model inventory refresh failed; all model and session selections were cleared."
                .to_owned(),
        );
    }
}

fn session_row_state(row: &OperatorChatSessionRow) -> (bool, &'static str) {
    if row.status == "available"
        && !row.session_id.trim().is_empty()
        && row
            .parent_session_id
            .as_deref()
            .is_some_and(|parent| !parent.trim().is_empty())
    {
        (true, "available")
    } else {
        (false, "governed owner/parent lineage unavailable")
    }
}

fn byok_row_state(row: &OperatorChatCloudRow) -> (bool, &'static str) {
    if row.status == "configured" {
        (true, "configured")
    } else {
        (false, "key not configured or keychain unavailable")
    }
}

fn cli_row_state(row: &OperatorChatCloudRow) -> (bool, &'static str) {
    match row.status.as_str() {
        "logged_in" => (true, "logged in"),
        "logged_out" => (false, "official CLI sign-in required"),
        "expired" => (false, "official CLI sign-in expired"),
        _ => (false, "official CLI unavailable"),
    }
}

fn subagent_row_state(row: &OperatorChatSubagentRow) -> (bool, &'static str) {
    if row.status == "available" {
        (true, "available")
    } else {
        (false, "subagent manager unavailable")
    }
}

const LAUNCH_RECOVERY_CAPACITY: usize = 8;

struct LaunchRecoveryEntry {
    ui_key: u64,
    workspace_id: String,
    cell: LaunchCell,
    returned: Option<OperatorChatLaunched>,
}

/// The Operator Chat / Launch pane factory.
pub struct OperatorChatLaunchPaneFactory {
    state: Arc<Mutex<OperatorChatUiState>>,
    client: Option<Arc<dyn OperatorChatBackend>>,
    // Account lifetime is the factory lifetime. Workspace changes retain only bounded receipts.
    launch_recovery: Mutex<Vec<LaunchRecoveryEntry>>,
    launch_recovery_ui_sequence: Mutex<u64>,
}

impl OperatorChatLaunchPaneFactory {
    /// Offline factory (no backend client): renders every control (for Argus
    /// drive and layout) but launch/enumerate report "backend not wired".
    pub fn offline() -> Self {
        Self {
            state: Arc::new(Mutex::new(OperatorChatUiState::default())),
            client: None,
            launch_recovery: Mutex::new(Vec::new()),
            launch_recovery_ui_sequence: Mutex::new(0),
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
            launch_recovery: Mutex::new(Vec::new()),
            launch_recovery_ui_sequence: Mutex::new(0),
        }
    }

    fn recovery_scope_is_active(&self) -> bool {
        self.client.as_ref().is_some_and(|client| client.launch_recovery_scope_is_active())
    }

    fn forget_launch_receipt(&self, cell: &LaunchCell) {
        if let Ok(mut entries) = self.launch_recovery.lock() {
            entries.retain(|entry| !Arc::ptr_eq(&entry.cell, cell));
        }
    }

    fn retain_launch_result(&self, cell: &LaunchCell, result: &Result<OperatorChatLaunched, String>) {
        if !self.recovery_scope_is_active() {
            if let Ok(mut entries) = self.launch_recovery.lock() { entries.clear(); }
            return;
        }
        match result {
            Ok(returned) => {
                if let Ok(mut entries) = self.launch_recovery.lock() {
                    if let Some(entry) = entries.iter_mut().find(|entry| Arc::ptr_eq(&entry.cell, cell)) {
                        entry.returned = Some(returned.clone());
                    }
                }
            }
            Err(_) => self.forget_launch_receipt(cell),
        }
    }

    fn drain_detached_launches(&self, active: Option<&LaunchCell>) {
        let Ok(mut entries) = self.launch_recovery.lock() else { return; };
        if !self.recovery_scope_is_active() { entries.clear(); return; }
        entries.retain_mut(|entry| {
            if entry.returned.is_some() || active.is_some_and(|cell| Arc::ptr_eq(cell, &entry.cell)) {
                return true;
            }
            let result = entry.cell.lock().ok().and_then(|mut slot| slot.take());
            match result {
                Some(Ok(returned)) => { entry.returned = Some(returned); true }
                Some(Err(_)) => false,
                None => true,
            }
        });
    }

    fn drain_cells(&self) {
        let (models_cell, launch_cell, transcript_cell, routing_cells) = {
            let Ok(state) = self.state.lock() else { return; };
            (state.models_cell.clone(),
             state.pending_launch_request.as_ref().map(|pending| pending.cell.clone()),
             state.transcript_cell.clone(),
             state.pending_routing.iter().map(|pending| pending.cell.clone()).collect::<Vec<_>>())
        };
        self.drain_detached_launches(launch_cell.as_ref());
        if let Ok(mut slot) = models_cell.lock() {
            if let Some(result) = slot.take() {
                if let Ok(mut state) = self.state.lock() {
                    state.pending_models = false;
                    match result {
                        Ok(inventory) => state.reconcile_with_refreshed_inventory(inventory),
                        Err(_) => state.fail_inventory_refresh(),
                    }
                }
            }
        }
        let selection_cell = self.state.lock().ok().and_then(|state|
            state.pending_selection.as_ref().map(|pending| pending.cell.clone()));
        if let Some(cell) = selection_cell {
            let result = cell.lock().ok().and_then(|mut cell| cell.take());
            if let Some(result) = result {
                if let Ok(mut state) = self.state.lock() {
                    if !state.pending_selection.as_ref().is_some_and(|pending|
                        Arc::ptr_eq(&pending.cell, &cell)) { return; }
                    if let Some(pending) = state.pending_selection.take() {
                        let current = pending.workspace_id == state.workspace_id
                            && pending.owner_session_id == state.selected_owner_session_id
                            && pending.working_dir == state.folder
                            && pending.prompt == state.prompt
                            && state.selection_is_canonical_and_available(&pending.selection);
                        if result.is_ok() && current {
                            state.selected_model = pending.display;
                            state.selected = Some(pending.selection);
                            state.error = None;
                        } else {
                            state.error = Some(
                                "Model selection was not accepted. Complete the current inputs and retry."
                                    .to_owned(),
                            );
                        }
                    }
                }
            }
        }
        // A successful launch triggers a transcript fetch so the pane renders the
        // captured ModelLaneMessage rows (F8), not just a local echo.
        let mut fetch_run: Option<String> = None;
        if let Some(cell) = launch_cell {
            let result = cell.lock().ok().and_then(|mut cell| cell.take());
            if let Some(result) = result {
                self.retain_launch_result(&cell, &result);
                if let Ok(mut state) = self.state.lock() {
                    if !state.pending_launch_request.as_ref().is_some_and(|pending|
                        Arc::ptr_eq(&pending.cell, &cell)) { return; }
                    if let Some(pending) = state.pending_launch_request.take() {
                        state.pending_launch = false;
                        let current_context = pending.workspace_id == state.workspace_id
                            && Some(pending.owner_session_id.as_str())
                                == state.selected_owner_session_id.as_deref()
                            && pending.working_dir == state.folder
                            && pending.prompt == state.prompt;
                        if current_context {
                            match result {
                                Ok(launched) => {
                                    let current_choice = state.local_candidate.as_ref()
                                        .is_some_and(|(selection, _)| selection == &pending.selection)
                                        || state.selected.as_ref() == Some(&pending.selection);
                                    let accepted = self.recovery_scope_is_active() && current_choice
                                        && state.selection_is_canonical_and_available(&pending.selection)
                                        && launched.engine_profile == pending.selection.engine_profile
                                        && !launched.run_id.trim().is_empty()
                                        && !launched.lane_id.trim().is_empty()
                                        && !launched.instance_id.trim().is_empty();
                                    if accepted {
                                        self.forget_launch_receipt(&cell);
                                        state.selected = Some(pending.selection.clone());
                                        state.selected_model = pending.display;
                                        state.local_candidate = None;
                                    }
                                    state.launch_status = Some(if accepted {
                                        format!("launched run {} lane {} (instance {})",
                                            launched.run_id, launched.lane_id, launched.instance_id)
                                    } else {
                                        format!("Backend returned run {} lane {} (instance {})",
                                            launched.run_id, launched.lane_id, launched.instance_id)
                                    });
                                    if accepted {
                                        fetch_run = Some(launched.run_id.clone());
                                        if let Some(client) = &self.client {
                                            client.record_diagnostic_observation(
                                                &launched.run_id, &launched.lane_id,
                                            );
                                        }
                                        state.error = None;
                                    } else {
                                        state.error = Some(
                                            "Launch response did not match the current complete request and profile. Returned run references are shown for recovery; selection remains pending."
                                                .to_owned(),
                                        );
                                    }
                                }
                                Err(_) => {
                                    if pending.selection.lane_kind == "local" {
                                        state.local_candidate = None;
                                        state.selected = None;
                                        state.selected_model.clear();
                                    }
                                    state.error = Some(
                                        "Launch was not accepted; no audited local selection was applied."
                                            .to_owned(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(run_id) = fetch_run {
            if let Some(client) = &self.client {
                if let Ok(mut state) = self.state.lock() {
                    state.pending_transcript = true;
                }
                client.fetch_transcript(&run_id, transcript_cell.clone());
            }
        }
        // Drain fetched transcript rows: append each captured turn so it renders.
        if let Ok(mut slot) = transcript_cell.lock() {
            if let Some(result) = slot.take() {
                if let Ok(mut state) = self.state.lock() {
                    state.pending_transcript = false;
                    match result {
                        Ok(rows) => {
                            for row in rows {
                                state.transcript.push(row);
                            }
                        }
                        Err(_) => {
                            state.error = Some(
                                "Captured transcript could not be loaded from the backend."
                                    .to_owned(),
                            )
                        }
                    }
                }
            }
        }
        for cell in routing_cells {
            let result = cell.lock().ok().and_then(|mut slot| slot.take());
            if let Some(result) = result {
                if let Ok(mut state) = self.state.lock() {
                    let Some(index) = state.pending_routing.iter().position(|pending| Arc::ptr_eq(&pending.cell, &cell)) else { continue; };
                    let pending = state.pending_routing.remove(index);
                    let current = pending.workspace_id == state.workspace_id && self.recovery_scope_is_active();
                    let result = if current { result } else { Err(OperatorChatRoutingFailure::SessionChanged) };
                    match result {
                        Ok(value) => match pending.outcome(value) {
                            Some(execution) => {
                                let identity = (execution.execution_id.clone(), execution.run_id.clone());
                                let conflict = state.routing_observed.keys().any(|(id, run)|
                                    id == &execution.execution_id && run != &execution.run_id);
                                let previous = state.routing_observed.get(&identity);
                                if conflict || previous.is_some_and(|previous|
                                    previous.revision == execution.revision && previous != &execution)
                                {
                                    state.routing_errors.insert(pending.action, "Conflicting execution readback; inspect the canonical execution before retrying.".to_owned());
                                } else if previous.is_some_and(|previous| previous.revision > execution.revision) {
                                    state.routing_errors.insert(pending.action, "Older execution readback ignored; the newer observed revision remains retained for this view.".to_owned());
                                } else {
                                    state.routing_observed.insert(identity, execution.clone());
                                    state.routing_results.retain(|_, previous| previous.execution_id != execution.execution_id);
                                    state.routing_results.insert(pending.action, execution);
                                    state.routing_errors.remove(&pending.action);
                                }
                            }
                            None => {
                                state.routing_errors.insert(pending.action, OperatorChatRoutingFailure::ResponseInvalid.safe_message());
                            }
                        },
                        Err(failure) => {
                            state.routing_errors.insert(pending.action, failure.safe_message());
                        }
                    }
                    // At most four pending requests plus four visible per-action readbacks retain observations.
                    state.prune_routing_observations();
                }
            }
        }
    }

    /// Record the operator's model selection as an auditable decision (F6). Wires
    /// the previously-dead `POST /operator-chat/selection` path.
    fn record_selection(
        &self,
        selection: OperatorChatLaunchSelection,
        working_dir: Option<String>,
        cell: SelectionCell,
    ) {
        if let Some(client) = &self.client {
            client.record_selection(selection, working_dir, cell);
        }
    }

    fn refresh_models(&self) {
        match &self.client {
            Some(client) => {
                let cell = {
                    let Ok(mut state) = self.state.lock() else { return; };
                    if state.pending_models { return; }
                    state.pending_models = true;
                    state.error = None;
                    state.models_cell.clone()
                };
                client.fetch_models(cell);
            }
            None => {
                if let Ok(mut state) = self.state.lock() {
                    state.error = Some("backend not wired (offline pane)".to_string());
                }
            }
        }
    }

    fn launch(&self) {
        let pending = {
            let Ok(mut state) = self.state.lock() else { return; };
            if state.pending_launch { return; }
            let choice = state.local_candidate.clone().or_else(||
                state.selected.clone().map(|selection| (selection, state.selected_model.clone())));
            let Some((selection, display)) = choice else { return; };
            if !state.owner_session_is_canonical_and_available()
                || !state.selection_is_canonical_and_available(&selection)
                || state.folder.trim().is_empty() || state.prompt.trim().is_empty()
            {
                state.error = Some("Choose an available profile and owner, folder and prompt.".to_owned());
                return;
            }
            let Some(owner_session_id) = state.selected_owner_session_id.clone() else { return; };
            let pending = PendingLaunch {
                selection, display, workspace_id: state.workspace_id.clone(),
                owner_session_id, working_dir: state.folder.clone(), prompt: state.prompt.clone(),
                cell: Arc::new(Mutex::new(None)),
            };
            let args = (pending.selection.clone(), pending.owner_session_id.clone(),
                pending.working_dir.clone(), pending.prompt.clone(), pending.cell.clone());
            if !self.recovery_scope_is_active() {
                state.error = Some("The captured launch account is no longer active.".to_owned());
                return;
            }
            let Ok(mut entries) = self.launch_recovery.lock() else { return; };
            if entries.len() >= LAUNCH_RECOVERY_CAPACITY {
                state.error = Some("Launch recovery is full. Review and dismiss completed returned references in their original workspaces before launching again.".to_owned());
                return;
            }
            let Ok(mut next_ui_key) = self.launch_recovery_ui_sequence.lock() else { return; };
            let Some(following_ui_key) = next_ui_key.checked_add(1) else {
                state.error = Some("Launch recovery UI identity is exhausted. No new launch was dispatched.".to_owned());
                return;
            };
            let ui_key = *next_ui_key;
            *next_ui_key = following_ui_key;
            entries.push(LaunchRecoveryEntry {
                ui_key,
                workspace_id: pending.workspace_id.clone(), cell: pending.cell.clone(), returned: None,
            });
            state.pending_launch_request = Some(pending);
            state.pending_launch = true;
            state.error = None;
            state.launch_status = None;
            args
        };
        if let Some(client) = &self.client {
            client.launch(pending.0, &pending.1, &pending.2, &pending.3, pending.4);
        } else if let Ok(mut state) = self.state.lock() {
            state.pending_launch = false;
            state.pending_launch_request = None;
            self.forget_launch_receipt(&pending.4);
            state.error = Some("backend not wired (offline pane)".to_owned());
        }
    }

    fn routing_action(&self, action: OperatorChatRoutingAction) -> bool {
        let Some(client) = self.client.as_ref().filter(|_| self.recovery_scope_is_active()) else {
            if let Ok(mut state) = self.state.lock() {
                state.routing_errors.insert(action, OperatorChatRoutingFailure::AuthenticationRequired.safe_message());
            }
            return false;
        };
        let request = {
            let Ok(mut state) = self.state.lock() else { return false; };
            if state.pending_routing.iter().any(|pending| pending.action == action) { return false; }
            let Some(pending) = PendingRouting::from_request(action,
                state.workspace_id.clone(), state.routing_request_json.clone()) else {
                state.routing_errors.insert(action, OperatorChatRoutingFailure::RequestInvalid.safe_message());
                return false;
            };
            let request = (pending.request_json.clone(), pending.cell.clone());
            state.pending_routing.push(pending);
            state.routing_errors.remove(&action);
            request
        };
        client.routing_action(action, request.0, request.1);
        true
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
        if let Ok(mut state) = self.state.lock() {
            if state.workspace_id != ctx.record.project_id
                || (self.client.is_some() && !self.recovery_scope_is_active())
            {
                *state = OperatorChatUiState {
                    workspace_id: ctx.record.project_id.clone(),
                    ..Default::default()
                };
            }
        }
        self.drain_cells();

        let surface_id = ctx.egui_id.with("operator-chat-surface");
        ui.ctx().accesskit_node_builder(surface_id, |node| {
            node.set_role(accesskit::Role::Group);
            node.set_author_id(SURFACE_AUTHOR_ID.to_owned());
            node.set_label("Operator Chat / Launch".to_owned());
        });

        let mut do_refresh = false;
        let mut do_launch = false;
        let mut routing_action = None;
        let mut select_model: Option<(OperatorChatLaunchSelection, String)> = None;
        let mut select_session: Option<String> = None;
        let mut audit_selection: Option<(OperatorChatLaunchSelection, Option<String>, SelectionCell)> = None;

        let Ok(mut state) = self.state.lock() else {
            ui.label("Operator chat state unavailable");
            return;
        };

        let previous_owner = state.selected_owner_session_id.clone();
        let previous_folder = state.folder.clone();
        let previous_prompt = state.prompt.clone();
        ui.vertical(|ui| {
            ui.heading("Operator Chat / Launch");

            ui.label(format!(
                "Inventory authority: {}",
                if state.inventory.inventory_source == "operator_chat_backend" {
                    "operator_chat_backend"
                } else {
                    "not loaded"
                }
            ));
            ui.label("Governed owner session");
            for row in &state.inventory.sessions {
                let author = session_selection_author_id(&row.session_id);
                let (available, status) = session_row_state(row);
                let label = format!("SESSION  {}  [{}]", row.label, status);
                if available && !state.pending_launch && labelled_button(ui, &author, &label) {
                    // Applied effect: the owner session is selected below (unconditional). Ack so an
                    // out-of-process argus.click on this row resolves Applied.
                    crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &author);
                    select_session = Some(row.session_id.clone());
                } else if !available {
                    labelled_disabled_button(ui, &author, &label);
                }
            }

            if let Some(active) = state.inventory.active_engine_profile.as_ref() {
                ui.label(format!(
                    "Workspace active profile: {} / revision {}",
                    active.profile_id, active.revision,
                ));
            }
            // Model picker: a local row is a candidate until ordinary launch succeeds.
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
                    crate::mcp::argus::acknowledge_action_effect(
                        ui.ctx(),
                        REFRESH_MODELS_AUTHOR_ID,
                    );
                    do_refresh = true;
                }
            });
            if state.pending_models {
                ui.label("Enumerating models...");
            }
            for row in &state.inventory.local {
                let author = row.engine_profile.as_ref().map(profile_selection_author_id)
                    .unwrap_or_else(|| model_selection_author_id("local", None, &row.model_id));
                let row_ready = row.ready && row.engine_profile.is_some();
                let status = if row_ready {
                    "ready"
                } else {
                    "runtime not ready"
                };
                let detail = row.engine_profile.as_ref().map(|snapshot| format!(
                    "{} / revision {} / {:?} / {:?} / {}",
                    snapshot.profile.id, snapshot.profile.revision,
                    snapshot.profile.engine, snapshot.profile.device,
                    snapshot.profile.preset.name,
                )).unwrap_or_else(|| row.runtime_binding.clone());
                let active = row.engine_profile.as_ref().is_some_and(|snapshot|
                    state.inventory.active_engine_profile.as_ref().is_some_and(|active|
                        active.profile_id == snapshot.profile.id
                        && active.revision == snapshot.profile.revision
                        && active.fingerprint == snapshot.fingerprint));
                let label = format!("LOCAL  {}  ({})  [{}]{}",
                    row.display_name, detail, status, if active { " [active]" } else { "" });
                if row_ready && !state.pending_launch && labelled_button(ui, &author, &label) {
                    // Acknowledge the candidate UI effect only. The ordinary launch owns local audit.
                    if state.pending_selection.is_none() {
                        crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &author);
                    }
                    if let Some(profile) = row.engine_profile.clone() {
                        select_model = Some((
                            OperatorChatLaunchSelection::local(profile),
                            detail,
                        ));
                    }
                } else if !row_ready || state.pending_launch {
                    labelled_disabled_button(ui, &author, &label);
                }
            }
            for row in &state.inventory.cloud_byok {
                let author =
                    model_selection_author_id("cloud", Some(&row.provider), &row.model_id);
                let (available, status) = byok_row_state(row);
                let label = format!("CLOUD  {}  [{}]", row.label, status);
                if available && !state.pending_launch && labelled_button(ui, &author, &label) {
                    if state.pending_selection.is_none() {
                        crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &author);
                    }
                    select_model = Some((
                        OperatorChatLaunchSelection::cloud(
                            row.model_id.clone(),
                            row.provider.clone(),
                        ),
                        format!("{} / {}", row.provider, row.model_id),
                    ));
                } else if !available {
                    labelled_disabled_button(ui, &author, &label);
                }
            }
            for row in &state.inventory.cloud_cli_bridge {
                let author = model_selection_author_id("cli", Some(&row.provider), &row.model_id);
                let (available, status) = cli_row_state(row);
                let label = format!("CLI  {}  [{}]", row.label, status);
                if available && !state.pending_launch && labelled_button(ui, &author, &label) {
                    if state.pending_selection.is_none() {
                        crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &author);
                    }
                    select_model = Some((
                        OperatorChatLaunchSelection::cli(
                            row.model_id.clone(),
                            row.provider.clone(),
                        ),
                        format!("{} / {}", row.provider, row.model_id),
                    ));
                } else if !available {
                    labelled_disabled_button(ui, &author, &label);
                }
            }
            for row in &state.inventory.subagents {
                let author = model_selection_author_id("subagent", Some(&row.role), &row.model_id);
                let (available, status) = subagent_row_state(row);
                let label = format!("SUBAGENT  {}  [{}]", row.label, status);
                if available && !state.pending_launch && labelled_button(ui, &author, &label) {
                    if state.pending_selection.is_none() {
                        crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &author);
                    }
                    select_model = Some((
                        OperatorChatLaunchSelection::subagent(row.model_id.clone()),
                        format!("{} / {}", row.role, row.model_id),
                    ));
                } else if !available {
                    labelled_disabled_button(ui, &author, &label);
                }
            }

            // Keep the complete ordinary request immutable while awaiting its result.
            ui.add_enabled_ui(!state.pending_launch, |ui| {
            // Focus-safe folder/worktree injection; this is an ordinary text
            // field and never invokes a native foreground picker.
            ui.horizontal(|ui| {
                ui.label("Folder / worktree");
                labelled_text_edit(
                    ui,
                    ctx.egui_id,
                    WORKTREE_SELECTION_AUTHOR_ID,
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

            });
            // Launch.
            let launch_ready = state.owner_session_is_canonical_and_available()
                && (state.local_candidate.as_ref().is_some_and(|(selection, _)|
                        state.selection_is_canonical_and_available(selection))
                    || state.selected_model_is_canonical_and_available())
                && state.pending_selection.is_none()
                && !state.folder.trim().is_empty()
                && !state.prompt.trim().is_empty()
                && !state.pending_launch;
            ui.add_enabled_ui(launch_ready, |ui| {
                if labelled_button(ui, LAUNCH_AUTHOR_ID, "Launch session") {
                    crate::mcp::argus::acknowledge_action_effect(ui.ctx(), LAUNCH_AUTHOR_ID);
                    do_launch = true;
                }
            });
            if state.local_candidate.is_some() && !state.pending_launch {
                ui.label("Pending profile. Complete owner, folder and prompt, then launch.");
            }
            if state.pending_launch {
                ui.label("Launching and recording the selection audit...");
            }
            if state.pending_selection.is_some() {
                ui.label("Recording model-selection audit before accepting the selection...");
            }
            if self.recovery_scope_is_active() {
                if let Ok(mut entries) = self.launch_recovery.lock() {
                    entries.retain(|entry| {
                        if entry.workspace_id != state.workspace_id { return true; }
                        let Some(returned) = entry.returned.as_ref() else { return true; };
                        ui.label(format!("Unaccepted launch response: run {} lane {} (instance {}). Selection was not activated; execution was not cancelled.",
                            returned.run_id, returned.lane_id, returned.instance_id));
                        let author = format!("operator-chat.launch-recovery.dismiss.{}", entry.ui_key);
                        !labelled_button(ui, &author, "Dismiss returned launch references")
                    });
                }
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

            ui.separator();
            ui.label("Persisted routing lifecycle request (JSON)");
            labelled_text_edit(
                ui,
                ctx.egui_id,
                ROUTING_REQUEST_AUTHOR_ID,
                "Canonical routing lifecycle request",
                &mut state.routing_request_json,
                true,
                480.0,
            );
            let scope_active = self.recovery_scope_is_active();
            ui.horizontal(|ui| {
                for (action, author) in [
                    (OperatorChatRoutingAction::Lifecycle, ROUTING_LIFECYCLE_AUTHOR_ID),
                    (OperatorChatRoutingAction::Recover, ROUTING_RECOVER_AUTHOR_ID),
                    (OperatorChatRoutingAction::Cancel, ROUTING_CANCEL_AUTHOR_ID),
                    (OperatorChatRoutingAction::Authority, ROUTING_AUTHORITY_AUTHOR_ID),
                ] {
                    if scope_active && !state.pending_routing.iter().any(|pending| pending.action == action) {
                        if labelled_button(ui, author, action.label()) {
                            routing_action = Some((action, author));
                        }
                    } else {
                        labelled_disabled_button(ui, author, action.label());
                    }
                }
            });
            let mut observations = Vec::new();
            if !scope_active {
                observations.push("Routing unavailable: active authenticated backend required.".to_owned());
            }
            for pending in &state.pending_routing {
                observations.push(format!("{} pending for execution {}; outcome unconfirmed.",
                    pending.action.label(), pending.execution_id));
            }
            for (action, execution) in &state.routing_results {
                observations.push(format!(
                    "{} last readback: {} for execution {} / run {}, revision {}. EventLedger {} (sequence {}).",
                    action.label(), execution.status.label(), execution.execution_id, execution.run_id,
                    execution.revision, execution.event_ledger_event_id, execution.event_ledger_seq));
            }
            for (action, error) in &state.routing_errors {
                observations.push(format!("{}: {error}", action.label()));
            }
            if !observations.is_empty() {
                observations.push("Execution readback does not establish process shutdown or drain.".to_owned());
                let status = observations.join("\n");
                let response = ui.label(&status);
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_author_id(ROUTING_STATUS_AUTHOR_ID.to_owned());
                    node.set_label(status);
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

        if let Some(session_id) = select_session {
            state.selected_owner_session_id = Some(session_id);
        }
        if previous_owner != state.selected_owner_session_id
            || previous_folder != state.folder || previous_prompt != state.prompt
        {
            state.pending_selection = None;
        }
        if let Some((selection, display)) = select_model {
            if !state.pending_launch && selection.lane_kind == "local" {
                state.pending_selection = None;
                state.selected = None;
                state.selected_model = format!("Pending profile: {display}");
                state.local_candidate = Some((selection, display));
                state.error = None;
            } else if !state.pending_launch && state.pending_selection.is_none() {
                state.local_candidate = None;
                let cell = Arc::new(Mutex::new(None));
                let working_dir = (!state.folder.trim().is_empty()).then(|| state.folder.clone());
                audit_selection = Some((selection.clone(), working_dir, cell.clone()));
                state.pending_selection = Some(PendingSelection {
                    selection, display, workspace_id: state.workspace_id.clone(),
                    owner_session_id: state.selected_owner_session_id.clone(),
                    working_dir: state.folder.clone(), prompt: state.prompt.clone(), cell,
                });
                state.error = None;
            }
        }
        drop(state);

        if let Some((selection, working_dir, cell)) = audit_selection {
            self.record_selection(selection, working_dir, cell);
        }
        if do_refresh {
            self.refresh_models();
        }
        if do_launch {
            self.launch();
        }
        if let Some((action, author)) = routing_action {
            if self.routing_action(action) {
                crate::mcp::argus::acknowledge_action_effect_allow_target_transition(ui.ctx(), author);
            }
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Pane
    }
}

#[cfg(test)]
mod client_safety_tests {
    use super::*;

    #[test]
    fn inventory_refresh_failure_clears_stale_launch_identity() {
        let mut state = OperatorChatUiState {
            inventory: OperatorChatModelInventory {
                inventory_source: "operator_chat_backend".to_owned(),
                ..Default::default()
            },
            selected_model: "gpt-5-codex".to_owned(),
            selected: Some(OperatorChatLaunchSelection::cli("gpt-5-codex", "codex")),
            selected_owner_session_id: Some("owner".to_owned()),
            pending_selection: Some(PendingSelection {
                selection: OperatorChatLaunchSelection::cli("gpt-5-codex", "codex"),
                display: "GPT/Codex subscription".to_owned(),
                workspace_id: String::new(), owner_session_id: None,
                working_dir: String::new(), prompt: String::new(),
                cell: Arc::new(Mutex::new(None)),
            }),
            ..Default::default()
        };

        state.fail_inventory_refresh();

        assert!(state.inventory.inventory_source.is_empty());
        assert!(state.selected.is_none());
        assert!(state.selected_model.is_empty());
        assert!(state.selected_owner_session_id.is_none());
        assert!(state.pending_selection.is_none());
        assert!(!state.owner_session_is_canonical_and_available());
        assert!(!state.selected_model_is_canonical_and_available());
    }

    #[test]
    fn successful_refresh_clears_a_pending_selection_that_became_unavailable() {
        let mut state = OperatorChatUiState {
            pending_selection: Some(PendingSelection {
                selection: OperatorChatLaunchSelection::cli("gpt-5-codex", "codex"),
                display: "GPT/Codex subscription".to_owned(),
                workspace_id: String::new(), owner_session_id: None,
                working_dir: String::new(), prompt: String::new(),
                cell: Arc::new(Mutex::new(None)),
            }),
            ..Default::default()
        };
        let inventory = OperatorChatModelInventory {
            inventory_source: "operator_chat_backend".to_owned(),
            cloud_cli_bridge: vec![OperatorChatCloudRow {
                provider: "codex".to_owned(),
                model_id: "gpt-5-codex".to_owned(),
                label: "GPT/Codex subscription".to_owned(),
                status: "unavailable".to_owned(),
            }],
            ..Default::default()
        };

        state.reconcile_with_refreshed_inventory(inventory);

        assert!(state.pending_selection.is_none());
        assert!(state
            .error
            .as_deref()
            .is_some_and(|error| error.contains("pending model selection")));
    }

    #[test]
    fn disabled_reasons_do_not_echo_untrusted_status_values() {
        let canary = "canary-secret-status";
        let byok = OperatorChatCloudRow {
            status: canary.to_owned(),
            ..Default::default()
        };
        let cli = OperatorChatCloudRow {
            status: canary.to_owned(),
            ..Default::default()
        };
        let session = OperatorChatSessionRow {
            status: canary.to_owned(),
            ..Default::default()
        };
        let subagent = OperatorChatSubagentRow {
            status: canary.to_owned(),
            ..Default::default()
        };

        for (available, reason) in [
            byok_row_state(&byok),
            cli_row_state(&cli),
            session_row_state(&session),
            subagent_row_state(&subagent),
        ] {
            assert!(!available);
            assert!(!reason.contains(canary));
        }
    }
}
